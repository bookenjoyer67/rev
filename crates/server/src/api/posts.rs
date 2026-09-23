//! `/api/posts` — the flat post collection.
//!
//! A3.1: posts used to hang off a per-tenant path segment, and every handler began by resolving
//! that segment to a row in a table the squashed schema no longer has. The path segment, the
//! lookup and the membership/role checks that gated posting are all gone. `require_auth` is the
//! only gate on the mutating half of this router.

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

use komun_core::models::{CreatePost, Post, PostStatus, Urgency};
use crate::auth::{require_auth, AuthUser};
use crate::AppState;

use super::StatusError;

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
    Query(filters): Query<PostFilters>,
) -> Result<Json<Vec<Post>>, StatusError> {
    let posts = crate::db::posts::list(
        &state.pool,
        filters.kind,
        filters.category,
        filters.status,
        filters.q,
    )
    .await?;
    Ok(Json(posts))
}

async fn get_post(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Post>, StatusError> {
    let post = load_post(&state, id).await?;
    Ok(Json(post))
}

async fn create_post(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
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
        return Err(StatusError::with_status(
            StatusCode::TOO_MANY_REQUESTS,
            format!(
                "rate limit: max {} posts per hour",
                state.config.security.max_posts_per_hour
            ),
        ));
    }

    validate_market_fields(&input)?;

    let post = crate::db::posts::create(&state.pool, auth.user_id, input).await?;
    Ok(Json(post))
}

/// The `chk_posts_market_fields` and `chk_posts_currency` constraints are the real authority on
/// this; checking here turns a constraint violation (a 500, with the SQL in the log) into a 400
/// that says what is wrong.
fn validate_market_fields(input: &CreatePost) -> Result<(), StatusError> {
    let bad_request =
        |m: &str| StatusError::with_status(StatusCode::BAD_REQUEST, m.to_string());

    if !input.kind.is_market()
        && (input.market_listed
            || input.price_cents.is_some()
            || input.item_condition.is_some())
    {
        return Err(bad_request(
            "price, condition and market_listed belong to 'listing' and 'want' posts only",
        ));
    }
    if input.price_cents.is_some_and(|c| c < 0) {
        return Err(bad_request("price_cents cannot be negative"));
    }
    if let Some(currency) = &input.currency {
        if currency.len() != 3 || !currency.chars().all(|c| c.is_ascii_uppercase()) {
            return Err(bad_request("currency must be a three-letter uppercase code"));
        }
    }
    Ok(())
}

#[derive(Deserialize)]
struct UpdatePostRequest {
    title: Option<String>,
    body: Option<String>,
    /// Typed, so an unknown value is a 422 from serde rather than a CHECK violation at the
    /// bottom of the stack.
    urgency: Option<Urgency>,
    status: Option<PostStatus>,
}

async fn update_post(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdatePostRequest>,
) -> Result<Json<serde_json::Value>, StatusError> {
    let post = load_post(&state, id).await?;
    if post.author_id != auth.user_id {
        return Err(StatusError::with_status(StatusCode::FORBIDDEN, "not your post"));
    }

    // `hidden` and `flagged` are moderation states; an author setting either on their own post
    // would either hide it from moderators' queues or fake a report outcome.
    if matches!(input.status, Some(PostStatus::Hidden) | Some(PostStatus::Flagged)) {
        return Err(StatusError::with_status(
            StatusCode::FORBIDDEN,
            "that status is set by moderators, not by the author",
        ));
    }

    crate::db::posts::update(&state.pool, id, input.title, input.body, input.urgency, input.status)
        .await?;
    Ok(Json(json!({"status": "updated"})))
}

async fn withdraw_post(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusError> {
    let post = load_post(&state, id).await?;
    if post.author_id != auth.user_id {
        return Err(StatusError::with_status(StatusCode::FORBIDDEN, "not your post"));
    }
    crate::db::posts::withdraw(&state.pool, id).await?;
    Ok(Json(json!({"status": "withdrawn"})))
}

async fn upload_images(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, StatusError> {
    let post = load_post(&state, id).await?;
    if post.author_id != auth.user_id {
        return Err(StatusError::with_status(StatusCode::FORBIDDEN, "not your post"));
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
            .map_err(|_| StatusError::with_status(StatusCode::BAD_REQUEST, "invalid image"))?;
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

/// A missing post is a 404. The old code let `anyhow!("post not found")` fall through the
/// blanket `From` impl and answered 500.
async fn load_post(state: &AppState, id: Uuid) -> Result<Post, StatusError> {
    crate::db::posts::get(&state.pool, id)
        .await?
        .ok_or_else(|| StatusError::with_status(StatusCode::NOT_FOUND, "post not found"))
}
