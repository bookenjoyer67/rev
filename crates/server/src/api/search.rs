use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::AppState;
use super::communities::StatusError;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/search", get(search))
        .route("/search/communities", get(search_communities))
        .route("/search/users", get(search_users))
        .with_state(state)
}

#[derive(Deserialize)]
struct SearchParams {
    q: String,
    kind: Option<String>,
    community: Option<String>,
    limit: Option<i64>,
}

#[derive(Serialize, FromRow)]
struct SearchResult {
    id: Uuid,
    community_id: Uuid,
    kind: String,
    category: String,
    title: String,
    body: Option<String>,
    location_name: Option<String>,
    urgency: Option<String>,
    status: String,
    tags: Option<serde_json::Value>,
    author_id: Uuid,
    verified_by: Option<Uuid>,
    created_at: DateTime<Utc>,
    rank: f32,
    community_slug: Option<String>,
    community_name: Option<String>,
}

#[derive(Serialize, FromRow)]
struct CommunitySearchResult {
    id: Uuid,
    slug: String,
    name: String,
    description: Option<String>,
    location_name: Option<String>,
}

#[derive(Serialize, FromRow)]
struct UserSearchResult {
    id: Uuid,
    display_name: String,
    role: String,
    endorsement_count: Option<i64>,
}

async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<Vec<SearchResult>>, StatusError> {
    let limit = params.limit.unwrap_or(20).min(50);
    let query = params.q.trim();
    if query.is_empty() {
        return Ok(Json(vec![]));
    }

    let mut sql = String::from(
        r#"SELECT p.id, p.community_id, p.kind, p.category, p.title, p.body,
           p.location_name, p.urgency, p.status, p.tags, p.author_id, p.verified_by,
           p.created_at,
           ts_rank(p.search_vector, plainto_tsquery('english', $1)) AS rank,
           c.slug AS community_slug, c.name AS community_name
           FROM posts p
           JOIN communities c ON c.id = p.community_id
           WHERE p.search_vector @@ plainto_tsquery('english', $1)
             AND p.status = 'active'"#
    );

    if params.kind.is_some() {
        sql.push_str(" AND p.kind = $2");
    }
    if params.community.is_some() {
        sql.push_str(&format!(
            " AND c.slug = ${}",
            if params.kind.is_some() { "$3" } else { "$2" }
        ));
    }

    sql.push_str(" ORDER BY rank DESC LIMIT $");
    sql.push_str(&(1 + params.kind.is_some() as usize + params.community.is_some() as usize + 1).to_string());

    let mut q = sqlx::query_as::<_, SearchResult>(&sql).bind(query);

    if let Some(ref kind) = params.kind {
        q = q.bind(kind);
    }
    if let Some(ref slug) = params.community {
        q = q.bind(slug);
    }
    q = q.bind(limit);

    let results = q.fetch_all(&state.pool).await?;
    Ok(Json(results))
}

async fn search_communities(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<Vec<CommunitySearchResult>>, StatusError> {
    let query = params.q.trim();
    if query.is_empty() {
        return Ok(Json(vec![]));
    }

    let pattern = format!("%{}%", query);
    let results = sqlx::query_as::<_, CommunitySearchResult>(
        r#"SELECT id, slug, name, description, location_name
           FROM communities
           WHERE name ILIKE $1 OR description ILIKE $1 OR location_name ILIKE $1
           ORDER BY name
           LIMIT 20"#
    )
    .bind(&pattern)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(results))
}

async fn search_users(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<Vec<UserSearchResult>>, StatusError> {
    let query = params.q.trim();
    if query.is_empty() {
        return Ok(Json(vec![]));
    }

    let pattern = format!("%{}%", query);
    let results = sqlx::query_as::<_, UserSearchResult>(
        r#"SELECT u.id, u.display_name, u.role,
           (SELECT COUNT(*) FROM endorsements WHERE endorsee_id = u.id) AS endorsement_count
           FROM users u
           WHERE u.display_name ILIKE $1
           ORDER BY u.display_name
           LIMIT 20"#
    )
    .bind(&pattern)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(results))
}
