use axum::{
    extract::{Extension, Path, Request, State},
    http::header,
    middleware,
    routing::{delete, get, patch, post},
    Json, Router,
};
use serde::Serialize;

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
        .route("/{slug}/invite", post(create_invite))
        .route("/{slug}/invites", get(list_invites))
        .route("/{slug}/invites/{code}", delete(delete_invite))
        .route("/{slug}/join", post(join_community))
        .layer(middleware::from_fn(require_auth));

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
        "SELECT display_name, role, joined_at FROM members WHERE community_id = $1 ORDER BY joined_at"
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
