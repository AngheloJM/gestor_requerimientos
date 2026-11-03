mod almanaque;
mod aprobaciones;
mod auth;
mod cli;
mod cors;
mod dto;
mod error;
mod limitador;
mod matriz;
mod me;
mod passwords;
mod requerimientos;
mod seguridad;
mod sesion;
mod state;
mod tablero;

use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::StatusCode,
    middleware,
    routing::{get, patch, post},
};
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use auth::TokenService;
use limitador::Limitador;
use state::{AppState, CookieConfig};

/// Ventana del limitador de intentos fallidos.
const VENTANA_INTENTOS: Duration = Duration::from_secs(15 * 60);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Carga variables desde .env si existe (solo desarrollo; en producción usar variables reales).
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "api=info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args: Vec<String> = std::env::args().collect();
    if cli::es_comando(&args) {
        return cli::ejecutar(&args).await;
    }

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
    passwords::precalentar();

    let state = AppState {
        db,
        tokens: TokenService::from_env()?,
        cookies: CookieConfig::from_env()?,
        limite_cuenta: Arc::new(Limitador::new(VENTANA_INTENTOS, 5)),
        limite_ip: Arc::new(Limitador::new(VENTANA_INTENTOS, 20)),
    };
    let hsts = std::env::var("HSTS").is_ok_and(|v| v == "true");

    let app = Router::new()
        .route("/health", get(health))
        .route("/auth/login", post(sesion::login))
        .route("/auth/refresh", post(sesion::refresh))
        .route("/auth/logout", post(sesion::logout))
        .merge(protected_routes(state.clone()))
        .with_state(state)
        .layer(DefaultBodyLimit::max(64 * 1024));
    let app = seguridad::aplicar(app, hsts)
        // CORS por fuera de todo: los preflight (OPTIONS) no llevan token.
        .layer(cors::desde_env()?);

    let addr = std::env::var("API_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!("API escuchando en {addr}");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    Ok(())
}

/// Rutas que requieren access token. Rust autentica y propaga la identidad;
/// **la autorización la aplican las políticas RLS de Postgres** en cada consulta.
fn protected_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/me", get(me::get_me))
        .route("/auth/cambiar-password", post(sesion::cambiar_password))
        .route("/matriz-servicio", get(matriz::get_matriz_servicio))
        .route("/almanaque", get(almanaque::get_almanaque))
        .route("/tablero", get(tablero::get_tablero))
        .route("/requerimientos", get(requerimientos::listar))
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
