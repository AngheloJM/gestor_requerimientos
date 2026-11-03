//! Credenciales (hash argon2id) y sesiones (refresh tokens) del login propio.
//!
//! Estas consultas corren con el rol de conexión del backend (no con `authenticated`):
//! ocurren antes de conocer la identidad, y las tablas no son accesibles para los roles de
//! la API pública (ver migración 0002). Nunca se guarda el refresh token, solo su SHA-256.

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgConnection, PgPool};
use uuid::Uuid;

use crate::DbError;

#[derive(Debug, Clone, FromRow)]
pub struct Credencial {
    pub usuario_id: Uuid,
    pub password_hash: String,
    /// `usuarios.activo`: un usuario inactivo no puede iniciar sesión.
    pub activo: bool,
}

/// Busca por email (se normaliza a minúsculas).
pub async fn credencial_por_email(
    pool: &PgPool,
    email: &str,
) -> Result<Option<Credencial>, DbError> {
    Ok(sqlx::query_as(
        "SELECT c.usuario_id, c.password_hash, coalesce(u.activo, false) AS activo
         FROM credenciales c JOIN usuarios u ON u.id = c.usuario_id
         WHERE c.email = lower($1)",
    )
    .bind(email.trim())
    .fetch_optional(pool)
    .await?)
}

pub async fn credencial_por_usuario(
    pool: &PgPool,
    usuario_id: Uuid,
) -> Result<Option<Credencial>, DbError> {
    Ok(sqlx::query_as(
        "SELECT c.usuario_id, c.password_hash, coalesce(u.activo, false) AS activo
         FROM credenciales c JOIN usuarios u ON u.id = c.usuario_id
         WHERE c.usuario_id = $1",
    )
    .bind(usuario_id)
    .fetch_optional(pool)
    .await?)
}

/// Crea o reemplaza la credencial de un usuario existente.
pub async fn guardar_credencial(
    pool: &PgPool,
    usuario_id: Uuid,
    email: &str,
    password_hash: &str,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO credenciales (usuario_id, email, password_hash) VALUES ($1, lower($2), $3)
         ON CONFLICT (usuario_id) DO UPDATE
           SET email = excluded.email, password_hash = excluded.password_hash, actualizado_en = now()",
    )
    .bind(usuario_id)
    .bind(email.trim())
    .bind(password_hash)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn actualizar_password(
    pool: &PgPool,
    usuario_id: Uuid,
    password_hash: &str,
) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE credenciales SET password_hash = $2, actualizado_en = now() WHERE usuario_id = $1",
    )
    .bind(usuario_id)
    .bind(password_hash)
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct NuevaSesion<'a> {
    pub usuario_id: Uuid,
    pub token_hash: &'a [u8],
    pub expira_en: DateTime<Utc>,
    pub user_agent: Option<&'a str>,
    pub ip: Option<std::net::IpAddr>,
}

async fn insertar_sesion(conn: &mut PgConnection, s: &NuevaSesion<'_>) -> Result<Uuid, DbError> {
    Ok(sqlx::query_scalar(
        "INSERT INTO sesiones (usuario_id, token_hash, expira_en, user_agent, ip)
         VALUES ($1, $2, $3, left($4, 300), $5::text::inet) RETURNING id",
    )
    .bind(s.usuario_id)
    .bind(s.token_hash)
    .bind(s.expira_en)
    .bind(s.user_agent)
    .bind(s.ip.map(|ip| ip.to_string()))
    .fetch_one(conn)
    .await?)
}

pub async fn crear_sesion(pool: &PgPool, s: &NuevaSesion<'_>) -> Result<Uuid, DbError> {
    let mut conn = pool.acquire().await?;
    insertar_sesion(&mut conn, s).await
}

#[derive(Debug, PartialEq, Eq)]
pub enum ResultadoRotacion {
    /// Sesión válida: se revocó la anterior y se creó la nueva.
    Rotada { usuario_id: Uuid },
    /// Token desconocido, expirado o de un usuario inactivo.
    Invalida,
    /// Se presentó un token YA rotado/revocado: posible robo. Se revocaron todas las
    /// sesiones del usuario.
    Reutilizada { usuario_id: Uuid },
}

#[derive(FromRow)]
struct SesionRow {
    id: Uuid,
    usuario_id: Uuid,
    expira_en: DateTime<Utc>,
    revocada_en: Option<DateTime<Utc>>,
    activo: bool,
}

/// Rota un refresh token de forma atómica (bloquea la fila).
pub async fn rotar_sesion(
    pool: &PgPool,
    token_hash_actual: &[u8],
    nueva: &NuevaSesion<'_>,
) -> Result<ResultadoRotacion, DbError> {
    let mut tx = pool.begin().await?;
    let fila: Option<SesionRow> = sqlx::query_as(
        "SELECT s.id, s.usuario_id, s.expira_en, s.revocada_en, coalesce(u.activo, false) AS activo
         FROM sesiones s JOIN usuarios u ON u.id = s.usuario_id
         WHERE s.token_hash = $1 FOR UPDATE OF s",
    )
    .bind(token_hash_actual)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(s) = fila else {
        return Ok(ResultadoRotacion::Invalida);
    };
    if s.revocada_en.is_some() {
        revocar_todas_en(&mut tx, s.usuario_id).await?;
        tx.commit().await?;
        return Ok(ResultadoRotacion::Reutilizada {
            usuario_id: s.usuario_id,
        });
    }
    if s.expira_en <= Utc::now() || !s.activo {
        return Ok(ResultadoRotacion::Invalida);
    }

    let nueva = NuevaSesion {
        usuario_id: s.usuario_id,
        ..nueva.clone()
    };
    let nueva_id = insertar_sesion(&mut tx, &nueva).await?;
    sqlx::query("UPDATE sesiones SET revocada_en = now(), reemplazada_por = $2 WHERE id = $1")
        .bind(s.id)
        .bind(nueva_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(ResultadoRotacion::Rotada {
        usuario_id: s.usuario_id,
    })
}

/// Revoca una sesión (logout). Idempotente.
pub async fn revocar_sesion(pool: &PgPool, token_hash: &[u8]) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE sesiones SET revocada_en = now() WHERE token_hash = $1 AND revocada_en IS NULL",
    )
    .bind(token_hash)
    .execute(pool)
    .await?;
    Ok(())
}

async fn revocar_todas_en(conn: &mut PgConnection, usuario_id: Uuid) -> Result<u64, DbError> {
    Ok(sqlx::query(
        "UPDATE sesiones SET revocada_en = now() WHERE usuario_id = $1 AND revocada_en IS NULL",
    )
    .bind(usuario_id)
    .execute(conn)
    .await?
    .rows_affected())
}

/// Revoca todas las sesiones activas del usuario (ej. tras cambiar la contraseña).
pub async fn revocar_todas(pool: &PgPool, usuario_id: Uuid) -> Result<u64, DbError> {
    let mut conn = pool.acquire().await?;
    revocar_todas_en(&mut conn, usuario_id).await
}
