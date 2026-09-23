use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::{FromRow, PgPool, Row};
use uuid::Uuid;

// A1.5: `Community`, `CreateCommunity` and `Invite` left komun-core with the rest of the
// multi-tenant model, and the `communities` / `members` / `invites` tables are gone from the
// schema. These local stand-ins keep the module compiling until A3.1 deletes it outright.
// `visibility` is a plain string here because `Visibility::Federated` no longer exists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Community {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub location_name: Option<String>,
    pub location_lat: Option<f64>,
    pub location_lon: Option<f64>,
    pub visibility: String,
    pub created_at: chrono::DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map_community_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map_secret_hex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCommunity {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub location_name: Option<String>,
    pub location_lat: Option<f64>,
    pub location_lon: Option<f64>,
    pub visibility: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invite {
    pub code: String,
    pub community_id: Uuid,
    pub created_by: Uuid,
    pub uses_remaining: Option<i32>,
    pub expires_at: Option<chrono::DateTime<Utc>>,
    pub created_at: chrono::DateTime<Utc>,
}

pub async fn list(pool: &PgPool) -> Result<Vec<Community>> {
    let rows = sqlx::query_as::<_, CommunityRow>(
        "SELECT id, slug, name, description, location_name, location_lat, location_lon, visibility, map_community_id, map_secret_key, image_path, created_at FROM communities ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn get_by_slug(pool: &PgPool, slug: &str) -> Result<Community> {
    let row = sqlx::query_as::<_, CommunityRow>(
        "SELECT id, slug, name, description, location_name, location_lat, location_lon, visibility, map_community_id, map_secret_key, image_path, created_at FROM communities WHERE slug = $1"
    )
    .bind(slug)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| anyhow!("community not found"))?;

    Ok(row.into())
}

pub async fn create(pool: &PgPool, input: CreateCommunity, map_community_id: Option<Uuid>, map_secret_key: Option<&[u8]>) -> Result<Community> {
    let id = Uuid::now_v7();
    let visibility = input.visibility.clone().unwrap_or_else(|| "federated".to_string());

    sqlx::query(
        "INSERT INTO communities (id, slug, name, description, location_name, location_lat, location_lon, visibility, map_community_id, map_secret_key) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
    )
    .bind(id)
    .bind(&input.slug)
    .bind(&input.name)
    .bind(&input.description)
    .bind(&input.location_name)
    .bind(input.location_lat)
    .bind(input.location_lon)
    .bind(&visibility)
    .bind(map_community_id)
    .bind(map_secret_key)
    .execute(pool)
    .await?;

    get_by_slug(pool, &input.slug).await
}

pub async fn refresh_directory_communities(pool: &PgPool, server_url: &str) -> Result<()> {
    let communities_json: Option<JsonValue> = sqlx::query_scalar(
        r#"SELECT jsonb_agg(jsonb_build_object(
            'slug', slug, 'name', name,
            'location_name', location_name,
            'location_lat', location_lat,
            'location_lon', location_lon
        ) ORDER BY name)
        FROM communities
        WHERE location_lat IS NOT NULL AND location_lon IS NOT NULL"#
    )
    .fetch_one(pool)
    .await?;

    sqlx::query(
        "UPDATE directory_entries SET community_locations = $1, last_seen = now() WHERE url = $2"
    )
    .bind(&communities_json)
    .bind(server_url)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn create_invite(pool: &PgPool, community_id: Uuid, created_by: Uuid) -> Result<Invite> {
    let code = generate_invite_code();
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO invites (code, community_id, created_by, created_at) VALUES ($1, $2, $3, $4)"
    )
    .bind(&code)
    .bind(community_id)
    .bind(created_by)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(Invite {
        code,
        community_id,
        created_by,
        uses_remaining: None,
        expires_at: None,
        created_at: now,
    })
}

pub async fn use_invite(pool: &PgPool, code: &str) -> Result<Uuid> {
    let row = sqlx::query(
        "SELECT community_id, uses_remaining, expires_at FROM invites WHERE code = $1"
    )
    .bind(code)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| anyhow!("invalid invite code"))?;

    let community_id: Uuid = row.get("community_id");
    let uses_remaining: Option<i32> = row.get("uses_remaining");
    let expires_at: Option<chrono::DateTime<Utc>> = row.get("expires_at");

    if let Some(expires) = expires_at {
        if expires < Utc::now() {
            return Err(anyhow!("invite expired"));
        }
    }

    if let Some(remaining) = uses_remaining {
        if remaining <= 0 {
            return Err(anyhow!("invite exhausted"));
        }
        sqlx::query("UPDATE invites SET uses_remaining = uses_remaining - 1 WHERE code = $1")
            .bind(code)
            .execute(pool)
            .await?;
    }

    Ok(community_id)
}

