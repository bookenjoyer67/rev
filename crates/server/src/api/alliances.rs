use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    middleware,
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::AppState;
use crate::auth::{require_auth, AuthUser};
use crate::db::alliances;

#[derive(Deserialize)]
struct ProposeRequest {
    remote_domain: String,
    remote_name: Option<String>,
}

pub fn router(state: AppState) -> Router {
    let public = Router::new()
        .route("/alliances", get(list_alliances));

    let protected = Router::new()
        .route("/alliances", post(propose_alliance))
        .route("/alliances/{id}/accept", post(accept_alliance))
        .route("/alliances/{id}/reject", post(reject_alliance))
        .route("/alliances/{id}", delete(delete_alliance))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    public.merge(protected).with_state(state)
}

async fn list_alliances(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let list = alliances::list_alliances(&state.pool).await.map_err(|e| {
        tracing::error!("list alliances failed: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"})))
    })?;
    Ok(Json(serde_json::json!(list)))
}

async fn propose_alliance(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(input): Json<ProposeRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let domain = input.remote_domain.trim().to_lowercase();
    if domain.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "domain required"}))));
    }

    if let Ok(Some(existing)) = alliances::find_by_domain(&state.pool, &domain).await {
        return Err((StatusCode::CONFLICT, Json(serde_json::json!({
            "error": "alliance already exists",
            "id": existing.id,
            "status": existing.status,
        }))));
    }

    let node_url = format!("https://{}/api/node", domain);
    let remote_info: Option<serde_json::Value> = match reqwest::get(&node_url).await {
        Ok(resp) => resp.json().await.ok(),
        Err(_) => None,
    };

    let remote_name = input.remote_name.or_else(|| {
        remote_info.as_ref()
            .and_then(|v| v.get("name"))
            .and_then(|n| n.as_str())
            .map(String::from)
    });

    let alliance = alliances::create_alliance(
        &state.pool,
        &domain,
        remote_name.as_deref(),
        None,
        "outgoing",
    )
    .await
    .map_err(|e| {
        tracing::error!("create alliance failed: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"})))
    })?;

    if remote_info.is_some() {
        let our_domain = state.config.federation.domain.as_deref().unwrap_or("unknown");
        let _ = reqwest::Client::new()
            .post(format!("https://{}/api/alliances/propose", domain))
            .json(&serde_json::json!({
                "domain": our_domain,
                "name": state.config.node.name,
                "alliance_id": alliance.id,
            }))
            .send()
            .await;
    }

    Ok(Json(serde_json::json!({
        "id": alliance.id,
        "remote_domain": alliance.remote_domain,
        "remote_name": alliance.remote_name,
        "status": alliance.status,
        "created_at": alliance.created_at,
    })))
}

async fn accept_alliance(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let alliance = alliances::update_status(&state.pool, id, "accepted")
        .await
        .map_err(|e| {
            tracing::error!("accept alliance failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"})))
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "alliance not found"}))))?;

    let _ = reqwest::Client::new()
        .post(format!("https://{}/api/alliances/{}/accept", alliance.remote_domain, id))
        .send()
        .await;

    Ok(Json(serde_json::json!({"id": id, "status": "accepted"})))
}

async fn reject_alliance(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let _alliance = alliances::update_status(&state.pool, id, "rejected")
        .await
        .map_err(|e| {
            tracing::error!("reject alliance failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"})))
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "alliance not found"}))))?;

    Ok(Json(serde_json::json!({"id": id, "status": "rejected"})))
}

async fn delete_alliance(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let removed = alliances::delete_alliance(&state.pool, id)
        .await
        .map_err(|e| {
            tracing::error!("delete alliance failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"})))
        })?;

    if removed {
        Ok(Json(serde_json::json!({"ok": true})))
    } else {
        Err((StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "alliance not found"}))))
    }
}
