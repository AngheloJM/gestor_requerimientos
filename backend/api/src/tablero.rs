//! GET /tablero — (1) indicadores por completitud y (2) Kanban de `solicitudes_cambio`.

use axum::{Json, extract::State};
use chrono::{FixedOffset, NaiveDate, Utc};
use db::PgPool;
use domain::{Completitud, Kanban, SolicitudCambio, tablero::CRITERIOS_SIN_DEFINIR};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    dto::RefDto,
    error::{ApiResult, tx_usuario},
};

/// Zona horaria del negocio (Bolivia, UTC-4) para decidir qué día es "hoy" (vigencia).
const UTC_OFFSET_NEGOCIO_SEGUNDOS: i32 = -4 * 3600;

pub(crate) fn hoy_negocio() -> NaiveDate {
    let tz = FixedOffset::east_opt(UTC_OFFSET_NEGOCIO_SEGUNDOS).expect("offset válido");
    Utc::now().with_timezone(&tz).date_naive()
}

fn rfc3339(f: chrono::DateTime<Utc>) -> String {
    f.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

#[derive(Debug, Serialize)]
pub struct TableroDto {
    pub indicadores: IndicadoresDto,
    /// Criterios de completitud aún no evaluados (pendientes de definición de negocio).
    pub criterios_sin_definir: &'static [&'static str],
    pub servicios_incompletos: Vec<IncompletoDto>,
    pub kanban: KanbanDto,
}

#[derive(Debug, Serialize)]
pub struct IndicadoresDto {
    pub servicios_total: u32,
    pub servicios_configurados: u32,
    pub servicios_incompletos: u32,
    pub areas_sin_completar: u32,
}

#[derive(Debug, Serialize)]
pub struct TareaDto {
    pub codigo: &'static str,
    pub area: &'static str,
}

#[derive(Debug, Serialize)]
pub struct IncompletoDto {
    pub servicio: RefDto,
    pub cliente: Option<String>,
    pub tareas: Vec<TareaDto>,
}

#[derive(Debug, Serialize)]
pub struct KanbanDto {
    pub comercial: Vec<TarjetaDto>,
    pub operaciones: Vec<TarjetaDto>,
    pub rrhh: Vec<TarjetaDto>,
}

#[derive(Debug, Serialize)]
pub struct TarjetaDto {
    pub id: Uuid,
    pub servicio: RefDto,
    pub cargo: Option<RefDto>,
    pub campo: String,
    pub solicitante_id: Option<Uuid>,
    pub motivo: Option<String>,
    pub descripcion: Option<String>,
    pub estado: Option<String>,
    pub responsable_id: Option<Uuid>,
    pub comentario_resolucion: Option<String>,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub cerrada_at: Option<String>,
}

impl From<SolicitudCambio> for TarjetaDto {
    fn from(s: SolicitudCambio) -> Self {
        Self {
            id: s.id.into(),
            servicio: RefDto::new(s.servicio_id, s.servicio_nombre),
            cargo: s.cargo_id.map(|id| RefDto::new(id, s.cargo_nombre)),
            campo: s.campo,
            solicitante_id: s.solicitante_id.map(Into::into),
            motivo: s.motivo,
            descripcion: s.descripcion,
            estado: s.estado,
            responsable_id: s.responsable_id.map(Into::into),
            comentario_resolucion: s.comentario_resolucion,
            created_at: rfc3339(s.created_at),
            updated_at: s.updated_at.map(rfc3339),
            cerrada_at: s.cerrada_at.map(rfc3339),
        }
    }
}

fn armar(c: Completitud, k: Kanban) -> TableroDto {
    let conv = |v: Vec<SolicitudCambio>| v.into_iter().map(TarjetaDto::from).collect();
    TableroDto {
        indicadores: IndicadoresDto {
            servicios_total: c.indicadores.servicios_total,
            servicios_configurados: c.indicadores.servicios_configurados,
            servicios_incompletos: c.indicadores.servicios_incompletos,
            areas_sin_completar: c.indicadores.areas_sin_completar,
        },
        criterios_sin_definir: CRITERIOS_SIN_DEFINIR,
        servicios_incompletos: c
            .incompletos
            .into_iter()
            .map(|i| IncompletoDto {
                servicio: RefDto::new(i.servicio_id, Some(i.servicio_nombre)),
                cliente: i.cliente_nombre,
                tareas: i
                    .tareas
                    .iter()
                    .map(|t| TareaDto {
                        codigo: t.codigo(),
                        area: t.area().as_str(),
                    })
                    .collect(),
            })
            .collect(),
        kanban: KanbanDto {
            comercial: conv(k.comercial),
            operaciones: conv(k.operaciones),
            rrhh: conv(k.rrhh),
        },
    }
}

pub async fn get_tablero(
    State(pool): State<PgPool>,
    user: AuthUser,
) -> ApiResult<Json<TableroDto>> {
    let mut tx = tx_usuario(&pool, &user).await?;
    let estados = db::tablero::estado_servicios(&mut tx).await?;
    let solicitudes = db::tablero::solicitudes_cambio(&mut tx).await?;
    Ok(Json(armar(
        Completitud::construir(&estados, hoy_negocio()),
        Kanban::construir(solicitudes),
    )))
}
