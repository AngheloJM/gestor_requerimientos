//! GET /almanaque?mes=&anio= — Entradas / Salidas / Entregas / Rectificaciones por día.

use axum::{
    Json,
    extract::{Query, State},
};
use db::PgPool;
use domain::{Almanaque, DiaAlmanaque, LineaAlmanaque, Periodo};
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthUser,
    dto::RefDto,
    error::{ApiError, ApiResult, tx_usuario},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlmanaqueQuery {
    pub mes: u32,
    pub anio: i32,
}

#[derive(Debug, Serialize)]
pub struct AlmanaqueDto {
    pub anio: i32,
    pub mes: u32,
    pub dias: Vec<DiaDto>,
}

#[derive(Debug, Serialize)]
pub struct DiaDto {
    pub fecha: String,
    pub entradas: Vec<LineaDto>,
    pub salidas: Vec<LineaDto>,
    pub entregas: Vec<LineaDto>,
    pub rectificaciones: Vec<LineaDto>,
}

#[derive(Debug, Serialize)]
pub struct LineaDto {
    /// `null` si el requerimiento no tiene servicio.
    pub servicio: Option<RefDto>,
    pub cargo: RefDto,
    pub cantidad: u64,
    /// Registros con cantidad NULL (no sumados).
    pub registros_sin_cantidad: u32,
}

impl From<LineaAlmanaque> for LineaDto {
    fn from(l: LineaAlmanaque) -> Self {
        Self {
            servicio: l.servicio_id.map(|id| RefDto::new(id, l.servicio_nombre)),
            cargo: RefDto::new(l.cargo_id, l.cargo_nombre),
            cantidad: l.cantidad,
            registros_sin_cantidad: l.registros_sin_cantidad,
        }
    }
}

impl From<DiaAlmanaque> for DiaDto {
    fn from(d: DiaAlmanaque) -> Self {
        let conv = |v: Vec<LineaAlmanaque>| v.into_iter().map(LineaDto::from).collect();
        Self {
            fecha: d.fecha.format("%Y-%m-%d").to_string(),
            entradas: conv(d.entradas),
            salidas: conv(d.salidas),
            entregas: conv(d.entregas),
            rectificaciones: conv(d.rectificaciones),
        }
    }
}

pub async fn get_almanaque(
    State(pool): State<PgPool>,
    user: AuthUser,
    Query(q): Query<AlmanaqueQuery>,
) -> ApiResult<Json<AlmanaqueDto>> {
    let periodo = Periodo::new(q.anio, q.mes).map_err(|e| ApiError::BadRequest(e.to_string()))?;
    let mut tx = tx_usuario(&pool, &user).await?;
    let movs = db::almanaque::listar_movimientos(&mut tx, periodo).await?;
    let a = Almanaque::construir(periodo, &movs);
    Ok(Json(AlmanaqueDto {
        anio: periodo.anio(),
        mes: periodo.mes(),
        dias: a.dias.into_iter().map(DiaDto::from).collect(),
    }))
}
