//! Aprobaciones (autorización: RLS):
//!   GET   /aprobaciones
//!   PATCH /aprobaciones/{id}            body: {"fecha_fin": "YYYY-MM-DD" | null, "glosa"?: "..."}
//!   GET   /aprobaciones/{id}/historial  (requerimiento_historial completo)

use axum::{
    Json,
    extract::{Path, State},
};
use chrono::NaiveDate;
use db::{PgPool, aprobaciones::ActualizarError};
use domain::{Aprobacion, EntradaHistorial};
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    dto::RefDto,
    error::{ApiError, ApiResult, tx_usuario},
};

fn rfc3339(f: chrono::DateTime<chrono::Utc>) -> String {
    f.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

#[derive(Debug, Serialize)]
pub struct ModificacionDto {
    pub usuario_id: Option<Uuid>,
    pub fecha: String,
}

#[derive(Debug, Serialize)]
pub struct AprobacionDto {
    pub id: Uuid,
    pub numero: i64,
    pub folio: Option<String>,
    pub tipo: Option<String>,
    pub estado: Option<String>,
    pub servicio: Option<RefDto>,
    pub cargo: RefDto,
    /// `requerimientos.cantidad_personas`.
    pub cantidad: Option<u32>,
    pub fecha_inicio: NaiveDate,
    pub fecha_fin_tentativa: Option<NaiveDate>,
    pub fecha_fin: Option<NaiveDate>,
    pub ultima_modificacion: Option<ModificacionDto>,
}

impl From<Aprobacion> for AprobacionDto {
    fn from(a: Aprobacion) -> Self {
        Self {
            id: a.id.into(),
            numero: a.numero,
            folio: a.folio,
            tipo: a.tipo,
            estado: a.estado,
            servicio: a.servicio_id.map(|id| RefDto::new(id, a.servicio_nombre)),
            cargo: RefDto::new(a.cargo_id, a.cargo_nombre),
            cantidad: a.cantidad,
            fecha_inicio: a.fecha_inicio,
            fecha_fin_tentativa: a.fecha_fin_tentativa,
            fecha_fin: a.fecha_fin,
            ultima_modificacion: a.ultima_modificacion.map(|m| ModificacionDto {
                usuario_id: m.usuario_id.map(Into::into),
                fecha: rfc3339(m.fecha),
            }),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct HistorialDto {
    pub id: String,
    pub campo: String,
    pub valor_anterior: Option<String>,
    pub valor_nuevo: Option<String>,
    pub usuario_id: Option<Uuid>,
    pub created_at: String,
    pub glosa: Option<String>,
    pub grupo_edicion: Option<String>,
    pub categoria_desvio: Option<String>,
    pub detalle_desvio: Option<String>,
}

impl From<EntradaHistorial> for HistorialDto {
    fn from(h: EntradaHistorial) -> Self {
        Self {
            id: h.id,
            campo: h.campo,
            valor_anterior: h.valor_anterior,
            valor_nuevo: h.valor_nuevo,
            usuario_id: h.usuario_id.map(Into::into),
            created_at: rfc3339(h.created_at),
            glosa: h.glosa,
            grupo_edicion: h.grupo_edicion,
            categoria_desvio: h.categoria_desvio,
            detalle_desvio: h.detalle_desvio,
        }
    }
}

/// `fecha_fin` debe venir explícitamente: ausente → 400; `null` → quitar la fecha.
/// TODO(negocio): uso de `grupo_edicion`, `categoria_desvio`, `detalle_desvio` del historial.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatchAprobacion {
    #[serde(default, deserialize_with = "presente")]
    pub fecha_fin: Option<Option<NaiveDate>>,
    pub glosa: Option<String>,
}

/// Distingue "campo ausente" (None) de "campo = null" (Some(None)).
fn presente<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

pub async fn listar(
    State(pool): State<PgPool>,
    user: AuthUser,
) -> ApiResult<Json<Vec<AprobacionDto>>> {
    let mut tx = tx_usuario(&pool, &user).await?;
    let v = db::aprobaciones::listar(&mut tx).await?;
    Ok(Json(v.into_iter().map(AprobacionDto::from).collect()))
}

pub async fn historial(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<HistorialDto>>> {
    let mut tx = tx_usuario(&pool, &user).await?;
    let h = db::aprobaciones::historial(&mut tx, id.into())
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(h.into_iter().map(HistorialDto::from).collect()))
}

pub async fn editar_fecha_fin(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<PatchAprobacion>,
) -> ApiResult<Json<AprobacionDto>> {
    let nueva = body.fecha_fin.ok_or_else(|| {
        ApiError::BadRequest("falta el campo `fecha_fin` (usar null para quitarla)".into())
    })?;
    let glosa = body
        .glosa
        .as_deref()
        .map(str::trim)
        .filter(|g| !g.is_empty());

    let mut tx = tx_usuario(&pool, &user).await?;
    let a = db::aprobaciones::actualizar_fecha_fin(
        &mut tx,
        id.into(),
        nueva,
        user.user_id.into(),
        glosa,
    )
    .await
    .map_err(|e| match e {
        ActualizarError::NoEncontrado => ApiError::NotFound,
        ActualizarError::SinPermiso => ApiError::Forbidden,
        ActualizarError::Validacion(e) => ApiError::Unprocessable(e.to_string()),
        ActualizarError::Db(e) => e.into(),
    })?;
    tx.commit().await?;

    tracing::info!(requerimiento_id = %id, usuario_id = %user.user_id, fecha_fin = ?nueva, "fecha_fin editada");
    Ok(Json(AprobacionDto::from(a)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Result<PatchAprobacion, serde_json::Error> {
        serde_json::from_str(s)
    }

    #[test]
    fn body_distingue_ausente_null_y_fecha() {
        assert_eq!(parse("{}").unwrap().fecha_fin, None);
        assert_eq!(
            parse(r#"{"fecha_fin":null}"#).unwrap().fecha_fin,
            Some(None)
        );
        let p = parse(r#"{"fecha_fin":"2026-12-31","glosa":"extensión"}"#).unwrap();
        assert_eq!(p.fecha_fin, Some(NaiveDate::from_ymd_opt(2026, 12, 31)));
        assert_eq!(p.glosa.as_deref(), Some("extensión"));
        assert!(parse(r#"{"fecha_fin":"31/12/2026"}"#).is_err());
        assert!(parse(r#"{"fecha_fin":null,"cantidad":9}"#).is_err());
    }
}
