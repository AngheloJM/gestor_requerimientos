//! Solicitudes de cambio (Kanban del tablero).
//!
//!   POST /solicitudes-cambio              {servicio_id, cargo_id?, campo, motivo, descripcion?} → 201 {id}
//!   POST /solicitudes-cambio/{id}/cierre  {accion: resolver|rechazar|cancelar, comentario?}    → 204
//!
//! Permisos y transiciones: la base (RLS + trigger `validar_solicitud_cambio`). Aquí solo se
//! validan formatos para responder 422 en lugar de un error de CHECK.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use db::{
    PgPool,
    solicitudes::{CAMPOS, CerrarError, Cierre, NuevaSolicitud},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::{ApiError, ApiResult, tx_usuario},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NuevaBody {
    pub servicio_id: Uuid,
    pub cargo_id: Option<Uuid>,
    pub campo: String,
    pub motivo: String,
    pub descripcion: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreadaDto {
    pub id: Uuid,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Accion {
    Resolver,
    Rechazar,
    Cancelar,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CierreBody {
    pub accion: Accion,
    pub comentario: Option<String>,
}

fn validar_nueva(b: &NuevaBody) -> Result<(), ApiError> {
    if !CAMPOS.contains(&b.campo.as_str()) {
        return Err(ApiError::Unprocessable(format!(
            "campo inválido: {}",
            b.campo
        )));
    }
    if matches!(b.campo.as_str(), "sueldo_basico" | "bonos") && b.cargo_id.is_none() {
        return Err(ApiError::Unprocessable(
            "indicá el cargo para cambios de sueldo o bonos".into(),
        ));
    }
    let largo = b.motivo.trim().chars().count();
    if !(3..=500).contains(&largo) {
        return Err(ApiError::Unprocessable(
            "el motivo debe tener entre 3 y 500 caracteres".into(),
        ));
    }
    if b.descripcion
        .as_deref()
        .is_some_and(|d| d.chars().count() > 2000)
    {
        return Err(ApiError::Unprocessable(
            "la descripción admite hasta 2000 caracteres".into(),
        ));
    }
    Ok(())
}

pub async fn crear(
    State(pool): State<PgPool>,
    user: AuthUser,
    Json(b): Json<NuevaBody>,
) -> ApiResult<(StatusCode, Json<CreadaDto>)> {
    validar_nueva(&b)?;
    let mut tx = tx_usuario(&pool, &user).await?;
    let id = db::solicitudes::crear(
        &mut tx,
        &NuevaSolicitud {
            servicio_id: b.servicio_id.into(),
            cargo_id: b.cargo_id.map(Into::into),
            campo: &b.campo,
            motivo: &b.motivo,
            descripcion: b.descripcion.as_deref(),
        },
    )
    .await?;
    tx.commit().await?;
    tracing::info!(usuario_id = %user.user_id, solicitud = %id, campo = %b.campo, "solicitud de cambio creada");
    Ok((StatusCode::CREATED, Json(CreadaDto { id })))
}

pub async fn cerrar(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(b): Json<CierreBody>,
) -> ApiResult<StatusCode> {
    if b.comentario
        .as_deref()
        .is_some_and(|c| c.chars().count() > 2000)
    {
        return Err(ApiError::Unprocessable(
            "el comentario admite hasta 2000 caracteres".into(),
        ));
    }
    let cierre = match b.accion {
        Accion::Resolver => Cierre::Resolver,
        Accion::Rechazar => Cierre::Rechazar,
        Accion::Cancelar => Cierre::Cancelar,
    };
    let mut tx = tx_usuario(&pool, &user).await?;
    db::solicitudes::cerrar(&mut tx, id, cierre, b.comentario.as_deref())
        .await
        .map_err(|e| match e {
            CerrarError::NoDisponible => ApiError::Conflict(
                "la solicitud no existe, ya está cerrada o no podés modificarla".into(),
            ),
            CerrarError::Db(e) => e.into(),
        })?;
    tx.commit().await?;
    tracing::info!(usuario_id = %user.user_id, solicitud = %id, accion = ?b.accion, "solicitud de cambio cerrada");
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valida_campo_cargo_y_motivo() {
        let b = |campo: &str, cargo: bool, motivo: &str| NuevaBody {
            servicio_id: Uuid::new_v4(),
            cargo_id: cargo.then(Uuid::new_v4),
            campo: campo.into(),
            motivo: motivo.into(),
            descripcion: None,
        };
        assert!(validar_nueva(&b("modalidad", false, "Cambió")).is_ok());
        assert!(validar_nueva(&b("otro", false, "Cambió")).is_err());
        assert!(validar_nueva(&b("bonos", false, "Cambió")).is_err());
        assert!(validar_nueva(&b("bonos", true, "  ab ")).is_err());
        let accion: CierreBody =
            serde_json::from_str(r#"{"accion":"resolver","comentario":"ok"}"#).unwrap();
        assert_eq!(accion.accion, Accion::Resolver);
        assert!(serde_json::from_str::<CierreBody>(r#"{"accion":"aprobar"}"#).is_err());
    }
}
