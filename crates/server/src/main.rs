mod api;
pub mod auth;
pub mod config;
mod db;
mod relay_bridge;
mod repl;
mod tasks;

use std::sync::Arc;

use axum::Router;
use config::Config;
use sqlx::postgres::PgPoolOptions;
use tower_http::{
    cors::CorsLayer,
    services::ServeDir,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub config: Arc<Config>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "komun_server=info,tower_http=info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    dotenvy::dotenv().ok();

    let config = Config::load().expect("failed to load configuration");

    std::env::set_var("JWT_SECRET", &config.auth.jwt_secret);

    let pool = PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .connect(&config.database.url)
        .await
        .expect("failed to connect to database");

    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");

    let state = AppState {
        pool: pool.clone(),
        config: Arc::new(config.clone()),
    };

    tasks::spawn_background_tasks(state.clone());

    if config.relay.enabled {
        let relay_config = config.relay.clone();
        tokio::spawn(relay_bridge::spawn_relay(relay_config));
    }

    let mut app = Router::new()
        .nest("/api", api::router(state.clone()))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    if let Some(ref static_dir) = config.server.static_dir {
        if !static_dir.is_empty() {
            app = app.fallback_service(ServeDir::new(static_dir));
        }
    }

    let bind = config.bind_addr();
    let listener = tokio::net::TcpListener::bind(&bind).await.unwrap();

    tracing::info!("komun listening on {}", bind);

    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        repl::run_repl(state).await;
    } else {
        server.await.unwrap();
    }
}
