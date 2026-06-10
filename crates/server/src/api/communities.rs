use axum::{
    extract::{Extension, Path, State},
    middleware,
    routing::{get, post},
    Json, Router,
};

use komun_core::models::{Community, CreateCommunity, Invite};
use crate::auth::{require_auth, AuthUser};
use crate::AppState;

pub fn router(state: AppState) -> Router {
    let public = Router::new()
        .route("/", get(list_communities))
        .route("/{slug}", get(get_community));

    let protected = Router::new()
        .route("/", post(create_community))
        .route("/{slug}/invite", post(create_invite))
        .route("/{slug}/join", post(join_community))
        .layer(middleware::from_fn(require_auth));

    public.merge(protected).with_state(state)
}

async fn list_communities(
    State(state): State<AppState>,
) -> Result<Json<Vec<Community>>, StatusError> {
    let communities = crate::db::communities::list(&state.pool).await?;
    Ok(Json(communities))
}

async fn get_community(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Community>, StatusError> {
    let community = crate::db::communities::get_by_slug(&state.pool, &slug).await?;
    Ok(Json(community))
}

async fn create_community(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(input): Json<CreateCommunity>,
) -> Result<Json<Community>, StatusError> {
    let community = crate::db::communities::create(&state.pool, input).await?;
    crate::db::communities::add_member(&state.pool, community.id, auth.user_id, "admin").await?;
    Ok(Json(community))
}

async fn create_invite(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(slug): Path<String>,
) -> Result<Json<Invite>, StatusError> {
    let community = crate::db::communities::get_by_slug(&state.pool, &slug).await?;
    let invite = crate::db::communities::create_invite(&state.pool, community.id, auth.user_id).await?;
    Ok(Json(invite))
}

async fn join_community(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(slug): Path<String>,
    Json(payload): Json<JoinPayload>,
) -> Result<Json<serde_json::Value>, StatusError> {
    let community = crate::db::communities::get_by_slug(&state.pool, &slug).await?;
    crate::db::communities::use_invite(&state.pool, &payload.code).await?;
    crate::db::communities::add_member(&state.pool, community.id, auth.user_id, "member").await?;
    Ok(Json(serde_json::json!({"status": "joined"})))
}

#[derive(serde::Deserialize)]
struct JoinPayload {
    code: String,
}

use axum::response::{IntoResponse, Response};
use axum::http::StatusCode;

pub struct StatusError(anyhow::Error);

impl<E: Into<anyhow::Error>> From<E> for StatusError {
    fn from(err: E) -> Self {
        StatusError(err.into())
    }
}

impl IntoResponse for StatusError {
    fn into_response(self) -> Response {
        tracing::error!("request error: {:?}", self.0);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": self.0.to_string()})),
        ).into_response()
    }
}
