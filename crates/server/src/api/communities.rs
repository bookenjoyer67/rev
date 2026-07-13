use axum::{
    extract::{Extension, Multipart, Path, Request, State},
    http::header,
    middleware,
    routing::{delete, get, patch, post},
    Json, Router,
};
use serde::Serialize;
use uuid::Uuid;

use komun_core::models::{Community, CreateCommunity, Invite};
use crate::auth::{require_auth, verify_token, AuthUser};
use crate::AppState;

pub fn router(state: AppState) -> Router {
    let public = Router::new()
        .route("/", get(list_communities))
        .route("/{slug}", get(get_community))
        .route("/{slug}/members", get(list_members));

    let protected = Router::new()
        .route("/", post(create_community))
        .route("/{slug}", patch(update_community))
        .route("/{slug}/image", post(upload_community_image))
        .route("/{slug}/invite", post(create_invite))
        .route("/{slug}/invites", get(list_invites))
        .route("/{slug}/invites/{code}", delete(delete_invite))
        .route("/{slug}/join", post(join_community))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    public.merge(protected).with_state(state)
}

#[derive(Serialize)]
struct CommunityResponse {
    #[serde(flatten)]
    community: Community,
    is_member: bool,
    member_role: Option<String>,
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
    request: Request,
) -> Result<Json<CommunityResponse>, StatusError> {
    let community = crate::db::communities::get_by_slug(&state.pool, &slug).await?;

    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_default();
    let user_id = request.headers().get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .and_then(|token| verify_token(&jwt_secret, token));

    let member_role = if let Some(uid) = user_id {
        crate::db::communities::get_member_role(&state.pool, community.id, uid).await.ok().flatten()
    } else {
        None
    };

    Ok(Json(CommunityResponse {
        is_member: member_role.is_some(),
        member_role,
        community,
    }))
}

async fn create_community(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(input): Json<CreateCommunity>,
) -> Result<Json<Community>, StatusError> {
    let description = input.description.clone().unwrap_or_default();
    let visibility_str = match input.visibility.clone().unwrap_or(komun_core::models::Visibility::Federated) {
        komun_core::models::Visibility::Public => "public",
        komun_core::models::Visibility::Federated => "federated",
        komun_core::models::Visibility::Private => "private",
    };

    let (map_community_id, map_secret_key) = if let Some(ref store) = state.relay_store {
        match crate::relay_ops::create_relay_community(
            store,
            &input.name,
            &description,
            visibility_str,
        ).await {
            Ok((cid, secret)) => (uuid::Uuid::parse_str(&cid).ok(), Some(secret)),
            Err(e) => {
                tracing::warn!("relay community creation failed: {}", e);
                (None, None)
            }
        }
    } else {
        (None, None)
    };

    let community = crate::db::communities::create(
        &state.pool,
        input,
        map_community_id,
        map_secret_key.as_deref(),
    ).await?;
    crate::db::communities::add_member(&state.pool, community.id, auth.user_id, "admin").await?;
    if state.config.discovery.listed || state.config.discovery.directory_enabled {
        let _ = crate::db::communities::refresh_directory_communities(&state.pool, &state.config.public_url()).await;
    }
    Ok(Json(community))
}

async fn update_community(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(slug): Path<String>,
    Json(input): Json<UpdateCommunityRequest>,
) -> Result<Json<serde_json::Value>, StatusError> {
    let community = crate::db::communities::get_by_slug(&state.pool, &slug).await?;
    let role = crate::db::communities::get_member_role(&state.pool, community.id, auth.user_id).await?;
    if role.as_deref() != Some("admin") {
        return Ok(Json(serde_json::json!({"error": "admin access required"})));
    }
    crate::db::communities::update_community(&state.pool, &slug, input.name, input.description, input.visibility, input.location_name, input.location_lat, input.location_lon).await?;
    if state.config.discovery.listed || state.config.discovery.directory_enabled {
        let _ = crate::db::communities::refresh_directory_communities(&state.pool, &state.config.public_url()).await;
    }
    Ok(Json(serde_json::json!({"status": "updated"})))
}

#[derive(serde::Deserialize)]
struct UpdateCommunityRequest {
    name: Option<String>,
    description: Option<String>,
    visibility: Option<String>,
    location_name: Option<String>,
    location_lat: Option<f64>,
    location_lon: Option<f64>,
}

async fn create_invite(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(slug): Path<String>,
) -> Result<Json<Invite>, StatusError> {
    let community = crate::db::communities::get_by_slug(&state.pool, &slug).await?;
    let role = crate::db::communities::get_member_role(&state.pool, community.id, auth.user_id).await?;
    if role.as_deref() != Some("admin") {
        return Err(anyhow::anyhow!("admin access required").into());
    }
    let invite = crate::db::communities::create_invite(&state.pool, community.id, auth.user_id).await?;
    Ok(Json(invite))
}

async fn list_invites(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(slug): Path<String>,
) -> Result<Json<Vec<Invite>>, StatusError> {
    let community = crate::db::communities::get_by_slug(&state.pool, &slug).await?;
    let role = crate::db::communities::get_member_role(&state.pool, community.id, auth.user_id).await?;
    if role.as_deref() != Some("admin") {
        return Err(anyhow::anyhow!("admin access required").into());
    }
    let invites = crate::db::communities::list_invites(&state.pool, community.id).await?;
    Ok(Json(invites))
}

