//! GET  /requerimientos[?mios=true] — "Mis requerimientos".
//! GET  /requerimientos/opciones   — servicios/cargos elegibles y catálogos del formulario.
//! POST /requerimientos            — crea vía `crear_requerimiento_con_snapshot` → 201 {folio}.
//!
//! Devuelve lo que RLS permite ver (RR.HH.: todos). Con `mios=true`, solo los que pidió el
//! usuario. Cada fila indica `es_mio` y su `estado`, para que el solicitante vea si fue
//! aprobado o no.

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use chrono::NaiveDate;
use db::PgPool;
use domain::{
    BonoExtra, CargoOpcion, Decimal, NuevoRequerimiento, RequerimientoResumen,
    requerimiento::{MOTIVOS, TIPOS},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    dto::RefDto,
    error::{ApiError, ApiResult, tx_usuario},
};

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FiltroRequerimientos {
    #[serde(default)]
    pub mios: bool,
}

#[derive(Debug, Serialize)]
pub struct RequerimientoDto {
    pub id: Uuid,
    pub numero: i64,
    pub folio: Option<String>,
    pub es_mio: bool,
    pub servicio: Option<RefDto>,
    pub cargo: RefDto,
    pub tipo: Option<String>,
    pub motivo: Option<String>,
    pub cantidad: Option<u32>,
    /// pendiente | en_proceso | aprobado | rechazado | cerrado (valores de la base).
    pub estado: Option<String>,
    pub fecha_inicio: NaiveDate,
    pub fecha_inicio_operacion: Option<NaiveDate>,
    pub creado: String,
}

impl RequerimientoDto {
    fn new(r: RequerimientoResumen, yo: Uuid) -> Self {
        Self {
            id: r.id.into(),
            numero: r.numero,
            folio: r.folio,
            es_mio: r.solicitante_id.map(Uuid::from) == Some(yo),
            servicio: r.servicio_id.map(|id| RefDto::new(id, r.servicio_nombre)),
            cargo: RefDto::new(r.cargo_id, r.cargo_nombre),
            tipo: r.tipo,
            motivo: r.motivo,
            cantidad: r.cantidad,
            estado: r.estado,
            fecha_inicio: r.fecha_inicio,
            fecha_inicio_operacion: r.fecha_inicio_operacion,
            creado: r
                .created_at
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        }
    }
}

pub async fn listar(
    State(pool): State<PgPool>,
    user: AuthUser,
    Query(f): Query<FiltroRequerimientos>,
) -> ApiResult<Json<Vec<RequerimientoDto>>> {
    let mut tx = tx_usuario(&pool, &user).await?;
    let solicitante = f.mios.then_some(user.user_id.into());
    let v = db::requerimientos::listar(&mut tx, solicitante).await?;
    let yo = user.user_id;
    Ok(Json(
        v.into_iter()
            .map(|r| RequerimientoDto::new(r, yo))
            .collect(),
    ))
}

// ---------------------------------------------------------------------------------------
// GET /requerimientos/opciones · POST /requerimientos
// ---------------------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct CargoOpcionDto {
    pub id: Uuid,
    pub nombre: String,
    pub tipo: Option<String>,
    /// Referencia informativa (requerimientos de crecimiento); no se autocompleta nada.
    pub cantidad_mujeres: Option<i32>,
    pub cantidad_hombres: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct ServicioOpcionDto {
    pub id: Uuid,
    pub nombre: String,
    pub tiempo_capacitacion_dias: Option<i32>,
    pub cargos: Vec<CargoOpcionDto>,
}

#[derive(Debug, Serialize)]
pub struct OpcionesDto {
    pub servicios: Vec<ServicioOpcionDto>,
    pub cargos_staff: Vec<CargoOpcionDto>,
    /// Catálogos de la base (CHECK / validaciones verificadas en producción).
    pub tipos: &'static [&'static str],
    pub motivos: &'static [&'static str],
}

fn cargo_dto(c: CargoOpcion) -> CargoOpcionDto {
    CargoOpcionDto {
        id: c.id.into(),
        nombre: c.nombre,
        tipo: c.tipo,
        cantidad_mujeres: c.cantidad_mujeres,
        cantidad_hombres: c.cantidad_hombres,
    }
}

pub async fn opciones(State(pool): State<PgPool>, user: AuthUser) -> ApiResult<Json<OpcionesDto>> {
    let mut tx = tx_usuario(&pool, &user).await?;
    let o = db::requerimientos::opciones(&mut tx).await?;
    Ok(Json(OpcionesDto {
        servicios: o
            .servicios
            .into_iter()
            .map(|s| ServicioOpcionDto {
                id: s.id.into(),
                nombre: s.nombre,
                tiempo_capacitacion_dias: s.tiempo_capacitacion_dias,
                cargos: s.cargos.into_iter().map(cargo_dto).collect(),
            })
            .collect(),
        cargos_staff: o.cargos_staff.into_iter().map(cargo_dto).collect(),
        tipos: &TIPOS,
        motivos: &MOTIVOS,
    }))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BonoExtraBody {
    pub nombre: String,
    pub monto: Decimal,
    pub motivo: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrearRequerimientoBody {
    pub tipo: String,
    pub servicio_id: Option<Uuid>,
    pub cargo_id: Uuid,
    pub cantidad: u32,
    pub motivo: String,
    pub fecha_inicio_capacitacion: NaiveDate,
    pub fecha_inicio_operacion: NaiveDate,
    pub fecha_fin: Option<NaiveDate>,
    pub comentarios: Option<String>,
    #[serde(default)]
    pub bonos_extra: Vec<BonoExtraBody>,
}

#[derive(Debug, Serialize)]
pub struct CreadoDto {
    pub folio: String,
}

/// Límite defensivo (el body ya está acotado a 64 KB).
const MAX_BONOS_EXTRA: usize = 20;

pub async fn crear(
    State(pool): State<PgPool>,
    user: AuthUser,
    Json(b): Json<CrearRequerimientoBody>,
) -> ApiResult<(StatusCode, Json<CreadoDto>)> {
    if b.bonos_extra.len() > MAX_BONOS_EXTRA {
        return Err(ApiError::Unprocessable(format!(
            "como máximo {MAX_BONOS_EXTRA} bonos extra"
        )));
    }
    let nuevo = NuevoRequerimiento {
        tipo: b.tipo,
        servicio_id: b.servicio_id.map(Into::into),
        cargo_id: b.cargo_id.into(),
        cantidad: b.cantidad,
        motivo: b.motivo,
        fecha_inicio_capacitacion: b.fecha_inicio_capacitacion,
        fecha_inicio_operacion: b.fecha_inicio_operacion,
        fecha_fin: b.fecha_fin,
        comentarios: b.comentarios,
        bonos_extra: b
            .bonos_extra
            .into_iter()
            .map(|x| BonoExtra {
                nombre: x.nombre,
                monto: x.monto,
                motivo: x.motivo,
            })
            .collect(),
    };
    let mut tx = tx_usuario(&pool, &user).await?;
    let folio = db::requerimientos::crear(&mut tx, &nuevo).await?;
    tx.commit().await?;
    tracing::info!(usuario_id = %user.user_id, %folio, "requerimiento creado");
    Ok((StatusCode::CREATED, Json(CreadoDto { folio })))
}
