//! GET /requerimientos[?mios=true] — "Mis requerimientos".
//!
//! Devuelve lo que RLS permite ver (RR.HH.: todos). Con `mios=true`, solo los que pidió el
//! usuario. Cada fila indica `es_mio` y su `estado`, para que el solicitante vea si fue
//! aprobado o no.

use axum::{
    Json,
    extract::{Query, State},
};
use chrono::NaiveDate;
use db::PgPool;
use domain::RequerimientoResumen;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    dto::RefDto,
    error::{ApiResult, tx_usuario},
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
