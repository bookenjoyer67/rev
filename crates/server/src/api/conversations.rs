use axum::{
    extract::{Extension, Path, State},
    middleware,
    routing::{get, patch, post},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::{require_auth, AuthUser};
use crate::AppState;
use super::communities::StatusError;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/posts/{post_id}/respond", post(respond_to_post))
        .route("/me/conversations", get(list_conversations))
        .route("/conversations/{match_id}", get(get_conversation))
        .route("/conversations/{match_id}/messages", post(send_message))
        .route("/conversations/{match_id}/status", patch(update_status))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth))
        .with_state(state)
}

#[derive(Deserialize)]
struct RespondRequest {
    message: String,
}

async fn respond_to_post(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(post_id): Path<Uuid>,
    Json(input): Json<RespondRequest>,
) -> Result<Json<serde_json::Value>, StatusError> {
    let recent: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM matches WHERE responder_id = $1 AND created_at > now() - interval '1 hour'"
    )
    .bind(auth.user_id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    if recent >= state.config.security.max_matches_per_hour as i64 {
        return Err(anyhow::anyhow!(
            "rate limit: max {} responses per hour",
            state.config.security.max_matches_per_hour
        )
        .into());
    }

    let (match_id, _message_id) = crate::db::conversations::create_match(
        &state.pool,
        post_id,
        auth.user_id,
        &input.message,
    )
    .await?;

    Ok(Json(serde_json::json!({
        "match_id": match_id,
        "status": "proposed"
    })))
}

async fn list_conversations(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Vec<crate::db::conversations::ConversationPreview>>, StatusError> {
    let convos = crate::db::conversations::list_conversations(&state.pool, auth.user_id).await?;
    Ok(Json(convos))
}

async fn get_conversation(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(match_id): Path<Uuid>,
) -> Result<Json<crate::db::conversations::Conversation>, StatusError> {
    let convo = crate::db::conversations::get_conversation(&state.pool, match_id, auth.user_id).await?;
    Ok(Json(convo))
}

#[derive(Deserialize)]
struct SendMessageRequest {
    body: String,
}

async fn send_message(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(match_id): Path<Uuid>,
    Json(input): Json<SendMessageRequest>,
) -> Result<Json<crate::db::conversations::MessageRow>, StatusError> {
    let recent: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM messages WHERE sender_id = $1 AND created_at > now() - interval '1 hour'"
    )
    .bind(auth.user_id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    if recent >= state.config.security.max_messages_per_hour as i64 {
        return Err(anyhow::anyhow!("rate limit: max {} messages per hour", state.config.security.max_messages_per_hour).into());
    }

    let msg = crate::db::conversations::send_message(
        &state.pool,
        match_id,
        auth.user_id,
        &input.body,
    )
    .await?;
    Ok(Json(msg))
}

#[derive(Deserialize)]
struct UpdateStatusRequest {
    status: String,
}

async fn update_status(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(match_id): Path<Uuid>,
    Json(input): Json<UpdateStatusRequest>,
) -> Result<Json<serde_json::Value>, StatusError> {
    let participant = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM matches m JOIN posts p ON p.id = m.post_id WHERE m.id = $1 AND ($2 = m.responder_id OR $2 = p.author_id))"
    )
    .bind(match_id)
    .bind(auth.user_id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(false);

    if !participant {
        return Ok(Json(serde_json::json!({"error": "not a participant"})));
    }

    crate::db::conversations::update_status(&state.pool, match_id, &input.status).await?;
    Ok(Json(serde_json::json!({"status": input.status})))
}
