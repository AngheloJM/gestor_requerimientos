use std::{sync::Arc, time::Duration};

use axum::extract::FromRef;
use sqlx::PgPool;

use crate::{auth::TokenService, limitador::Limitador};

/// Configuración de la cookie del refresh token.
#[derive(Clone, Debug)]
pub struct CookieConfig {
    /// `Secure` (por defecto true; los navegadores lo aceptan en http://localhost).
    pub secure: bool,
    pub path: String,
    pub vida: Duration,
}

impl CookieConfig {
    /// `COOKIE_SECURE` (por defecto true), `COOKIE_PATH` (por defecto "/"),
    /// `REFRESH_TOKEN_DIAS` (por defecto 7, máximo 30).
    pub fn from_env() -> Result<Self, String> {
        let secure = match std::env::var("COOKIE_SECURE").as_deref() {
            Err(_) | Ok("true") | Ok("1") => true,
            Ok("false") | Ok("0") => false,
            Ok(v) => return Err(format!("COOKIE_SECURE inválido: {v}")),
        };
        let dias: u64 = std::env::var("REFRESH_TOKEN_DIAS")
            .ok()
            .map(|v| {
                v.parse()
                    .map_err(|_| format!("REFRESH_TOKEN_DIAS inválido: {v}"))
            })
            .transpose()?
            .unwrap_or(7);
        Ok(Self {
            secure,
            path: std::env::var("COOKIE_PATH").unwrap_or_else(|_| "/".into()),
            vida: Duration::from_secs(dias.clamp(1, 30) * 86_400),
        })
    }
}

/// Estado compartido; handlers y middlewares extraen lo que necesitan vía `FromRef`.
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub tokens: TokenService,
    pub cookies: CookieConfig,
    /// Fallos de login por email (y de cambio de contraseña por usuario).
    pub limite_cuenta: Arc<Limitador>,
    /// Fallos de login por IP.
    pub limite_ip: Arc<Limitador>,
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

impl FromRef<AppState> for TokenService {
    fn from_ref(state: &AppState) -> Self {
        state.tokens.clone()
    }
}
