mod api;
pub mod auth;
pub mod config;
mod db;
mod relay_bridge;
mod relay_ops;
mod repl;
mod security_headers;
mod tasks;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use axum::{http::HeaderValue, http::header, middleware, Router};
use config::Config;
use sqlx::postgres::PgPoolOptions;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub config: Arc<Config>,
    pub relay_store: Option<Arc<komun_relay::storage::PersistentStore>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "komun_server=info,tower_http=info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    dotenvy::dotenv().ok();

    let config = Config::load()
        .context("Failed to load configuration. Copy config.example.toml to config.toml and edit it, or set KOMUN_CONFIG to a custom path.")?;

    std::env::set_var("JWT_SECRET", &config.auth.jwt_secret);

    if config.auth.jwt_secret.len() < 32 {
        anyhow::bail!(
            "JWT_SECRET is too short (< 32 chars). Set it in config.toml [auth] jwt_secret or via the JWT_SECRET environment variable. Generate with: openssl rand -base64 48"
        );
    }

    let pool = PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .connect(&config.database.url)
        .await
        .with_context(|| format!(
            "Failed to connect to PostgreSQL at {}. Is PostgreSQL running? Check config.toml [database] url or DATABASE_URL env var.",
            config.database.url
        ))?;

    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .context("Failed to run database migrations. Is the migrations/ directory present and accessible from the working directory?")?;

    let relay_store: Option<Arc<komun_relay::storage::PersistentStore>> = if config.relay.enabled {
        let storage_path = PathBuf::from(&config.relay.storage_path);
        std::fs::create_dir_all(&storage_path)
            .with_context(|| format!("Failed to create relay storage directory: {}", storage_path.display()))?;
        let snapshot_path = storage_path.join("community_data.json");
        let store = Arc::new(
            komun_relay::storage::PersistentStore::new(
                Some(snapshot_path),
                10000,
            )
        );
        let relay_config = config.relay.clone();
        tokio::spawn(relay_bridge::spawn_relay(relay_config, store.clone()));
        Some(store)
    } else {
        None
    };

    let state = AppState {
        pool: pool.clone(),
        config: Arc::new(config.clone()),
        relay_store,
    };

    tasks::spawn_background_tasks(state.clone());

    let allowed_origins = &state.config.security.allowed_origins;
    let cors_headers = [header::AUTHORIZATION, header::CONTENT_TYPE];
    let cors = if allowed_origins == "*" {
        CorsLayer::new()
            .allow_origin(AllowOrigin::any())
            .allow_methods(tower_http::cors::Any)
            .allow_headers(cors_headers)
    } else {
        let origins: Vec<HeaderValue> = allowed_origins
            .split(',')
            .filter_map(|s| {
                let trimmed = s.trim();
                if trimmed.is_empty() { None }
                else { HeaderValue::from_str(trimmed).ok() }
            })
            .collect();
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(origins))
            .allow_methods(tower_http::cors::Any)
            .allow_headers(cors_headers)
    };

    let app = Router::new()
        .nest("/api", api::router(state.clone()))
        .layer(cors)
        .layer(middleware::from_fn(security_headers::security_headers))
        .layer(TraceLayer::new_for_http());

    let bind = config.bind_addr();
    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .with_context(|| format!(
            "Failed to bind to {}. Is another process using port {}? Check config.toml [server] port or KOMUN_PORT env var.",
            bind, config.server.port
        ))?;

    tracing::info!("komun listening on http://{}", bind);

    let server = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!("Server error: {}", e);
        }
    });

    if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        repl::run_repl(state).await;
    } else {
        server.await
            .context("Server task panicked")?;
    }

    Ok(())
}
