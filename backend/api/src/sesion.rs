//! Endpoints de sesión:
//!   POST /auth/login              {email, password} → access token + cookie de refresh
//!   POST /auth/refresh            (cookie) → rota el refresh token y emite access token
//!   POST /auth/logout             (cookie) → revoca la sesión
//!   POST /auth/cambiar-password   (Bearer) {password_actual, password_nueva}
//!
//! Medidas: argon2id, mensaje de error genérico, tiempo constante ante email inexistente,
//! límite de intentos por cuenta e IP, refresh token opaco en cookie HttpOnly + Secure +
//! SameSite=Strict (solo se guarda su SHA-256), rotación con detección de reuso, y encabezado
//! anti-CSRF obligatorio (`X-CSRF: 1`) en todos los POST de /auth.

use std::net::{IpAddr, SocketAddr};

use axum::{
    Json,
    extract::{ConnectInfo, FromRequestParts, State},
    http::{HeaderMap, StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use db::auth::{NuevaSesion, ResultadoRotacion};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::{ApiError, ApiResult},
    passwords,
    state::{AppState, CookieConfig},
};

pub const COOKIE_REFRESH: &str = "gr_refresh";
pub const HEADER_CSRF: &str = "x-csrf";

/// IP del cliente (conexión directa).
/// TODO(infra): detrás de un proxy inverso, leer `X-Forwarded-For` solo de proxies confiables.
pub struct IpCliente(pub Option<IpAddr>);

impl<S: Send + Sync> FromRequestParts<S> for IpCliente {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(
            parts
                .extensions
                .get::<ConnectInfo<SocketAddr>>()
                .map(|c| c.0.ip()),
        ))
    }
}

fn exigir_csrf(headers: &HeaderMap) -> ApiResult<()> {
    match headers.get(HEADER_CSRF).and_then(|v| v.to_str().ok()) {
        Some("1") => Ok(()),
        _ => Err(ApiError::BadRequest("falta el encabezado X-CSRF".into())),
    }
}

fn user_agent(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
}

/// Refresh token opaco: 256 bits aleatorios, base64url.
fn nuevo_refresh_token() -> String {
    let mut b = [0u8; 32];
    OsRng.fill_bytes(&mut b);
    URL_SAFE_NO_PAD.encode(b)
}

fn hash_token(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

fn cookie_refresh(cfg: &CookieConfig, valor: String) -> Cookie<'static> {
    Cookie::build((COOKIE_REFRESH, valor))
        .http_only(true)
        .secure(cfg.secure)
        .same_site(SameSite::Strict)
        .path(cfg.path.clone())
        .max_age(time_dur(cfg.vida))
        .build()
}

fn cookie_borrada(cfg: &CookieConfig) -> Cookie<'static> {
    let mut c = cookie_refresh(cfg, String::new());
    c.make_removal();
    c
}

