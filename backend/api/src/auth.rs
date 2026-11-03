//! Autenticación: validación del JWT emitido por Supabase Auth.
//!
//! - **Principal: claves asimétricas (ES256 / RS256 / EdDSA) publicadas en el JWKS** de
//!   `{SUPABASE_URL}/auth/v1/.well-known/jwks.json`. El JWKS se descarga al arrancar, se
//!   refresca periódicamente y también bajo demanda cuando llega un `kid` desconocido
//!   (rotación de claves), con un cooldown para que tokens con `kid` inventados no
//!   provoquen una descarga por request.
//! - **Fallback legacy HS256 (opcional, desactivado por defecto):** solo si se define
//!   `SUPABASE_JWT_SECRET`. Pensado para la transición; quitarlo cuando no queden tokens
//!   HS256 vigentes.
//!
//! El algoritmo con el que se verifica sale **de la clave del JWKS**, nunca solo del header
//! del token (evita ataques de confusión de algoritmo).

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    Json,
    extract::{FromRequestParts, Request, State},
    http::{StatusCode, header, request::Parts},
    middleware::Next,
    response::{IntoResponse, Response},
};
use jsonwebtoken::{
    Algorithm, DecodingKey, Validation, decode, decode_header,
    jwk::{AlgorithmParameters, EllipticCurve, Jwk, JwkSet},
};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

/// Audiencia que Supabase asigna a los usuarios logueados.
const SUPABASE_AUDIENCE: &str = "authenticated";
/// Algoritmos asimétricos aceptados desde el JWKS.
const ALGORITMOS_JWKS: [Algorithm; 4] = [
    Algorithm::ES256,
    Algorithm::RS256,
    Algorithm::EdDSA,
    Algorithm::ES384,
];

#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub jwks_url: String,
    /// Valor esperado del claim `iss` (`{SUPABASE_URL}/auth/v1`).
    pub issuer: String,
    /// Habilita el fallback HS256. `None` = tokens HS256 rechazados.
    pub legacy_hs256_secret: Option<String>,
    pub refresh_interval: Duration,
    /// Tiempo mínimo entre refrescos disparados por un `kid` desconocido.
    pub unknown_kid_cooldown: Duration,
    pub http_timeout: Duration,
}

impl JwtConfig {
    /// Construye la configuración desde la URL del proyecto (`https://<ref>.supabase.co`).
    pub fn for_project(supabase_url: &str) -> Result<Self, String> {
        let base = supabase_url.trim().trim_end_matches('/');
        let es_local = base.starts_with("http://127.0.0.1") || base.starts_with("http://localhost");
        if !base.starts_with("https://") && !es_local {
            return Err(format!("SUPABASE_URL debe usar https: {base}"));
        }
        Ok(Self {
            jwks_url: format!("{base}/auth/v1/.well-known/jwks.json"),
            issuer: format!("{base}/auth/v1"),
            legacy_hs256_secret: None,
            refresh_interval: Duration::from_secs(600),
            unknown_kid_cooldown: Duration::from_secs(10),
            http_timeout: Duration::from_secs(10),
        })
    }

    /// Variables: `SUPABASE_URL` (obligatoria), `SUPABASE_JWT_SECRET` (opcional, fallback HS256),
    /// `JWKS_REFRESH_SECS` (opcional, por defecto 600).
    pub fn from_env() -> Result<Self, String> {
        let url = std::env::var("SUPABASE_URL")
            .map_err(|_| "falta la variable de entorno SUPABASE_URL".to_string())?;
        let mut cfg = Self::for_project(&url)?;

        cfg.legacy_hs256_secret = std::env::var("SUPABASE_JWT_SECRET")
            .ok()
            .filter(|s| !s.trim().is_empty());

        if let Ok(v) = std::env::var("JWKS_REFRESH_SECS") {
            let secs: u64 = v
                .parse()
                .map_err(|_| format!("JWKS_REFRESH_SECS inválido: {v}"))?;
            cfg.refresh_interval = Duration::from_secs(secs.max(60));
        }
        Ok(cfg)
    }
}

#[derive(Debug)]
pub enum AuthInitError {
    Http(reqwest::Error),
    JwksSinClaves,
}

