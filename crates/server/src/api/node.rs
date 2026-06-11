use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;

use crate::AppState;

#[derive(Serialize)]
struct NodeInfo {
    name: String,
    description: String,
    version: String,
    location: Option<NodeLocation>,
    communities_count: i64,
    listed: bool,
    federation_enabled: bool,
    relay_url: Option<String>,
}

#[derive(Serialize)]
struct NodeLocation {
    name: Option<String>,
    lat: Option<f64>,
    lon: Option<f64>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/node", get(get_node_info))
        .with_state(state)
}

async fn get_node_info(State(state): State<AppState>) -> Json<NodeInfo> {
    let communities_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM communities")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);

    let config = &state.config;
    let location = if config.node.location_name.is_some()
        || config.node.location_lat.is_some()
    {
        Some(NodeLocation {
            name: config.node.location_name.clone(),
            lat: config.node.location_lat,
            lon: config.node.location_lon,
        })
    } else {
        None
    };

    Json(NodeInfo {
        name: config.node.name.clone(),
        description: config.node.description.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        location,
        communities_count,
        listed: config.discovery.listed,
        federation_enabled: config.federation.enabled,
        relay_url: config.relay.external_url.clone(),
    })
}
