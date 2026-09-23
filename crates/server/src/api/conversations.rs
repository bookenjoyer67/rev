use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    middleware,
    routing::{get, patch, post},
    Json, Router,
};
use base64::Engine;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::{require_auth, AuthUser};
use crate::AppState;
use super::StatusError;

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

/// A3.3: what arrives is a sealed box, not text. The server stores the bytes and never learns
/// what they say, so the only validation possible here is that they decode and are not empty.
#[derive(Deserialize)]
struct SealedMessage {
    ciphertext: String,
    nonce: Option<String>,
}

impl SealedMessage {
    fn decode(&self) -> Result<(Vec<u8>, Option<Vec<u8>>), StatusError> {
        let bad = |m: &str| StatusError::with_status(StatusCode::BAD_REQUEST, m.to_string());
        let engine = base64::engine::general_purpose::STANDARD;

        let ciphertext = engine
            .decode(&self.ciphertext)
            .map_err(|_| bad("ciphertext is not valid base64"))?;
        if ciphertext.is_empty() {
            return Err(bad("ciphertext is empty"));
        }

        let nonce = match self.nonce.as_deref() {
            None | Some("") => None,
            Some(n) => Some(
                engine
                    .decode(n)
                    .map_err(|_| bad("nonce is not valid base64"))?,
            ),
        };

        Ok((ciphertext, nonce))
    }
}

async fn respond_to_post(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(post_id): Path<Uuid>,
    Json(input): Json<SealedMessage>,
) -> Result<Json<serde_json::Value>, StatusError> {
    let recent: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM matches WHERE responder_id = $1 AND created_at > now() - interval '1 hour'"
    )
    .bind(auth.user_id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    if recent >= state.config.security.max_matches_per_hour as i64 {
        return Err(StatusError::with_status(
            StatusCode::TOO_MANY_REQUESTS,
            format!(
                "rate limit: max {} responses per hour",
                state.config.security.max_matches_per_hour
            ),
        ));
    }

    let (ciphertext, nonce) = input.decode()?;

    let (match_id, _message_id) = crate::db::conversations::create_match(
        &state.pool,
        post_id,
        auth.user_id,
        &ciphertext,
        nonce.as_deref(),
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

async fn send_message(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(match_id): Path<Uuid>,
    Json(input): Json<SealedMessage>,
) -> Result<Json<crate::db::conversations::MessageRow>, StatusError> {
    let recent: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM messages WHERE sender_id = $1 AND created_at > now() - interval '1 hour'"
    )
    .bind(auth.user_id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    if recent >= state.config.security.max_messages_per_hour as i64 {
        return Err(StatusError::with_status(
            StatusCode::TOO_MANY_REQUESTS,
            format!(
                "rate limit: max {} messages per hour",
                state.config.security.max_messages_per_hour
            ),
        ));
    }

    let (ciphertext, nonce) = input.decode()?;

    let msg = crate::db::conversations::send_message(
        &state.pool,
        match_id,
        auth.user_id,
        &ciphertext,
        nonce.as_deref(),
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
        // Was HTTP 200 with an `error` key in the body — the same shape A2b fixed on the admin
        // role route. A client checking the status code read this as success.
        return Err(StatusError::with_status(StatusCode::FORBIDDEN, "not a participant"));
    }

    if !matches!(input.status.as_str(), "proposed" | "accepted" | "completed" | "withdrawn") {
        return Err(StatusError::with_status(
            StatusCode::BAD_REQUEST,
            "status must be one of: proposed, accepted, completed, withdrawn",
        ));
    }

    crate::db::conversations::update_status(&state.pool, match_id, &input.status).await?;
    Ok(Json(serde_json::json!({"status": input.status})))
}
