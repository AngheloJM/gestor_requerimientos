use axum::extract::FromRef;
use sqlx::PgPool;

use crate::auth::JwtVerifier;

/// Estado compartido de la aplicación. Los middlewares y handlers extraen
/// solo la parte que necesitan (`State<PgPool>`, `State<JwtVerifier>`) vía `FromRef`.
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub jwt: JwtVerifier,
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

impl FromRef<AppState> for JwtVerifier {
    fn from_ref(state: &AppState) -> Self {
        state.jwt.clone()
    }
}
