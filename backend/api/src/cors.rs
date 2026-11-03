//! CORS: solo los orígenes listados en `CORS_ALLOWED_ORIGINS` (separados por coma).
//! Sin la variable, no se permite ningún origen cruzado (más seguro por defecto).

use std::time::Duration;

use axum::http::{HeaderName, HeaderValue, Method, header};
use tower_http::cors::{AllowOrigin, CorsLayer};

pub fn desde_env() -> Result<CorsLayer, String> {
    let raw = std::env::var("CORS_ALLOWED_ORIGINS").unwrap_or_default();
    construir(&raw)
}

pub fn construir(raw: &str) -> Result<CorsLayer, String> {
    let origenes = raw
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|o| {
            if o == "*" {
                return Err(
                    "CORS_ALLOWED_ORIGINS no admite \"*\": listar orígenes explícitos".into(),
                );
            }
            HeaderValue::from_str(o).map_err(|_| format!("origen CORS inválido: {o}"))
        })
        .collect::<Result<Vec<_>, String>>()?;

    if origenes.is_empty() {
        tracing::warn!(
            "CORS_ALLOWED_ORIGINS vacío: el navegador no podrá llamar a la API desde otro origen"
        );
    } else {
        tracing::info!(origenes = ?origenes, "CORS habilitado");
    }

    Ok(CorsLayer::new()
        .allow_origin(AllowOrigin::list(origenes))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::PUT,
            Method::DELETE,
        ])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
            HeaderName::from_static(crate::sesion::HEADER_CSRF),
        ])
        // Necesario para la cookie del refresh token entre orígenes distintos.
        .allow_credentials(true)
        .max_age(Duration::from_secs(600)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, body::Body, extract::Request, http::StatusCode, routing::get};
    use tower::ServiceExt;

    async fn preflight(origen: &str) -> (StatusCode, Option<String>) {
        let app = Router::new()
            .route("/x", get(|| async { "ok" }))
            .layer(construir("http://localhost:5173, https://app.example.com").unwrap());
        let req = Request::builder()
            .method(Method::OPTIONS)
            .uri("/x")
            .header(header::ORIGIN, origen)
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "PATCH")
            .header(
                header::ACCESS_CONTROL_REQUEST_HEADERS,
                "authorization,content-type",
            )
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        let acao = res
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .map(|v| v.to_str().unwrap().to_string());
        (res.status(), acao)
    }

    #[tokio::test]
    async fn origen_permitido_recibe_cabecera() {
        let (status, acao) = preflight("http://localhost:5173").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(acao.as_deref(), Some("http://localhost:5173"));
    }

    #[tokio::test]
    async fn origen_no_permitido_sin_cabecera() {
        assert_eq!(preflight("https://evil.example").await.1, None);
    }

    #[test]
    fn rechaza_comodin_y_origen_invalido() {
        assert!(construir("*").is_err());
        assert!(construir("http://ok.com, \n").is_ok());
        assert!(construir("http://bad\u{7f}").is_err());
    }
}
