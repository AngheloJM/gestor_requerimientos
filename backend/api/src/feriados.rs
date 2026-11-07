//! Feriados y plazos de un requerimiento nuevo (revisión del 06/11/2026, puntos 10 a 12).
//!
//!   GET    /feriados?anio=2026                         feriados del año (todos los autenticados)
//!   POST   /feriados {fecha, nombre}                   → 201 (solo RR.HH.: RLS)
//!   DELETE /feriados/{id}                              → 204 (solo RR.HH.: RLS)
//!   GET    /requerimientos/plazos?operacion=&servicio_id=&cargo_id=
//!          fechas máximas de capacitación (normal y ajustada) y días hábiles de reclutamiento.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{Datelike, NaiveDate};
use db::{PgPool, feriados::Feriado};
use domain::plazos::{self, OrigenNormal, TiemposCapacitacion};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::{ApiError, ApiResult, tx_usuario},
    tablero::hoy_negocio,
};

#[derive(Debug, Deserialize)]
pub struct FiltroAnio {
    pub anio: Option<i32>,
}

pub async fn listar(
    State(pool): State<PgPool>,
    user: AuthUser,
    Query(q): Query<FiltroAnio>,
) -> ApiResult<Json<Vec<Feriado>>> {
    let anio = q.anio.unwrap_or_else(|| hoy_negocio().year());
    if !(2000..=2100).contains(&anio) {
        return Err(ApiError::Unprocessable("año fuera de rango".into()));
    }
    let mut tx = tx_usuario(&pool, &user).await?;
    Ok(Json(db::feriados::del_anio(&mut tx, anio).await?))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NuevoFeriado {
    pub fecha: NaiveDate,
    pub nombre: String,
}

pub async fn crear(
    State(pool): State<PgPool>,
    user: AuthUser,
    Json(b): Json<NuevoFeriado>,
) -> ApiResult<StatusCode> {
    let nombre = b.nombre.trim();
    if !(2..=120).contains(&nombre.chars().count()) {
        return Err(ApiError::Unprocessable(
            "el nombre del feriado debe tener entre 2 y 120 caracteres".into(),
        ));
    }
    let mut tx = tx_usuario(&pool, &user).await?;
    db::feriados::crear(&mut tx, b.fecha, nombre)
        .await
        .map_err(|e| {
            if e.es_duplicado() {
                ApiError::Conflict("ya hay un feriado cargado en esa fecha".into())
            } else {
                e.into()
            }
        })?;
    tx.commit().await?;
    tracing::info!(usuario_id = %user.user_id, fecha = %b.fecha, "feriado registrado");
    Ok(StatusCode::CREATED)
}

pub async fn eliminar(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let mut tx = tx_usuario(&pool, &user).await?;
    if db::feriados::eliminar(&mut tx, id).await? == 0 {
        return Err(ApiError::NotFound);
    }
    tx.commit().await?;
    tracing::info!(usuario_id = %user.user_id, feriado_id = %id, "feriado eliminado");
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct ConsultaPlazos {
    pub operacion: NaiveDate,
    pub servicio_id: Option<Uuid>,
    pub cargo_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct PlazosDto {
    pub hoy: NaiveDate,
    pub capacitacion_normal: Option<NaiveDate>,
    /// "cargo" | "servicio" (respaldo con el tiempo general del servicio).
    pub origen_normal: Option<&'static str>,
    pub capacitacion_ajustada: Option<NaiveDate>,
    pub entrega_capacitacion: Option<NaiveDate>,
    pub dias_reclutamiento: Option<u32>,
}

pub async fn plazos(
    State(pool): State<PgPool>,
    user: AuthUser,
    Query(q): Query<ConsultaPlazos>,
) -> ApiResult<Json<PlazosDto>> {
    let hoy = hoy_negocio();
    let mut tx = tx_usuario(&pool, &user).await?;
    let tiempos = match (q.servicio_id, q.cargo_id) {
        (Some(s), Some(c)) => db::feriados::tiempos_capacitacion(&mut tx, s, c).await?,
        _ => TiemposCapacitacion::default(),
    };
    // Primero sin feriados, para saber hasta qué fecha buscarlos.
    let previo = plazos::calcular(q.operacion, tiempos, hoy, &Default::default());
    let feriados = match previo.entrega_capacitacion {
        Some(e) if e > hoy => db::feriados::entre(&mut tx, hoy, e).await?,
        _ => Default::default(),
    };
    let p = plazos::calcular(q.operacion, tiempos, hoy, &feriados);
    Ok(Json(PlazosDto {
        hoy,
        capacitacion_normal: p.capacitacion_normal,
        origen_normal: p.origen_normal.map(|o| match o {
            OrigenNormal::Cargo => "cargo",
            OrigenNormal::Servicio => "servicio",
        }),
        capacitacion_ajustada: p.capacitacion_ajustada,
        entrega_capacitacion: p.entrega_capacitacion,
        dias_reclutamiento: p.dias_reclutamiento,
    }))
}