impl std::fmt::Display for AuthInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http(e) => write!(f, "no se pudo descargar el JWKS de Supabase: {e}"),
            Self::JwksSinClaves => write!(f, "el JWKS de Supabase no contiene claves utilizables"),
        }
    }
}

impl std::error::Error for AuthInitError {}

/// Motivo de rechazo (solo para logs; al cliente siempre se le responde 401 genérico).
#[derive(Debug, PartialEq, Eq)]
pub enum RechazoJwt {
    HeaderInvalido,
    SinKid,
    KidDesconocido,
    AlgoritmoNoPermitido(Algorithm),
    Hs256Deshabilitado,
    Invalido(jsonwebtoken::errors::ErrorKind),
}

struct ClaveJwks {
    key: DecodingKey,
    alg: Algorithm,
}

struct Cache {
    claves: HashMap<String, Arc<ClaveJwks>>,
    actualizado: Instant,
}

struct Inner {
    cfg: JwtConfig,
    http: reqwest::Client,
    cache: RwLock<Cache>,
    /// Serializa los refrescos (evita varias descargas simultáneas).
    refresco: Mutex<()>,
    legacy: Option<DecodingKey>,
}

#[derive(Clone)]
pub struct JwtVerifier {
    inner: Arc<Inner>,
}

impl JwtVerifier {
    /// Crea el verificador descargando el JWKS. Falla si no se puede obtener al arrancar:
    /// sin claves no se puede autenticar a nadie.
    pub async fn new(cfg: JwtConfig) -> Result<Self, AuthInitError> {
        let http = reqwest::Client::builder()
            .timeout(cfg.http_timeout)
            .build()
            .map_err(AuthInitError::Http)?;

        let claves = descargar_jwks(&http, &cfg.jwks_url)
            .await
            .map_err(AuthInitError::Http)?;
        if claves.is_empty() {
            return Err(AuthInitError::JwksSinClaves);
        }
        tracing::info!(kids = ?claves.keys().collect::<Vec<_>>(), "JWKS de Supabase cargado");

        let legacy = cfg.legacy_hs256_secret.as_deref().map(|s| {
            tracing::warn!(
                "fallback HS256 HABILITADO (SUPABASE_JWT_SECRET). Quitarlo cuando no queden tokens HS256 vigentes"
            );
            DecodingKey::from_secret(s.as_bytes())
        });

        Ok(Self {
            inner: Arc::new(Inner {
                cfg,
                http,
                cache: RwLock::new(Cache {
                    claves,
                    actualizado: Instant::now(),
                }),
                refresco: Mutex::new(()),
                legacy,
            }),
        })
    }