async fn delete_invite(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path((slug, code)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusError> {
    let community = crate::db::communities::get_by_slug(&state.pool, &slug).await?;
    let role = crate::db::communities::get_member_role(&state.pool, community.id, auth.user_id).await?;
    if role.as_deref() != Some("admin") {
        return Err(anyhow::anyhow!("admin access required").into());
    }
    crate::db::communities::delete_invite(&state.pool, &code).await?;
    Ok(Json(serde_json::json!({"status": "deleted"})))
}

#[derive(serde::Serialize, sqlx::FromRow)]
struct MemberInfo {
    user_id: uuid::Uuid,
    display_name: String,
    role: String,
    joined_at: chrono::DateTime<chrono::Utc>,
}

async fn list_members(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Vec<MemberInfo>>, StatusError> {
    let community = crate::db::communities::get_by_slug(&state.pool, &slug).await?;
    let members = sqlx::query_as::<_, MemberInfo>(
        "SELECT user_id, display_name, role, joined_at FROM members WHERE community_id = $1 ORDER BY joined_at"
    )
    .bind(community.id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(members))
}

async fn join_community(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(slug): Path<String>,
    Json(payload): Json<JoinPayload>,
) -> Result<Json<serde_json::Value>, StatusError> {
    let community = crate::db::communities::get_by_slug(&state.pool, &slug).await?;

    let has_invites: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM invites WHERE community_id = $1")
        .bind(community.id)
        .fetch_one(&state.pool)
        .await?;

    if has_invites > 0 {
        if payload.code.is_empty() {
            return Err(anyhow::anyhow!("this community requires an invite code").into());
        }
        crate::db::communities::use_invite(&state.pool, &payload.code).await?;
    }

    crate::db::communities::add_member(&state.pool, community.id, auth.user_id, "member").await?;
    Ok(Json(serde_json::json!({"status": "joined"})))
}

#[derive(serde::Deserialize)]
struct JoinPayload {
    code: String,
}

use axum::response::{IntoResponse, Response};
use axum::http::StatusCode;

pub struct StatusError {
    status: StatusCode,
    inner: anyhow::Error,
}

impl<E: Into<anyhow::Error>> From<E> for StatusError {
    fn from(err: E) -> Self {
        StatusError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            inner: err.into(),
        }
    }
}

async fn upload_community_image(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(slug): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, StatusError> {
    let community = crate::db::communities::get_by_slug(&state.pool, &slug).await
        .map_err(|e| StatusError::with_status(StatusCode::NOT_FOUND, e))?;
    let role = crate::db::communities::get_member_role(&state.pool, community.id, auth.user_id).await
        .map_err(|_| StatusError::with_status(StatusCode::FORBIDDEN, "access denied"))?;
    if role.as_deref() != Some("admin") {
        return Err(StatusError::with_status(StatusCode::FORBIDDEN, "admin access required"));
    }

    let mut data: Vec<u8> = Vec::new();
    let mut content_type: Option<String> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            content_type = field.content_type().map(|s| s.to_string());
            data = field.bytes().await.map_err(|e|
                StatusError::with_status(StatusCode::BAD_REQUEST, format!("read error: {}", e))
            )?.to_vec();
            break;
        }
    }

    if data.is_empty() {
        return Err(StatusError::with_status(StatusCode::BAD_REQUEST, "no image file provided"));
    }

    let ct = content_type.as_deref().unwrap_or("");
    if ct != "image/png" && ct != "image/jpeg" && ct != "image/webp" {
        return Err(StatusError::with_status(StatusCode::BAD_REQUEST, "image must be PNG, JPEG, or WebP"));
    }

    if data.len() > state.config.media.max_community_image_bytes as usize {
        return Err(StatusError::with_status(StatusCode::BAD_REQUEST, "image too large (max 1MB)"));
    }

    let img = image::load_from_memory(&data).map_err(|e|
        StatusError::with_status(StatusCode::BAD_REQUEST, format!("invalid image: {}", e))
    )?;
    let img = img.resize(512, 512, image::imageops::FilterType::Lanczos3);
    let mut webp: Vec<u8> = Vec::new();
    let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut webp);
    img.write_with_encoder(encoder).map_err(|e|
        StatusError::with_status(StatusCode::INTERNAL_SERVER_ERROR, format!("encode error: {}", e))
    )?;

    let image_id = Uuid::now_v7();
    let filename = format!("{}.webp", image_id);
    let dir = std::path::Path::new(&state.config.media.community_images_dir);
    std::fs::create_dir_all(dir).ok();
    std::fs::write(dir.join(&filename), &webp).map_err(|e|
        StatusError::with_status(StatusCode::INTERNAL_SERVER_ERROR, format!("write error: {}", e))
    )?;

    sqlx::query("UPDATE communities SET image_path = $1 WHERE id = $2")
        .bind(&filename)
        .bind(community.id)
        .execute(&state.pool).await
        .map_err(|e| StatusError::with_status(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let url = format!("/community-images/{}", filename);
    Ok(Json(serde_json::json!({"image_url": url})))
}

impl StatusError {
    pub fn with_status(status: StatusCode, message: impl std::fmt::Display) -> Self {
        StatusError {
            status,
            inner: anyhow::anyhow!("{}", message),
        }
    }
}

impl IntoResponse for StatusError {
    fn into_response(self) -> Response {
        if self.status == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!("request error: {:?}", self.inner);
        }
        (
            self.status,
            Json(serde_json::json!({"error": self.inner.to_string()})),
        ).into_response()
    }
}
