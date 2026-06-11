use anyhow::{anyhow, Result};
use chrono::Utc;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use komun_core::models::{Category, CreatePost, Post, PostKind, PostStatus, Urgency, Visibility};

pub async fn list(
    pool: &PgPool,
    community_id: Uuid,
    kind: Option<String>,
    category: Option<String>,
    status: Option<String>,
    q: Option<String>,
) -> Result<Vec<Post>> {
    let search = q.map(|s| format!("%{}%", s));
    let rows = sqlx::query_as::<_, PostRow>(
        r#"SELECT id, community_id, author_id, kind, category, title, body,
           location_name, location_lat, location_lon, urgency, quantity, status,
           visibility, expires_at, tags, contact_method, verified_by, verified_at,
           federated_id, origin_node, created_at, updated_at
           FROM posts
           WHERE community_id = $1
           AND status != 'withdrawn'
           AND ($2::text IS NULL OR kind = $2)
           AND ($3::text IS NULL OR category = $3)
           AND ($4::text IS NULL OR status = $4)
           AND ($5::text IS NULL OR title ILIKE $5 OR body ILIKE $5)
           ORDER BY
             CASE WHEN urgency = 'critical' THEN 0
                  WHEN urgency = 'high' THEN 1
                  WHEN urgency = 'medium' THEN 2
                  ELSE 3 END,
             created_at DESC"#,
    )
    .bind(community_id)
    .bind(kind)
    .bind(category)
    .bind(status)
    .bind(search)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn get(pool: &PgPool, id: Uuid) -> Result<Post> {
    let row = sqlx::query_as::<_, PostRow>(
        r#"SELECT id, community_id, author_id, kind, category, title, body,
           location_name, location_lat, location_lon, urgency, quantity, status,
           visibility, expires_at, tags, contact_method, verified_by, verified_at,
           federated_id, origin_node, created_at, updated_at
           FROM posts WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| anyhow!("post not found"))?;

    Ok(row.into())
}

pub async fn create(
    pool: &PgPool,
    community_id: Uuid,
    author_id: Uuid,
    input: CreatePost,
) -> Result<Post> {
    let id = Uuid::now_v7();
    let now = Utc::now();
    let kind = serde_json::to_string(&input.kind)?.trim_matches('"').to_string();
    let category = serde_json::to_string(&input.category)?.trim_matches('"').to_string();
    let urgency = input.urgency.as_ref().map(|u| {
        serde_json::to_string(u).unwrap_or_default().trim_matches('"').to_string()
    });
    let visibility = match input.visibility.unwrap_or(Visibility::Federated) {
        Visibility::Public => "public".to_string(),
        Visibility::Federated => "federated".to_string(),
        Visibility::Private => "private".to_string(),
    };
    let tags = input.tags.unwrap_or_default();

    sqlx::query(
        r#"INSERT INTO posts (id, community_id, author_id, kind, category, title, body,
           location_name, location_lat, location_lon, urgency, quantity, status,
           visibility, expires_at, tags, contact_method, created_at, updated_at)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,'active',$13,$14,$15,$16,$17,$17)"#,
    )
    .bind(id)
    .bind(community_id)
    .bind(author_id)
    .bind(&kind)
    .bind(&category)
    .bind(&input.title)
    .bind(&input.body)
    .bind(&input.location_name)
    .bind(input.location_lat)
    .bind(input.location_lon)
    .bind(&urgency)
    .bind(input.quantity)
    .bind(&visibility)
    .bind(input.expires_at)
    .bind(&tags)
    .bind(&input.contact_method)
    .bind(now)
    .execute(pool)
    .await?;

    get(pool, id).await
}

pub async fn update(
    pool: &PgPool,
    id: Uuid,
    title: Option<String>,
    body: Option<String>,
    urgency: Option<String>,
    status: Option<String>,
) -> Result<()> {
    sqlx::query(
        r#"UPDATE posts SET
           title = COALESCE($2, title),
           body = COALESCE($3, body),
           urgency = COALESCE($4, urgency),
           status = COALESCE($5, status),
           updated_at = $6
           WHERE id = $1"#
    )
    .bind(id)
    .bind(title)
    .bind(body)
    .bind(urgency)
    .bind(status)
    .bind(Utc::now())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn withdraw(pool: &PgPool, id: Uuid) -> Result<()> {
    sqlx::query("UPDATE posts SET status = 'withdrawn', updated_at = $2 WHERE id = $1")
        .bind(id)
        .bind(Utc::now())
        .execute(pool)
        .await?;
    Ok(())
}

#[derive(FromRow)]
struct PostRow {
    id: Uuid,
    community_id: Uuid,
    author_id: Uuid,
    kind: String,
    category: String,
    title: String,
    body: Option<String>,
    location_name: Option<String>,
    location_lat: Option<f64>,
    location_lon: Option<f64>,
    urgency: Option<String>,
    quantity: Option<i32>,
    status: String,
    visibility: String,
    expires_at: Option<chrono::DateTime<Utc>>,
    tags: Option<Vec<String>>,
    contact_method: Option<String>,
    verified_by: Option<Uuid>,
    verified_at: Option<chrono::DateTime<Utc>>,
    federated_id: Option<String>,
    origin_node: Option<String>,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

impl From<PostRow> for Post {
    fn from(r: PostRow) -> Self {
        Post {
            id: r.id,
            community_id: r.community_id,
            author_id: r.author_id,
            kind: match r.kind.as_str() {
                "resource" => PostKind::Resource,
                "need" => PostKind::Need,
                "offer" => PostKind::Offer,
                _ => PostKind::Need,
            },
            category: match r.category.as_str() {
                "food" => Category::Food,
                "shelter" => Category::Shelter,
                "health" => Category::Health,
                "transport" => Category::Transport,
                "education" => Category::Education,
                "labor" => Category::Labor,
                "legal" => Category::Legal,
                _ => Category::Other,
            },
            title: r.title,
            body: r.body,
            location_name: r.location_name,
            location_lat: r.location_lat,
            location_lon: r.location_lon,
            urgency: r.urgency.and_then(|u| match u.as_str() {
                "critical" => Some(Urgency::Critical),
                "high" => Some(Urgency::High),
                "medium" => Some(Urgency::Medium),
                "low" => Some(Urgency::Low),
                _ => None,
            }),
            quantity: r.quantity,
            status: match r.status.as_str() {
                "active" => PostStatus::Active,
                "matched" => PostStatus::Matched,
                "fulfilled" => PostStatus::Fulfilled,
                "expired" => PostStatus::Expired,
                "withdrawn" => PostStatus::Withdrawn,
                _ => PostStatus::Active,
            },
            visibility: match r.visibility.as_str() {
                "public" => Visibility::Public,
                "private" => Visibility::Private,
                _ => Visibility::Federated,
            },
            expires_at: r.expires_at,
            tags: r.tags.unwrap_or_default(),
            contact_method: r.contact_method,
            verified_by: r.verified_by,
            verified_at: r.verified_at,
            federated_id: r.federated_id,
            origin_node: r.origin_node,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
