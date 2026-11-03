mod almanaque;
mod aprobaciones;
mod auth;
mod cors;
mod dto;
mod error;
mod matriz;
mod me;
mod state;
mod tablero;

use std::time::Duration;

use axum::{
    Router,
    http::StatusCode,
    middleware,
    routing::{get, patch},
};
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use auth::JwtVerifier;
use state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Carga variables desde .env si existe (solo desarrollo; en producción usar variables reales).
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "api=info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url =
        std::env::var("DATABASE_URL").map_err(|_| "falta la variable de entorno DATABASE_URL")?;
    let db = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&database_url)
        .await?;

    // La autorización depende de poder asumir `authenticated` (RLS): verificarlo al arrancar.
    db::rls::verificar(&db).await?;
    tracing::info!("RLS: la conexión puede asumir el rol `authenticated`");

    let jwt = JwtVerifier::from_env().await?;
    jwt.spawn_refresco_periodico();

    let state = AppState { db, jwt };

    let app = Router::new()
        .route("/health", get(health))
        .merge(protected_routes(state.clone()))
        .with_state(state)
        // CORS por fuera de todo: los preflight (OPTIONS) no llevan token.
        .layer(cors::desde_env()?);

    let addr = std::env::var("API_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!("API escuchando en {addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

/// Rutas que requieren JWT válido. Rust solo autentica (JWT) y propaga la identidad;
/// **la autorización la aplican las políticas RLS de Postgres** en cada consulta.
fn protected_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/me", get(me::get_me))
        .route("/matriz-servicio", get(matriz::get_matriz_servicio))
        .route("/almanaque", get(almanaque::get_almanaque))
        .route("/tablero", get(tablero::get_tablero))
        .route("/aprobaciones", get(aprobaciones::listar))
        .route("/aprobaciones/{id}", patch(aprobaciones::editar_fecha_fin))
        .route("/aprobaciones/{id}/historial", get(aprobaciones::historial))
        .layer(middleware::from_fn_with_state(state, auth::require_auth))
}

async fn health() -> (StatusCode, &'static str) {
    (StatusCode::OK, "ok")
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("no se pudo instalar el handler de Ctrl+C");
    tracing::info!("Apagando servidor...");
}
