//! GET /me — perfil del usuario autenticado (leído bajo RLS).

use axum::{Json, extract::State};
use db::PgPool;
use serde::Serialize;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::{ApiError, ApiResult, tx_usuario},
};

#[derive(Debug, Serialize)]
pub struct MeDto {
    pub user_id: Uuid,
    pub nombre: Option<String>,
    /// Tal cual `usuarios.rol` (rrhh_admin | estandar | encargado_costos | externo).
    /// Solo para la UI (menú); la autorización real la aplica RLS.
    pub rol: Option<String>,
    pub activo: Option<bool>,
}

pub async fn get_me(State(pool): State<PgPool>, user: AuthUser) -> ApiResult<Json<MeDto>> {
    let mut tx = tx_usuario(&pool, &user).await?;
    let perfil = db::usuarios::perfil(&mut tx, user.user_id)
        .await?
        // Token válido pero sin fila visible en `usuarios`.
        .ok_or(ApiError::Forbidden)?;
    Ok(Json(MeDto {
        user_id: perfil.id,
        nombre: perfil.nombre,
        rol: perfil.rol,
        activo: perfil.activo,
    }))
}
