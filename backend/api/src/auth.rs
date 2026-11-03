//! Tokens de acceso propios (JWT HS256) y middleware de autenticación.
//!
//! - El backend emite y valida sus propios access tokens (vida corta, por defecto 15 min).
//! - La clave sale de `APP_JWT_SECRET` (mínimo 32 bytes; generar con `openssl rand -base64 48`).
//! - Se valida algoritmo (solo HS256), `exp`, `iat`, `iss`, `aud` y `sub`.
//! - El `sub` es `usuarios.id`; se propaga a Postgres para RLS (ver `db::rls`).

use std::{sync::Arc, time::Duration};

use axum::{
    extract::{FromRequestParts, Request, State},
    http::{HeaderMap, header, request::Parts},
    middleware::Next,
    response::{IntoResponse, Response},
};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ApiError;

const ISSUER: &str = "gestor-requerimientos";
const AUDIENCE: &str = "gestor-requerimientos-api";
const MIN_SECRET_BYTES: usize = 32;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: Uuid,
    iat: u64,
    exp: u64,
    iss: String,
    aud: String,
    jti: Uuid,
}

struct Inner {
    encoding: EncodingKey,
    decoding: DecodingKey,
    validation: Validation,
    ttl: Duration,
}

/// Emite y verifica access tokens.
#[derive(Clone)]
pub struct TokenService {
    inner: Arc<Inner>,
}

impl TokenService {
    pub fn new(secret: &[u8], ttl: Duration) -> Result<Self, String> {
        if secret.len() < MIN_SECRET_BYTES {
            return Err(format!(
                "APP_JWT_SECRET debe tener al menos {MIN_SECRET_BYTES} bytes (generar con `openssl rand -base64 48`)"
            ));
        }
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[ISSUER]);
        validation.set_audience(&[AUDIENCE]);
        validation.set_required_spec_claims(&["exp", "iat", "sub", "iss", "aud"]);
        validation.leeway = 30;
        Ok(Self {
            inner: Arc::new(Inner {
                encoding: EncodingKey::from_secret(secret),
                decoding: DecodingKey::from_secret(secret),
                validation,
                ttl,
            }),
        })
    }

    /// `APP_JWT_SECRET` (obligatoria) y `ACCESS_TOKEN_MIN` (opcional, por defecto 15).
    pub fn from_env() -> Result<Self, String> {
        let secret = std::env::var("APP_JWT_SECRET")
            .map_err(|_| "falta la variable de entorno APP_JWT_SECRET".to_string())?;
        let min: u64 = std::env::var("ACCESS_TOKEN_MIN")
            .ok()
            .map(|v| {
                v.parse()
                    .map_err(|_| format!("ACCESS_TOKEN_MIN inválido: {v}"))
            })
            .transpose()?
            .unwrap_or(15);
        Self::new(
            secret.trim().as_bytes(),
            Duration::from_secs(min.clamp(1, 60) * 60),
        )
    }

    pub fn ttl(&self) -> Duration {
        self.inner.ttl
    }

    pub fn emitir(&self, usuario_id: Uuid) -> Result<String, ApiError> {
        let ahora = ahora_unix();
        let claims = Claims {
            sub: usuario_id,
            iat: ahora,
            exp: ahora + self.inner.ttl.as_secs(),
            iss: ISSUER.into(),
            aud: AUDIENCE.into(),
            jti: Uuid::new_v4(),
        };
        encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &self.inner.encoding,
        )
        .map_err(|e| ApiError::Internal(format!("no se pudo firmar el token: {e}")))
    }

    pub fn verificar(&self, token: &str) -> Result<Uuid, jsonwebtoken::errors::ErrorKind> {
        decode::<Claims>(token, &self.inner.decoding, &self.inner.validation)
            .map(|d| d.claims.sub)
            .map_err(|e| e.into_kind())
    }
}

fn ahora_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("reloj del sistema válido")
        .as_secs()
}

/// Usuario autenticado, inyectado por [`require_auth`]. Usable como extractor.
#[derive(Debug, Clone, Copy)]
pub struct AuthUser {
    pub user_id: Uuid,
}

impl<S: Send + Sync> FromRequestParts<S> for AuthUser {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthUser>()
            .copied()
            .ok_or_else(|| ApiError::Unauthorized.into_response())
    }
}