    pub async fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self::new(JwtConfig::from_env()?).await?)
    }

    /// Tarea en segundo plano que refresca el JWKS periódicamente. Si falla, se conservan
    /// las claves anteriores y se reintenta en el siguiente ciclo.
    pub fn spawn_refresco_periodico(&self) -> tokio::task::JoinHandle<()> {
        let this = self.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(this.inner.cfg.refresh_interval);
            tick.tick().await; // el primero es inmediato; ya se cargó en `new`
            loop {
                tick.tick().await;
                this.refrescar().await;
            }
        })
    }

    async fn refrescar(&self) {
        let _guard = self.inner.refresco.lock().await;
        match descargar_jwks(&self.inner.http, &self.inner.cfg.jwks_url).await {
            Ok(claves) if !claves.is_empty() => {
                let mut cache = self.inner.cache.write().await;
                cache.claves = claves;
                cache.actualizado = Instant::now();
                tracing::debug!(kids = ?cache.claves.keys().collect::<Vec<_>>(), "JWKS refrescado");
            }
            Ok(_) => tracing::warn!("JWKS sin claves utilizables; se conservan las anteriores"),
            Err(e) => {
                tracing::warn!(error = %e, "no se pudo refrescar el JWKS; se conservan las claves anteriores")
            }
        }
    }

    async fn clave(&self, kid: &str) -> Option<Arc<ClaveJwks>> {
        {
            let cache = self.inner.cache.read().await;
            if let Some(c) = cache.claves.get(kid) {
                return Some(c.clone());
            }
            if cache.actualizado.elapsed() < self.inner.cfg.unknown_kid_cooldown {
                return None;
            }
        }
        // `kid` desconocido y pasó el cooldown: puede ser una rotación de clave.
        let antes = self.inner.cache.read().await.actualizado;
        let _guard = self.inner.refresco.lock().await;
        // Otro request pudo haber refrescado mientras esperábamos el lock.
        if self.inner.cache.read().await.actualizado == antes {
            drop(_guard);
            self.refrescar().await;
        }
        self.inner.cache.read().await.claves.get(kid).cloned()
    }

    fn validacion(&self, alg: Algorithm) -> Validation {
        let mut v = Validation::new(alg);
        v.set_audience(&[SUPABASE_AUDIENCE]);
        v.set_issuer(&[self.inner.cfg.issuer.as_str()]);
        v.set_required_spec_claims(&["exp", "sub", "aud", "iss"]);
        v
    }

    pub async fn verify(&self, token: &str) -> Result<Uuid, RechazoJwt> {
        let header = decode_header(token).map_err(|_| RechazoJwt::HeaderInvalido)?;

        let (key_holder, alg): (KeyRef, Algorithm) = if header.alg == Algorithm::HS256 {
            if self.inner.legacy.is_none() {
                return Err(RechazoJwt::Hs256Deshabilitado);
            }
            (KeyRef::Legacy, Algorithm::HS256)
        } else {
            let kid = header.kid.as_deref().ok_or(RechazoJwt::SinKid)?;
            let clave = self.clave(kid).await.ok_or(RechazoJwt::KidDesconocido)?;
            // El algoritmo lo define la clave; el header debe coincidir.
            if header.alg != clave.alg {
                return Err(RechazoJwt::AlgoritmoNoPermitido(header.alg));
            }
            let alg = clave.alg;
            (KeyRef::Jwks(clave), alg)
        };

        let key = match &key_holder {
            KeyRef::Legacy => self.inner.legacy.as_ref().expect("verificado arriba"),
            KeyRef::Jwks(c) => &c.key,
        };

        let data = decode::<Claims>(token, key, &self.validacion(alg))
            .map_err(|e| RechazoJwt::Invalido(e.into_kind()))?;

        if alg == Algorithm::HS256 {
            tracing::warn!(user_id = %data.claims.sub, "token HS256 legacy aceptado");
        }
        Ok(data.claims.sub)
    }
}

enum KeyRef {
    Legacy,
    Jwks(Arc<ClaveJwks>),
}

async fn descargar_jwks(
    http: &reqwest::Client,
    url: &str,
) -> Result<HashMap<String, Arc<ClaveJwks>>, reqwest::Error> {
    let set: JwkSet = http
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(claves_utilizables(&set))
}

/// Convierte el JWKS en un mapa `kid → clave`, descartando (con log) las que no sirven.
fn claves_utilizables(set: &JwkSet) -> HashMap<String, Arc<ClaveJwks>> {
    set.keys
        .iter()
        .filter_map(|jwk| {
            let kid = jwk.common.key_id.clone()?;
            match clave_desde_jwk(jwk) {
                Ok(c) => Some((kid, Arc::new(c))),
                Err(motivo) => {
                    tracing::warn!(kid, motivo, "clave del JWKS ignorada");
                    None
                }
            }
        })
        .collect()
}

fn clave_desde_jwk(jwk: &Jwk) -> Result<ClaveJwks, String> {
    let alg = match jwk.common.key_algorithm {
        Some(ka) => Algorithm::try_from(ka).map_err(|_| format!("algoritmo no soportado: {ka}"))?,
        // Sin "alg" explícito se infiere de la curva / tipo.
        None => match &jwk.algorithm {
            AlgorithmParameters::EllipticCurve(p) => match p.curve {
                EllipticCurve::P256 => Algorithm::ES256,
                EllipticCurve::P384 => Algorithm::ES384,
                ref c => return Err(format!("curva no soportada: {c:?}")),
            },
            AlgorithmParameters::RSA(_) => Algorithm::RS256,
            AlgorithmParameters::OctetKeyPair(_) => Algorithm::EdDSA,
            AlgorithmParameters::OctetKey(_) => {
                return Err("clave simétrica en el JWKS (no permitida)".into());
            }
            _ => return Err("tipo de clave desconocido".into()),
        },
    };
    if !ALGORITMOS_JWKS.contains(&alg) {
        return Err(format!("algoritmo no permitido: {alg:?}"));
    }
    let key = DecodingKey::from_jwk(jwk).map_err(|e| format!("JWK inválido: {e}"))?;
    Ok(ClaveJwks { key, alg })
}

