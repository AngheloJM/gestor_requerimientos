//! Errores HTTP uniformes: `{"error": "<codigo>", "detalle"?: "..."}`.

use axum::{
    Json,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use db::{DbError, PgPool};
use serde_json::json;
use sqlx::{Postgres, Transaction};

use crate::auth::AuthUser;

#[derive(Debug)]
pub enum ApiError {
    /// Sin token, token inválido o credenciales incorrectas (mensaje siempre genérico).
    Unauthorized,
    /// Demasiados intentos: `Retry-After` en segundos.
    TooManyRequests(u64),
    /// RLS / permisos de Postgres rechazaron la operación.
    Forbidden,
    NotFound,
    BadRequest(String),
    Unprocessable(String),
    /// El detalle se registra en el log, nunca se envía al cliente.
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut extra: Option<(header::HeaderName, String)> = None;
        let (status, codigo, detalle) = match self {
            Self::Unauthorized => {
                extra = Some((header::WWW_AUTHENTICATE, "Bearer".into()));
                (StatusCode::UNAUTHORIZED, "unauthorized", None)
            }
            Self::TooManyRequests(seg) => {
                extra = Some((header::RETRY_AFTER, seg.to_string()));
                (StatusCode::TOO_MANY_REQUESTS, "too_many_requests", None)
            }
            Self::Forbidden => (StatusCode::FORBIDDEN, "forbidden", None),
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found", None),
            Self::BadRequest(d) => (StatusCode::BAD_REQUEST, "bad_request", Some(d)),
            Self::Unprocessable(d) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_error",
                Some(d),
            ),
            Self::Internal(d) => {
                tracing::error!(error = %d, "error interno");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", None)
            }
        };
        let body = match detalle {
            Some(d) => json!({ "error": codigo, "detalle": d }),
            None => json!({ "error": codigo }),
        };
        let mut res = (status, Json(body)).into_response();
        if let Some((nombre, Ok(v))) = extra.map(|(n, v)| (n, v.parse())) {
            res.headers_mut().insert(nombre, v);
        }
        res
    }
}

impl From<DbError> for ApiError {
    fn from(e: DbError) -> Self {
        if e.es_permiso_denegado() {
            Self::Forbidden
        } else {
            Self::Internal(e.to_string())
        }
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        DbError::Sqlx(e).into()
    }
}

/// Transacción con la identidad del usuario propagada a Postgres (RLS).
/// Devolverla sin `commit` = rollback (lecturas).
pub async fn tx_usuario(
    pool: &PgPool,
    user: &AuthUser,
) -> Result<Transaction<'static, Postgres>, ApiError> {
    Ok(db::rls::begin_como_usuario(pool, user.user_id.into()).await?)
}

pub type ApiResult<T> = Result<T, ApiError>;