pub async fn add_member(pool: &PgPool, community_id: Uuid, user_id: Uuid, role: &str) -> Result<()> {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO members (id, community_id, user_id, display_name, public_key, role) VALUES ($1, $2, $3, (SELECT display_name FROM users WHERE id = $3), (SELECT public_key FROM users WHERE id = $3), $4) ON CONFLICT (community_id, public_key) DO NOTHING"
    )
    .bind(id)
    .bind(community_id)
    .bind(user_id)
    .bind(role)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_community(
    pool: &PgPool,
    slug: &str,
    name: Option<String>,
    description: Option<String>,
    visibility: Option<String>,
    location_name: Option<String>,
    location_lat: Option<f64>,
    location_lon: Option<f64>,
) -> Result<()> {
    sqlx::query(
        r#"UPDATE communities SET
           name = COALESCE($2, name),
           description = COALESCE($3, description),
           visibility = COALESCE($4, visibility),
           location_name = COALESCE($5, location_name),
           location_lat = COALESCE($6, location_lat),
           location_lon = COALESCE($7, location_lon)
           WHERE slug = $1"#
    )
    .bind(slug)
    .bind(name)
    .bind(description)
    .bind(visibility)
    .bind(location_name)
    .bind(location_lat)
    .bind(location_lon)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_invites(pool: &PgPool, community_id: Uuid) -> Result<Vec<Invite>> {
    let rows = sqlx::query_as::<_, InviteRow>(
        "SELECT code, community_id, created_by, uses_remaining, expires_at, created_at FROM invites WHERE community_id = $1 ORDER BY created_at DESC"
    )
    .bind(community_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| Invite {
        code: r.code,
        community_id: r.community_id,
        created_by: r.created_by,
        uses_remaining: r.uses_remaining,
        expires_at: r.expires_at,
        created_at: r.created_at,
    }).collect())
}

pub async fn delete_invite(pool: &PgPool, code: &str) -> Result<()> {
    sqlx::query("DELETE FROM invites WHERE code = $1")
        .bind(code)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_member_role(pool: &PgPool, community_id: Uuid, user_id: Uuid) -> Result<Option<String>> {
    let role = sqlx::query_scalar::<_, String>(
        "SELECT role FROM members WHERE community_id = $1 AND user_id = $2"
    )
    .bind(community_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    Ok(role)
}

#[derive(FromRow)]
struct InviteRow {
    code: String,
    community_id: Uuid,
    created_by: Uuid,
    uses_remaining: Option<i32>,
    expires_at: Option<chrono::DateTime<Utc>>,
    created_at: chrono::DateTime<Utc>,
}

fn generate_invite_code() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let chars: Vec<char> = "abcdefghjkmnpqrstuvwxyz23456789".chars().collect();
    (0..8).map(|_| chars[rng.gen_range(0..chars.len())]).collect()
}

#[derive(FromRow)]
struct CommunityRow {
    id: Uuid,
    slug: String,
    name: String,
    description: Option<String>,
    location_name: Option<String>,
    location_lat: Option<f64>,
    location_lon: Option<f64>,
    visibility: String,
    map_community_id: Option<Uuid>,
    map_secret_key: Option<Vec<u8>>,
    image_path: Option<String>,
    created_at: chrono::DateTime<Utc>,
}

impl From<CommunityRow> for Community {
    fn from(r: CommunityRow) -> Self {
        Community {
            id: r.id,
            slug: r.slug,
            name: r.name,
            description: r.description,
            location_name: r.location_name,
            location_lat: r.location_lat,
            location_lon: r.location_lon,
            visibility: r.visibility,
            map_community_id: r.map_community_id,
            map_secret_hex: r.map_secret_key.map(|bytes| bytes.iter().map(|b| format!("{:02x}", b)).collect()),
            image_path: r.image_path,
            created_at: r.created_at,
        }
    }
}
