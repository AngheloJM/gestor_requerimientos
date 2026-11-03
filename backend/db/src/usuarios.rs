//! Perfil del usuario autenticado (`usuarios`, `id` = `auth.users.id`).

use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::DbError;

#[derive(Debug, Clone, FromRow)]
pub struct Perfil {
    pub id: Uuid,
    pub nombre: Option<String>,
    pub rol: Option<String>,
    pub activo: Option<bool>,
}

/// Perfil del usuario, si RLS permite verlo.
pub async fn perfil(conn: &mut PgConnection, id: Uuid) -> Result<Option<Perfil>, DbError> {
    Ok(sqlx::query_as(
        "SELECT id, nombre::text AS nombre, rol::text AS rol, activo FROM usuarios WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(conn)
    .await?)
}
