//! Encabezados de seguridad para todas las respuestas de la API (JSON).

use axum::{
    Router,
    http::{HeaderName, HeaderValue, header},
};
use tower_http::set_header::SetResponseHeaderLayer;

fn h(nombre: HeaderName, valor: &'static str) -> SetResponseHeaderLayer<HeaderValue> {
    SetResponseHeaderLayer::if_not_present(nombre, HeaderValue::from_static(valor))
}

/// `HSTS=true` agrega Strict-Transport-Security (solo cuando se sirve por HTTPS).
pub fn aplicar<S: Clone + Send + Sync + 'static>(router: Router<S>, hsts: bool) -> Router<S> {
    let r = router
        .layer(h(header::X_CONTENT_TYPE_OPTIONS, "nosniff"))
        .layer(h(header::X_FRAME_OPTIONS, "DENY"))
        .layer(h(header::REFERRER_POLICY, "no-referrer"))
        .layer(h(header::CACHE_CONTROL, "no-store"))
        .layer(h(
            header::CONTENT_SECURITY_POLICY,
            "default-src 'none'; frame-ancestors 'none'",
        ));
    if hsts {
        r.layer(h(
            header::STRICT_TRANSPORT_SECURITY,
            "max-age=31536000; includeSubDomains",
        ))
    } else {
        r
    }
}