/// Middleware: exige `Authorization: Bearer <access token>` válido; si no, 401.
pub async fn require_auth(
    State(tokens): State<TokenService>,
    mut req: Request,
    next: Next,
) -> Response {
    let Some(token) = bearer_token(req.headers()) else {
        return ApiError::Unauthorized.into_response();
    };
    match tokens.verificar(token) {
        Ok(user_id) => {
            req.extensions_mut().insert(AuthUser { user_id });
            next.run(req).await
        }
        Err(motivo) => {
            // Solo el motivo; nunca el token.
            tracing::debug!(?motivo, "access token rechazado");
            ApiError::Unauthorized.into_response()
        }
    }
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    let token = token.trim();
    (scheme.eq_ignore_ascii_case("bearer") && !token.is_empty()).then_some(token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, body::Body, http::StatusCode, middleware, routing::get};
    use jsonwebtoken::errors::ErrorKind;
    use tower::ServiceExt;

    const SECRET: &[u8] = b"secreto-de-prueba-de-al-menos-32-bytes!!";

    fn svc() -> TokenService {
        TokenService::new(SECRET, Duration::from_secs(900)).unwrap()
    }

    fn firmar(claims: serde_json::Value, alg: Algorithm, secret: &[u8]) -> String {
        encode(
            &Header::new(alg),
            &claims,
            &EncodingKey::from_secret(secret),
        )
        .unwrap()
    }

    fn claims_base() -> serde_json::Value {
        let ahora = ahora_unix();
        serde_json::json!({
            "sub": Uuid::nil(), "iat": ahora, "exp": ahora + 600,
            "iss": ISSUER, "aud": AUDIENCE, "jti": Uuid::new_v4()
        })
    }

    #[test]
    fn exige_secreto_largo() {
        assert!(TokenService::new(b"corto", Duration::from_secs(60)).is_err());
    }

    #[test]
    fn emite_y_verifica() {
        let s = svc();
        let u = Uuid::new_v4();
        assert_eq!(s.verificar(&s.emitir(u).unwrap()), Ok(u));
    }

    #[test]
    fn rechazos() {
        let s = svc();
        let mut c = claims_base();
        c["exp"] = (ahora_unix() - 120).into();
        assert_eq!(
            s.verificar(&firmar(c, Algorithm::HS256, SECRET)),
            Err(ErrorKind::ExpiredSignature)
        );

        let mut c = claims_base();
        c["aud"] = "otra".into();
        assert_eq!(
            s.verificar(&firmar(c, Algorithm::HS256, SECRET)),
            Err(ErrorKind::InvalidAudience)
        );

        let mut c = claims_base();
        c["iss"] = "supabase".into();
        assert_eq!(
            s.verificar(&firmar(c, Algorithm::HS256, SECRET)),
            Err(ErrorKind::InvalidIssuer)
        );

        let otro = b"otro-secreto-de-al-menos-treinta-y-dos-bytes";
        assert_eq!(
            s.verificar(&firmar(claims_base(), Algorithm::HS256, otro)),
            Err(ErrorKind::InvalidSignature)
        );
        // Algoritmo distinto (HS512) → rechazado aunque la firma sea con el mismo secreto.
        assert_eq!(
            s.verificar(&firmar(claims_base(), Algorithm::HS512, SECRET)),
            Err(ErrorKind::InvalidAlgorithm)
        );
        // alg "none"
        let partes: Vec<String> = s
            .emitir(Uuid::nil())
            .unwrap()
            .split('.')
            .map(String::from)
            .collect();
        use base64::Engine;
        let h = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(r#"{"alg":"none","typ":"JWT"}"#);
        assert!(s.verificar(&format!("{h}.{}.", partes[1])).is_err());
    }

    #[tokio::test]
    async fn middleware() {
        let s = svc();
        let app = Router::new()
            .route(
                "/me",
                get(|u: AuthUser| async move { u.user_id.to_string() }),
            )
            .layer(middleware::from_fn_with_state(s.clone(), require_auth));
        let llamar = |auth: Option<String>| {
            let app = app.clone();
            async move {
                let mut r = Request::builder().uri("/me");
                if let Some(a) = auth {
                    r = r.header(header::AUTHORIZATION, a);
                }
                app.oneshot(r.body(Body::empty()).unwrap())
                    .await
                    .unwrap()
                    .status()
            }
        };
        let t = s.emitir(Uuid::new_v4()).unwrap();
        assert_eq!(llamar(Some(format!("Bearer {t}"))).await, StatusCode::OK);
        assert_eq!(llamar(None).await, StatusCode::UNAUTHORIZED);
        assert_eq!(
            llamar(Some(format!("Basic {t}"))).await,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            llamar(Some("Bearer x.y.z".into())).await,
            StatusCode::UNAUTHORIZED
        );
    }
}