#[derive(Debug, Deserialize)]
struct Claims {
    sub: Uuid,
}

/// Usuario autenticado, inyectado como extensión del request por [`require_auth`].
/// Se puede usar directamente como extractor en los handlers.
#[derive(Debug, Clone, Copy)]
pub struct AuthUser {
    pub user_id: Uuid,
}

impl<S: Send + Sync> FromRequestParts<S> for AuthUser {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Si falta la extensión, la ruta no pasó por el middleware: se trata como no autenticado.
        parts
            .extensions
            .get::<AuthUser>()
            .copied()
            .ok_or_else(unauthorized)
    }
}

/// Middleware: exige `Authorization: Bearer <jwt>` válido; si no, responde 401.
pub async fn require_auth(
    State(verifier): State<JwtVerifier>,
    mut req: Request,
    next: Next,
) -> Response {
    let Some(token) = bearer_token(&req) else {
        return unauthorized();
    };

    match verifier.verify(token).await {
        Ok(user_id) => {
            req.extensions_mut().insert(AuthUser { user_id });
            next.run(req).await
        }
        Err(motivo) => {
            // Se registra solo el motivo, nunca el token.
            tracing::debug!(?motivo, "JWT rechazado");
            unauthorized()
        }
    }
}

fn bearer_token(req: &Request) -> Option<&str> {
    let value = req.headers().get(header::AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    let token = token.trim();
    (scheme.eq_ignore_ascii_case("bearer") && !token.is_empty()).then_some(token)
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [(header::WWW_AUTHENTICATE, "Bearer")],
        Json(json!({ "error": "unauthorized" })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, body::Body, middleware, routing::get};
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use http_body_util::BodyExt;
    use jsonwebtoken::{EncodingKey, Header, encode};
    use p256::{SecretKey, elliptic_curve::sec1::ToEncodedPoint, pkcs8::EncodePrivateKey};
    use serde::Serialize;
    use std::sync::Mutex as StdMutex;
    use tower::ServiceExt;

    const USER: &str = "6f1c2a3b-4d5e-4f60-8a7b-9c0d1e2f3a4b";

    /// Par de claves ES256 generado en cada ejecución (no hay claves privadas en el repo).
    struct ParEc {
        kid: String,
        encoding: EncodingKey,
        jwk: serde_json::Value,
    }

    fn par_ec(kid: &str) -> ParEc {
        // 32 bytes aleatorios a partir de dos UUID v4 (suficiente para claves de test).
        let mut bytes = [0u8; 32];
        bytes[..16].copy_from_slice(Uuid::new_v4().as_bytes());
        bytes[16..].copy_from_slice(Uuid::new_v4().as_bytes());
        let sk = SecretKey::from_slice(&bytes).expect("escalar válido");
        let pem = sk.to_pkcs8_pem(Default::default()).unwrap();
        let punto = sk.public_key().to_encoded_point(false);
        ParEc {
            kid: kid.into(),
            encoding: EncodingKey::from_ec_pem(pem.as_bytes()).unwrap(),
            jwk: json!({
                "kty": "EC", "crv": "P-256", "alg": "ES256", "use": "sig",
                "kid": kid,
                "x": URL_SAFE_NO_PAD.encode(punto.x().unwrap()),
                "y": URL_SAFE_NO_PAD.encode(punto.y().unwrap()),
            }),
        }
    }

    /// Servidor JWKS local; las claves publicadas se pueden cambiar en caliente (rotación).
    struct JwksServer {
        url: String,
        claves: Arc<StdMutex<Vec<serde_json::Value>>>,
        descargas: Arc<StdMutex<u32>>,
    }

    async fn jwks_server(claves: Vec<serde_json::Value>) -> JwksServer {
        let claves = Arc::new(StdMutex::new(claves));
        let descargas = Arc::new(StdMutex::new(0u32));
        let (c, d) = (claves.clone(), descargas.clone());
        let app = Router::new().route(
            "/auth/v1/.well-known/jwks.json",
            get(move || {
                let (c, d) = (c.clone(), d.clone());
                async move {
                    *d.lock().unwrap() += 1;
                    Json(json!({ "keys": *c.lock().unwrap() }))
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        JwksServer {
            url: format!("http://{addr}"),
            claves,
            descargas,
        }
    }

    impl JwksServer {
        fn config(&self) -> JwtConfig {
            let mut cfg = JwtConfig::for_project(&self.url).unwrap();
            cfg.unknown_kid_cooldown = Duration::ZERO;
            cfg
        }
        fn issuer(&self) -> String {
            format!("{}/auth/v1", self.url)
        }
        fn descargas(&self) -> u32 {
            *self.descargas.lock().unwrap()
        }
    }

    #[derive(Serialize)]
    struct TestClaims<'a> {
        sub: &'a str,
        aud: &'a str,
        iss: &'a str,
        exp: u64,
    }

    fn now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn token_es256(par: &ParEc, iss: &str, aud: &str, exp: u64) -> String {
        let mut h = Header::new(Algorithm::ES256);
        h.kid = Some(par.kid.clone());
        encode(
            &h,
            &TestClaims {
                sub: USER,
                aud,
                iss,
                exp,
            },
            &par.encoding,
        )
        .unwrap()
    }

    fn token_hs256(secret: &str, iss: &str) -> String {
        encode(
            &Header::new(Algorithm::HS256),
            &TestClaims {
                sub: USER,
                aud: "authenticated",
                iss,
                exp: now() + 3600,
            },
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap()
    }

    // ---------- verificación ----------

    #[tokio::test]
    async fn es256_valido() {
        let par = par_ec("k1");
        let srv = jwks_server(vec![par.jwk.clone()]).await;
        let v = JwtVerifier::new(srv.config()).await.unwrap();
        let t = token_es256(&par, &srv.issuer(), "authenticated", now() + 3600);
        assert_eq!(v.verify(&t).await, Ok(Uuid::parse_str(USER).unwrap()));
    }

    #[tokio::test]
    async fn rechazos_es256() {
        let par = par_ec("k1");
        let otra = par_ec("k1"); // mismo kid, otra clave privada → firma inválida
        let srv = jwks_server(vec![par.jwk.clone()]).await;
        let v = JwtVerifier::new(srv.config()).await.unwrap();
        let iss = srv.issuer();

        let casos = [
            (
                "firma de otra clave",
                token_es256(&otra, &iss, "authenticated", now() + 3600),
            ),
            (
                "expirado",
                token_es256(&par, &iss, "authenticated", now() - 3600),
            ),
            ("aud anon", token_es256(&par, &iss, "anon", now() + 3600)),
            (
                "otro issuer",
                token_es256(
                    &par,
                    "https://otro.supabase.co/auth/v1",
                    "authenticated",
                    now() + 3600,
                ),
            ),
        ];
        for (nombre, t) in casos {
            assert!(
                matches!(v.verify(&t).await, Err(RechazoJwt::Invalido(_))),
                "{nombre}"
            );
        }
    }

    #[tokio::test]
    async fn kid_ausente_o_desconocido() {
        let par = par_ec("k1");
        let srv = jwks_server(vec![par.jwk.clone()]).await;
        let v = JwtVerifier::new(srv.config()).await.unwrap();

        let mut sin_kid = Header::new(Algorithm::ES256);
        sin_kid.kid = None;
        let t = encode(
            &sin_kid,
            &TestClaims {
                sub: USER,
                aud: "authenticated",
                iss: &srv.issuer(),
                exp: now() + 3600,
            },
            &par.encoding,
        )
        .unwrap();
        assert_eq!(v.verify(&t).await, Err(RechazoJwt::SinKid));

        let desconocida = par_ec("k-no-publicada");
        let t = token_es256(&desconocida, &srv.issuer(), "authenticated", now() + 3600);
        assert_eq!(v.verify(&t).await, Err(RechazoJwt::KidDesconocido));
    }

    #[tokio::test]
    async fn hs256_rechazado_por_defecto() {
        let srv = jwks_server(vec![par_ec("k1").jwk]).await;
        let v = JwtVerifier::new(srv.config()).await.unwrap();
        let t = token_hs256("secreto-viejo", &srv.issuer());
        assert_eq!(v.verify(&t).await, Err(RechazoJwt::Hs256Deshabilitado));
    }

    #[tokio::test]
    async fn hs256_fallback_si_se_configura() {
        let srv = jwks_server(vec![par_ec("k1").jwk]).await;
        let mut cfg = srv.config();
        cfg.legacy_hs256_secret = Some("secreto-viejo".into());
        let v = JwtVerifier::new(cfg).await.unwrap();
        let iss = srv.issuer();
        assert!(v.verify(&token_hs256("secreto-viejo", &iss)).await.is_ok());
        assert!(matches!(
            v.verify(&token_hs256("otro-secreto", &iss)).await,
            Err(RechazoJwt::Invalido(_))
        ));
    }

    #[tokio::test]
    async fn header_alg_debe_coincidir_con_la_clave() {
        // Token con kid de una clave ES256 pero header alg ES384 → rechazado sin intentar decodificar.
        let par = par_ec("k1");
        let srv = jwks_server(vec![par.jwk.clone()]).await;
        let v = JwtVerifier::new(srv.config()).await.unwrap();
        let t = token_es256(&par, &srv.issuer(), "authenticated", now() + 3600);
        let mut partes: Vec<&str> = t.split('.').collect();
        let header_es384 = URL_SAFE_NO_PAD.encode(r#"{"alg":"ES384","typ":"JWT","kid":"k1"}"#);
        partes[0] = &header_es384;
        assert_eq!(
            v.verify(&partes.join(".")).await,
            Err(RechazoJwt::AlgoritmoNoPermitido(Algorithm::ES384))
        );
    }

    // ---------- JWKS: carga, rotación, cooldown ----------

    #[tokio::test]
    async fn rotacion_de_clave_se_detecta_por_kid_desconocido() {
        let vieja = par_ec("vieja");
        let nueva = par_ec("nueva");
        let srv = jwks_server(vec![vieja.jwk.clone()]).await;
        let v = JwtVerifier::new(srv.config()).await.unwrap();
        assert_eq!(srv.descargas(), 1);

        // Supabase rota: publica la nueva (y mantiene la vieja).
        *srv.claves.lock().unwrap() = vec![nueva.jwk.clone(), vieja.jwk.clone()];
        let t = token_es256(&nueva, &srv.issuer(), "authenticated", now() + 3600);
        assert!(v.verify(&t).await.is_ok());
        assert_eq!(
            srv.descargas(),
            2,
            "debe refrescar una vez al ver el kid nuevo"
        );

        // Kid ya conocido: no vuelve a descargar.
        assert!(v.verify(&t).await.is_ok());
        assert_eq!(srv.descargas(), 2);
    }

    #[tokio::test]
    async fn cooldown_evita_descargas_por_kids_inventados() {
        let srv = jwks_server(vec![par_ec("k1").jwk]).await;
        let mut cfg = srv.config();
        cfg.unknown_kid_cooldown = Duration::from_secs(3600);
        let v = JwtVerifier::new(cfg).await.unwrap();
        for i in 0..20 {
            let falsa = par_ec(&format!("falso-{i}"));
            let t = token_es256(&falsa, &srv.issuer(), "authenticated", now() + 3600);
            assert_eq!(v.verify(&t).await, Err(RechazoJwt::KidDesconocido));
        }
        assert_eq!(
            srv.descargas(),
            1,
            "dentro del cooldown no se descarga de nuevo"
        );
    }

    #[tokio::test]
    async fn arranque_falla_sin_jwks() {
        let srv = jwks_server(vec![]).await;
        assert!(matches!(
            JwtVerifier::new(srv.config()).await,
            Err(AuthInitError::JwksSinClaves)
        ));
        let mut cfg = srv.config();
        cfg.jwks_url = format!("{}/no-existe", srv.url);
        assert!(matches!(
            JwtVerifier::new(cfg).await,
            Err(AuthInitError::Http(_))
        ));
    }

    #[test]
    fn jwks_descarta_claves_simetricas_y_sin_kid() {
        let set: JwkSet = serde_json::from_value(json!({ "keys": [
            par_ec("ok").jwk,
            { "kty": "oct", "kid": "sim", "k": "c2VjcmV0", "alg": "HS256" },
            { "kty": "EC", "crv": "P-256", "x": par_ec("x").jwk["x"], "y": par_ec("x").jwk["y"] },
        ]}))
        .unwrap();
        let claves = claves_utilizables(&set);
        assert_eq!(claves.keys().collect::<Vec<_>>(), ["ok"]);
    }

    #[test]
    fn config_exige_https_salvo_local() {
        assert!(JwtConfig::for_project("http://evil.com").is_err());
        let c = JwtConfig::for_project("https://abc.supabase.co/").unwrap();
        assert_eq!(
            c.jwks_url,
            "https://abc.supabase.co/auth/v1/.well-known/jwks.json"
        );
        assert_eq!(c.issuer, "https://abc.supabase.co/auth/v1");
    }

    /// Descarga real por HTTPS (TLS con rustls). Usa `SUPABASE_URL`, o `JWKS_TEST_URL` para
    /// probar contra cualquier JWKS público:
    ///   SUPABASE_URL=https://<ref>.supabase.co cargo test -p api -- --ignored jwks_real
    #[ignore = "requiere red y SUPABASE_URL o JWKS_TEST_URL"]
    #[tokio::test]
    async fn jwks_real_por_https() {
        let cfg = match std::env::var("JWKS_TEST_URL") {
            Ok(url) => {
                let mut c = JwtConfig::for_project("https://example.invalid").unwrap();
                c.jwks_url = url;
                c
            }
            Err(_) => JwtConfig::from_env().expect("definir SUPABASE_URL o JWKS_TEST_URL"),
        };
        let v = JwtVerifier::new(cfg)
            .await
            .expect("JWKS descargado y con claves utilizables");
        let cache = v.inner.cache.read().await;
        println!("kids: {:?}", cache.claves.keys().collect::<Vec<_>>());
        println!(
            "algs: {:?}",
            cache.claves.values().map(|c| c.alg).collect::<Vec<_>>()
        );
    }

    // ---------- middleware ----------

    async fn llamar(v: JwtVerifier, auth: Option<String>) -> (StatusCode, String) {
        let app = Router::new()
            .route(
                "/me",
                get(|u: AuthUser| async move { u.user_id.to_string() }),
            )
            .layer(middleware::from_fn_with_state(v, require_auth));
        let mut req = Request::builder().uri("/me");
        if let Some(a) = auth {
            req = req.header(header::AUTHORIZATION, a);
        }
        let res = app.oneshot(req.body(Body::empty()).unwrap()).await.unwrap();
        let status = res.status();
        let body = res.into_body().collect().await.unwrap().to_bytes();
        (status, String::from_utf8(body.to_vec()).unwrap())
    }

    #[tokio::test]
    async fn middleware_inyecta_user_o_401() {
        let par = par_ec("k1");
        let srv = jwks_server(vec![par.jwk.clone()]).await;
        let v = JwtVerifier::new(srv.config()).await.unwrap();
        let t = token_es256(&par, &srv.issuer(), "authenticated", now() + 3600);

        assert_eq!(
            llamar(v.clone(), Some(format!("Bearer {t}"))).await,
            (StatusCode::OK, USER.into())
        );
        assert_eq!(llamar(v.clone(), None).await.0, StatusCode::UNAUTHORIZED);
        assert_eq!(
            llamar(v.clone(), Some(format!("Basic {t}"))).await.0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            llamar(v, Some("Bearer abc.def.ghi".into())).await.0,
            StatusCode::UNAUTHORIZED
        );
    }
}
