use axum::{
    extract::{Extension, Multipart, Path, Query, State},
    http::StatusCode,
    middleware,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

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
        .route("/{id}/images", axum::routing::post(upload_images))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

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

async fn upload_images(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path((_slug, id)): Path<(String, Uuid)>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, StatusError> {
    let post = crate::db::posts::get(&state.pool, id).await?;
    if post.author_id != auth.user_id {
        return Err(anyhow::anyhow!("not your post").into());
    }

    let current_count: i64 = sqlx::query_scalar(
        "SELECT COALESCE(array_length(images, 1), 0) FROM posts WHERE id = $1"
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    let max = state.config.media.max_post_images as i64;
    let mut filenames: Vec<String> = vec![];

    while let Some(field) = multipart.next_field().await.map_err(|e| anyhow::anyhow!("{}", e))? {
        if current_count + filenames.len() as i64 >= max {
            break;
        }

        let content_type = field.content_type().unwrap_or("").to_string();
        if !matches!(content_type.as_str(), "image/png" | "image/jpeg" | "image/webp") {
            continue;
        }

        let data = field.bytes().await.map_err(|e| anyhow::anyhow!("{}", e))?;
        if data.len() > state.config.media.max_post_image_bytes as usize {
            continue;
        }

        let img = image::load_from_memory(&data)
            .map_err(|_| anyhow::anyhow!("invalid image"))?;
        let img = if img.width() > 1920 || img.height() > 1920 {
            img.resize(1920, 1920, image::imageops::FilterType::Lanczos3)
        } else {
            img
        };

        let img_id = Uuid::now_v7();
        let filename = format!("{}.webp", img_id);
        let dir = std::path::Path::new(&state.config.media.post_images_dir);
        std::fs::create_dir_all(dir).ok();
        let path = dir.join(&filename);
        img.save(&path).map_err(|e| anyhow::anyhow!("{}", e))?;
        filenames.push(filename);
    }

    if filenames.is_empty() {
        return Ok(Json(json!({"images": []})));
    }

    sqlx::query("UPDATE posts SET images = array_cat(COALESCE(images, '{}'), $1::text[]) WHERE id = $2")
        .bind(&filenames)
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let urls: Vec<String> = filenames.iter()
        .map(|f| format!("/post-images/{}", f))
        .collect();

    Ok(Json(json!({"images": urls})))
}
