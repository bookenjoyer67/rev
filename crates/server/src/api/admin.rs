use axum::{
    extract::{Extension, Path, State},
    middleware,
    routing::{delete, get, patch},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::auth::{require_superadmin, AuthUser};
use crate::AppState;
use super::communities::StatusError;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/admin/stats", get(stats))
        .route("/admin/users", get(list_users))
        .route("/admin/users/{id}", delete(delete_user))
        .route("/admin/users/{id}/role", patch(change_role))
        .route("/admin/communities", get(list_communities))
        .route("/admin/communities/{id}", delete(delete_community))
        .route("/admin/directory", get(list_directory))
        .route("/admin/directory/{url}", delete(remove_directory_entry))
        .layer(middleware::from_fn_with_state(state.clone(), require_superadmin))
        .with_state(state)
}

async fn stats(State(state): State<AppState>) -> Result<Json<serde_json::Value>, StatusError> {
    let users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.pool).await?;
    let communities: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM communities")
        .fetch_one(&state.pool).await?;
    let active_posts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE status = 'active'")
        .fetch_one(&state.pool).await?;
    let total_posts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts")
        .fetch_one(&state.pool).await?;
    let matches: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM matches")
        .fetch_one(&state.pool).await?;
    let messages: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages")
        .fetch_one(&state.pool).await?;
    let directory: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM directory_entries")
        .fetch_one(&state.pool).await?;

    Ok(Json(serde_json::json!({
        "users": users,
        "communities": communities,
        "active_posts": active_posts,
        "total_posts": total_posts,
        "matches": matches,
        "messages": messages,
        "directory_entries": directory,
    })))
}

#[derive(Serialize, FromRow)]
struct AdminUser {
    id: Uuid,
    display_name: String,
    role: String,
    last_seen: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

async fn list_users(State(state): State<AppState>) -> Result<Json<Vec<AdminUser>>, StatusError> {
    let users = sqlx::query_as::<_, AdminUser>(
        "SELECT id, display_name, role, last_seen, created_at FROM users ORDER BY created_at DESC"
    )
    .fetch_all(&state.pool).await?;
    Ok(Json(users))
}

async fn delete_user(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusError> {
    if id == auth.user_id {
        return Ok(Json(serde_json::json!({"error": "cannot delete yourself"})));
    }
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(&state.pool).await?;
    Ok(Json(serde_json::json!({"status": "deleted"})))
}

#[derive(Deserialize)]
struct ChangeRoleRequest {
    role: String,
}

async fn change_role(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(input): Json<ChangeRoleRequest>,
) -> Result<Json<serde_json::Value>, StatusError> {
    if id == auth.user_id {
        return Ok(Json(serde_json::json!({"error": "cannot change your own role"})));
    }
    let valid_roles = ["user", "admin", "superadmin"];
    if !valid_roles.contains(&input.role.as_str()) {
        return Ok(Json(serde_json::json!({"error": "invalid role"})));
    }
    sqlx::query("UPDATE users SET role = $2 WHERE id = $1")
        .bind(id)
        .bind(&input.role)
        .execute(&state.pool).await?;
    Ok(Json(serde_json::json!({"status": "updated", "role": input.role})))
}

#[derive(Serialize, FromRow)]
struct AdminCommunity {
    id: Uuid,
    slug: String,
    name: String,
    description: Option<String>,
    visibility: String,
    created_at: DateTime<Utc>,
}

async fn list_communities(State(state): State<AppState>) -> Result<Json<Vec<AdminCommunity>>, StatusError> {
    let communities = sqlx::query_as::<_, AdminCommunity>(
        "SELECT id, slug, name, description, visibility, created_at FROM communities ORDER BY created_at DESC"
    )
    .fetch_all(&state.pool).await?;
    Ok(Json(communities))
}

async fn delete_community(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusError> {
    sqlx::query("DELETE FROM communities WHERE id = $1")
        .bind(id)
        .execute(&state.pool).await?;
    Ok(Json(serde_json::json!({"status": "deleted"})))
}

#[derive(Serialize, FromRow)]
struct AdminDirectoryEntry {
    url: String,
    name: String,
    location_name: Option<String>,
    communities_count: Option<i64>,
    last_seen: DateTime<Utc>,
    registered_at: DateTime<Utc>,
}

async fn list_directory(State(state): State<AppState>) -> Result<Json<Vec<AdminDirectoryEntry>>, StatusError> {
    let entries = sqlx::query_as::<_, AdminDirectoryEntry>(
        "SELECT url, name, location_name, communities_count, last_seen, registered_at FROM directory_entries ORDER BY registered_at DESC"
    )
    .fetch_all(&state.pool).await?;
    Ok(Json(entries))
}

async fn remove_directory_entry(
    State(state): State<AppState>,
    Path(url): Path<String>,
) -> Result<Json<serde_json::Value>, StatusError> {
    sqlx::query("DELETE FROM directory_entries WHERE url = $1")
        .bind(&url)
        .execute(&state.pool).await?;
    Ok(Json(serde_json::json!({"status": "removed"})))
}
