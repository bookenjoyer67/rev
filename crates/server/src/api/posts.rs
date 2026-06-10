use axum::{
    extract::{Extension, Path, Query, State},
    middleware,
    routing::get,
    Json, Router,
};
use serde::Deserialize;

use komun_core::models::{CreatePost, Post};
use crate::auth::{require_auth, AuthUser};
use crate::AppState;

use super::communities::StatusError;

pub fn router(state: AppState) -> Router {
    let public = Router::new()
        .route("/", get(list_posts))
        .route("/{id}", get(get_post));

    let protected = Router::new()
        .route("/", axum::routing::post(create_post))
        .route("/{id}", axum::routing::patch(update_post).delete(withdraw_post))
        .layer(middleware::from_fn(require_auth));

    public.merge(protected).with_state(state)
}

#[derive(Deserialize)]
struct PostFilters {
    kind: Option<String>,
    category: Option<String>,
    status: Option<String>,
}

async fn list_posts(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(filters): Query<PostFilters>,
) -> Result<Json<Vec<Post>>, StatusError> {
    let community = crate::db::communities::get_by_slug(&state.pool, &slug).await?;
    let posts = crate::db::posts::list(&state.pool, community.id, filters.kind, filters.category, filters.status).await?;
    Ok(Json(posts))
}

async fn get_post(
    State(state): State<AppState>,
    Path((_slug, id)): Path<(String, uuid::Uuid)>,
) -> Result<Json<Post>, StatusError> {
    let post = crate::db::posts::get(&state.pool, id).await?;
    Ok(Json(post))
}

async fn create_post(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(slug): Path<String>,
    Json(input): Json<CreatePost>,
) -> Result<Json<Post>, StatusError> {
    let community = crate::db::communities::get_by_slug(&state.pool, &slug).await?;
    let post = crate::db::posts::create(&state.pool, community.id, auth.user_id, input).await?;
    Ok(Json(post))
}

async fn update_post(
    State(_state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Path((_slug, _id)): Path<(String, uuid::Uuid)>,
) -> Result<Json<serde_json::Value>, StatusError> {
    Ok(Json(serde_json::json!({"status": "todo"})))
}

async fn withdraw_post(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Path((_slug, id)): Path<(String, uuid::Uuid)>,
) -> Result<Json<serde_json::Value>, StatusError> {
    crate::db::posts::withdraw(&state.pool, id).await?;
    Ok(Json(serde_json::json!({"status": "withdrawn"})))
}
