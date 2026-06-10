use anyhow::{anyhow, Result};
use chrono::Utc;
use sqlx::{FromRow, PgPool, Row};
use uuid::Uuid;

use komun_core::models::{Community, CreateCommunity, Invite, Visibility};

pub async fn list(pool: &PgPool) -> Result<Vec<Community>> {
    let rows = sqlx::query_as::<_, CommunityRow>(
        "SELECT id, slug, name, description, location_name, location_lat, location_lon, visibility, created_at FROM communities ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn get_by_slug(pool: &PgPool, slug: &str) -> Result<Community> {
    let row = sqlx::query_as::<_, CommunityRow>(
        "SELECT id, slug, name, description, location_name, location_lat, location_lon, visibility, created_at FROM communities WHERE slug = $1"
    )
    .bind(slug)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| anyhow!("community not found"))?;

    Ok(row.into())
}

pub async fn create(pool: &PgPool, input: CreateCommunity) -> Result<Community> {
    let id = Uuid::now_v7();
    let visibility = match input.visibility.unwrap_or(Visibility::Federated) {
        Visibility::Public => "public",
        Visibility::Federated => "federated",
        Visibility::Private => "private",
    };

    sqlx::query(
        "INSERT INTO communities (id, slug, name, description, location_name, location_lat, location_lon, visibility) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
    )
    .bind(id)
    .bind(&input.slug)
    .bind(&input.name)
    .bind(&input.description)
    .bind(&input.location_name)
    .bind(input.location_lat)
    .bind(input.location_lon)
    .bind(visibility)
    .execute(pool)
    .await?;

    get_by_slug(pool, &input.slug).await
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
            visibility: match r.visibility.as_str() {
                "public" => Visibility::Public,
                "private" => Visibility::Private,
                _ => Visibility::Federated,
            },
            created_at: r.created_at,
        }
    }
}