fn time_dur(d: std::time::Duration) -> time::Duration {
    time::Duration::seconds(i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

#[derive(Debug, Serialize)]
pub struct TokenDto {
    pub access_token: String,
    pub token_type: &'static str,
    /// Segundos de vida del access token.
    pub expires_in: u64,
}

fn token_dto(st: &AppState, usuario_id: Uuid) -> ApiResult<Json<TokenDto>> {
    Ok(Json(TokenDto {
        access_token: st.tokens.emitir(usuario_id)?,
        token_type: "Bearer",
        expires_in: st.tokens.ttl().as_secs(),
    }))
}

async fn abrir_sesion(
    st: &AppState,
    usuario_id: Uuid,
    headers: &HeaderMap,
    ip: Option<IpAddr>,
) -> ApiResult<String> {
    let token = nuevo_refresh_token();
    let hash = hash_token(&token);
    db::auth::crear_sesion(
        &st.db,
        &NuevaSesion {
            usuario_id,
            token_hash: &hash,
            expira_en: Utc::now() + st.cookies.vida,
            user_agent: user_agent(headers),
            ip,
        },
    )
    .await?;
    Ok(token)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoginBody {
    pub email: String,
    pub password: String,
}

pub async fn login(
    State(st): State<AppState>,
    IpCliente(ip): IpCliente,
    headers: HeaderMap,
    jar: CookieJar,
    Json(body): Json<LoginBody>,
) -> ApiResult<(CookieJar, Json<TokenDto>)> {
    exigir_csrf(&headers)?;
    let email = body.email.trim().to_lowercase();
    let clave_cuenta = format!("cuenta:{email}");
    let clave_ip = format!("ip:{}", ip.map_or("?".into(), |i| i.to_string()));

    for clave in [&clave_cuenta, &clave_ip] {
        let limite = if clave.starts_with("ip:") {
            &st.limite_ip
        } else {
            &st.limite_cuenta
        };
        if let Some(espera) = limite.bloqueado(clave) {
            tracing::warn!(ip = ?ip, "login bloqueado por demasiados intentos");
            return Err(ApiError::TooManyRequests(espera.as_secs().max(1)));
        }
    }

    // Entradas absurdas no se hashean (evita abuso de CPU); cuentan como fallo.
    let razonable = email.len() <= 254 && body.password.len() <= passwords::LONGITUD_MAX * 4;
    let cred = if razonable {
        db::auth::credencial_por_email(&st.db, &email).await?
    } else {
        None
    };
    let ok = razonable
        && passwords::verificar(
            cred.as_ref().map(|c| c.password_hash.clone()),
            body.password,
        )
        .await?;

    match cred {
        Some(c) if ok && c.activo => {
            st.limite_cuenta.limpiar(&clave_cuenta);
            let refresh = abrir_sesion(&st, c.usuario_id, &headers, ip).await?;
            tracing::info!(usuario_id = %c.usuario_id, "login exitoso");
            let dto = token_dto(&st, c.usuario_id)?;
            Ok((jar.add(cookie_refresh(&st.cookies, refresh)), dto))
        }
        _ => {
            st.limite_cuenta.registrar_fallo(&clave_cuenta);
            st.limite_ip.registrar_fallo(&clave_ip);
            tracing::warn!(ip = ?ip, "login fallido");
            Err(ApiError::Unauthorized)
        }
    }
}

pub async fn refresh(
    State(st): State<AppState>,
    IpCliente(ip): IpCliente,
    headers: HeaderMap,
    jar: CookieJar,
) -> Response {
    if let Err(e) = exigir_csrf(&headers) {
        return e.into_response();
    }
    let Some(actual) = jar.get(COOKIE_REFRESH).map(|c| c.value().to_string()) else {
        return ApiError::Unauthorized.into_response();
    };
    let nuevo = nuevo_refresh_token();
    let nuevo_hash = hash_token(&nuevo);
    let res = db::auth::rotar_sesion(
        &st.db,
        &hash_token(&actual),
        &NuevaSesion {
            usuario_id: Uuid::nil(), // lo fija la sesión rotada
            token_hash: &nuevo_hash,
            expira_en: Utc::now() + st.cookies.vida,
            user_agent: user_agent(&headers),
            ip,
        },
    )
    .await;

    // En cualquier rechazo se borra la cookie del navegador.
    let rechazo = |e: ApiError| (jar.clone().add(cookie_borrada(&st.cookies)), e).into_response();
    match res {
        Ok(ResultadoRotacion::Rotada { usuario_id }) => match token_dto(&st, usuario_id) {
            Ok(dto) => (jar.clone().add(cookie_refresh(&st.cookies, nuevo)), dto).into_response(),
            Err(e) => rechazo(e),
        },
        Ok(ResultadoRotacion::Invalida) => rechazo(ApiError::Unauthorized),
        Ok(ResultadoRotacion::Reutilizada { usuario_id }) => {
            tracing::warn!(%usuario_id, ip = ?ip, "reuso de refresh token: sesiones revocadas");
            rechazo(ApiError::Unauthorized)
        }
        Err(e) => ApiError::from(e).into_response(),
    }
}

pub async fn logout(
    State(st): State<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
) -> ApiResult<(CookieJar, StatusCode)> {
    exigir_csrf(&headers)?;
    if let Some(c) = jar.get(COOKIE_REFRESH) {
        db::auth::revocar_sesion(&st.db, &hash_token(c.value())).await?;
    }
    Ok((jar.add(cookie_borrada(&st.cookies)), StatusCode::NO_CONTENT))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CambiarPasswordBody {
    pub password_actual: String,
    pub password_nueva: String,
}

/// Cambia la contraseña y revoca TODAS las sesiones (hay que volver a iniciar sesión).
pub async fn cambiar_password(
    State(st): State<AppState>,
    user: AuthUser,
    headers: HeaderMap,
    jar: CookieJar,
    Json(body): Json<CambiarPasswordBody>,
) -> ApiResult<(CookieJar, StatusCode)> {
    exigir_csrf(&headers)?;
    let clave = format!("pw:{}", user.user_id);
    if let Some(espera) = st.limite_cuenta.bloqueado(&clave) {
        return Err(ApiError::TooManyRequests(espera.as_secs().max(1)));
    }
    passwords::validar_politica(&body.password_nueva).map_err(ApiError::Unprocessable)?;
    if body.password_nueva == body.password_actual {
        return Err(ApiError::Unprocessable(
            "la contraseña nueva debe ser distinta de la actual".into(),
        ));
    }

    let cred = db::auth::credencial_por_usuario(&st.db, user.user_id)
        .await?
        .ok_or(ApiError::Forbidden)?;
    if !passwords::verificar(Some(cred.password_hash), body.password_actual).await? {
        st.limite_cuenta.registrar_fallo(&clave);
        return Err(ApiError::Unprocessable(
            "la contraseña actual no es correcta".into(),
        ));
    }
    st.limite_cuenta.limpiar(&clave);

    let hash = passwords::hashear(body.password_nueva).await?;
    db::auth::actualizar_password(&st.db, user.user_id, &hash).await?;
    let n = db::auth::revocar_todas(&st.db, user.user_id).await?;
    tracing::info!(usuario_id = %user.user_id, sesiones_revocadas = n, "contraseña cambiada");
    Ok((jar.add(cookie_borrada(&st.cookies)), StatusCode::NO_CONTENT))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn cfg() -> CookieConfig {
        CookieConfig {
            secure: true,
            path: "/".into(),
            vida: Duration::from_secs(7 * 86_400),
        }
    }

    #[test]
    fn cookie_segura() {
        let c = cookie_refresh(&cfg(), "abc".into()).to_string();
        for atributo in [
            "HttpOnly",
            "Secure",
            "SameSite=Strict",
            "Path=/",
            "Max-Age=604800",
        ] {
            assert!(c.contains(atributo), "{c} no contiene {atributo}");
        }
        let b = cookie_borrada(&cfg()).to_string();
        assert!(b.contains("Max-Age=0"), "{b}");
    }

    #[test]
    fn refresh_tokens_aleatorios_y_hash_estable() {
        let a = nuevo_refresh_token();
        let b = nuevo_refresh_token();
        assert_ne!(a, b);
        assert_eq!(a.len(), 43); // 32 bytes en base64url sin relleno
        assert_eq!(hash_token(&a), hash_token(&a));
        assert_eq!(hash_token(&a).len(), 32);
    }

    #[test]
    fn csrf() {
        let mut h = HeaderMap::new();
        assert!(exigir_csrf(&h).is_err());
        h.insert(HEADER_CSRF, "0".parse().unwrap());
        assert!(exigir_csrf(&h).is_err());
        h.insert(HEADER_CSRF, "1".parse().unwrap());
        assert!(exigir_csrf(&h).is_ok());
    }
}
