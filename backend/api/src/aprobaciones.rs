//! Aprobaciones (autorización: RLS):
//!   GET   /aprobaciones
//!   PATCH /aprobaciones/{id}            body: {"fecha_fin"?: "YYYY-MM-DD" | null, "estado"?: "aprobado",
//!                                              "version": "<de GET>", "glosa": "...",
//!                                              "categoria_desvio"?, "detalle_desvio"?}
//!                                       (al menos `fecha_fin` o `estado`)
//!   GET   /aprobaciones/{id}/historial  (requerimiento_historial completo)

use axum::{
    Json,
    extract::{Path, State},
};
use chrono::NaiveDate;
use db::{
    PgPool,
    aprobaciones::{ActualizarError, EdicionRequerimiento},
};
use domain::{Aprobacion, EntradaHistorial, requerimiento::ESTADOS};
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
    pub fecha_inicio_operacion: Option<NaiveDate>,
    pub fecha_inicio_capacitacion: Option<NaiveDate>,
    pub motivo: Option<String>,
    pub solicitante: Option<String>,
    pub ultima_modificacion: Option<ModificacionDto>,
    /// `updated_at` con precisión completa (microsegundos): devolverla tal cual en el PATCH.
    pub version: String,
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
            fecha_inicio_operacion: a.fecha_inicio_operacion,
            fecha_inicio_capacitacion: a.fecha_inicio_capacitacion,
            motivo: a.motivo,
            solicitante: a.solicitante_nombre,
            ultima_modificacion: a.ultima_modificacion.map(|m| ModificacionDto {
                usuario_id: m.usuario_id.map(Into::into),
                fecha: rfc3339(m.fecha),
            }),
            version: a
                .version
                .to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true),
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

/// `fecha_fin`: ausente = no tocar; `null` = quitar la fecha. `estado`: nuevo estado.
/// `version` y `glosa` son obligatorias (las exige `editar_requerimiento_con_historial`).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatchAprobacion {
    #[serde(default, deserialize_with = "presente")]
    pub fecha_fin: Option<Option<NaiveDate>>,
    pub estado: Option<String>,
    pub version: chrono::DateTime<chrono::Utc>,
    pub glosa: String,
    pub categoria_desvio: Option<String>,
    pub detalle_desvio: Option<String>,
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

pub async fn editar(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<PatchAprobacion>,
) -> ApiResult<Json<AprobacionDto>> {
    if body.fecha_fin.is_none() && body.estado.is_none() {
        return Err(ApiError::BadRequest(
            "indicar `fecha_fin` (null para quitarla) y/o `estado`".into(),
        ));
    }
    if let Some(e) = body.estado.as_deref()
        && !ESTADOS.contains(&e)
    {
        return Err(ApiError::Unprocessable(format!(
            "estado inválido: {e} (admitidos: {})",
            ESTADOS.join(", ")
        )));
    }
    let edicion = EdicionRequerimiento {
        fecha_fin: body.fecha_fin,
        estado: body.estado.as_deref(),
        version: body.version,
        glosa: &body.glosa,
        categoria_desvio: body.categoria_desvio.as_deref(),
        detalle_desvio: body.detalle_desvio.as_deref(),
    };

    let mut tx = tx_usuario(&pool, &user).await?;
    let a = db::aprobaciones::actualizar(&mut tx, id.into(), edicion)
        .await
        .map_err(|e| match e {
            ActualizarError::NoEncontrado => ApiError::NotFound,
            ActualizarError::SinPermiso => ApiError::Forbidden,
            ActualizarError::Conflicto => ApiError::Conflict(
                "el requerimiento cambió; volver a cargarlo antes de guardar".into(),
            ),
            ActualizarError::Validacion(e) => ApiError::Unprocessable(e.to_string()),
            ActualizarError::Regla(m) => ApiError::Unprocessable(m),
            ActualizarError::Db(e) => e.into(),
        })?;
    tx.commit().await?;

    tracing::info!(
        requerimiento_id = %id, usuario_id = %user.user_id,
        fecha_fin = ?body.fecha_fin, estado = ?body.estado, "requerimiento editado"
    );
    Ok(Json(AprobacionDto::from(a)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Result<PatchAprobacion, serde_json::Error> {
        serde_json::from_str(s)
    }

    const V: &str = r#""version":"2026-11-03T12:00:00.123456Z","glosa":"extensión""#;

    #[test]
    fn body_distingue_ausente_null_y_fecha() {
        assert_eq!(parse(&format!("{{{V}}}")).unwrap().fecha_fin, None);
        let p = parse(&format!(r#"{{"estado":"aprobado",{V}}}"#)).unwrap();
        assert_eq!((p.fecha_fin, p.estado.as_deref()), (None, Some("aprobado")));
        assert_eq!(
            parse(&format!(r#"{{"fecha_fin":null,{V}}}"#))
                .unwrap()
                .fecha_fin,
            Some(None)
        );
        let p = parse(&format!(r#"{{"fecha_fin":"2026-12-31",{V}}}"#)).unwrap();
        assert_eq!(p.fecha_fin, Some(NaiveDate::from_ymd_opt(2026, 12, 31)));
        assert_eq!(p.glosa, "extensión");
        assert_eq!(
            p.version.timestamp_subsec_micros(),
            123_456,
            "sin perder precisión"
        );
        assert!(parse(&format!(r#"{{"fecha_fin":"31/12/2026",{V}}}"#)).is_err());
        assert!(parse(&format!(r#"{{"fecha_fin":null,"cantidad":9,{V}}}"#)).is_err());
    }

    #[test]
    fn body_exige_version_y_glosa() {
        assert!(parse(r#"{"fecha_fin":null,"glosa":"motivo"}"#).is_err());
        assert!(parse(r#"{"fecha_fin":null,"version":"2026-11-03T12:00:00Z"}"#).is_err());
    }
}
