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
    q: Option<String>,
}

async fn list_posts(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(filters): Query<PostFilters>,
) -> Result<Json<Vec<Post>>, StatusError> {
    let community = crate::db::communities::get_by_slug(&state.pool, &slug).await?;
    let posts = crate::db::posts::list(&state.pool, community.id, filters.kind, filters.category, filters.status, filters.q).await?;
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
    let recent: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM posts WHERE author_id = $1 AND created_at > now() - interval '1 hour'"
    )
    .bind(auth.user_id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    if recent >= state.config.security.max_posts_per_hour as i64 {
        return Err(anyhow::anyhow!("rate limit: max {} posts per hour", state.config.security.max_posts_per_hour).into());
    }

    let community = crate::db::communities::get_by_slug(&state.pool, &slug).await?;
    let post = crate::db::posts::create(&state.pool, community.id, auth.user_id, input).await?;
    Ok(Json(post))
}

#[derive(Deserialize)]
struct UpdatePostRequest {
    title: Option<String>,
    body: Option<String>,
    urgency: Option<String>,
    status: Option<String>,
}

async fn update_post(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path((_slug, id)): Path<(String, uuid::Uuid)>,
    Json(input): Json<UpdatePostRequest>,
) -> Result<Json<serde_json::Value>, StatusError> {
    let post = crate::db::posts::get(&state.pool, id).await?;
    if post.author_id != auth.user_id {
        return Ok(Json(serde_json::json!({"error": "not your post"})));
    }
    crate::db::posts::update(&state.pool, id, input.title, input.body, input.urgency, input.status).await?;
    Ok(Json(serde_json::json!({"status": "updated"})))
}

async fn withdraw_post(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path((_slug, id)): Path<(String, uuid::Uuid)>,
) -> Result<Json<serde_json::Value>, StatusError> {
    let post = crate::db::posts::get(&state.pool, id).await?;
    if post.author_id != auth.user_id {
        return Ok(Json(serde_json::json!({"error": "not your post"})));
    }
    crate::db::posts::withdraw(&state.pool, id).await?;
    Ok(Json(serde_json::json!({"status": "withdrawn"})))
}
