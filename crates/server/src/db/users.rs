use sqlx::PgPool;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
pub struct UserProfileRow {
    pub id: Uuid,
    pub display_name: String,
    pub bio: Option<String>,
    pub avatar_path: Option<String>,
    pub encryption_public_key: Option<Vec<u8>>,
    pub role: String,
    pub post_count: i64,
    pub verified_post_count: i64,
    pub endorsement_count: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_seen: chrono::DateTime<chrono::Utc>,
    pub profile_json: serde_json::Value,
}

pub async fn get_profile(pool: &PgPool, user_id: Uuid) -> Result<Option<UserProfileRow>, sqlx::Error> {
    // A2a: the previous version joined LATERAL against `members` and selected `u.public_key`,
    // both of which A1 dropped — so every call failed at runtime with an undefined-column error.
    // A3.2 removes the two stand-ins A2a left behind: `public_key` (an alias of
    // `encryption_public_key`, kept only so the response shape did not change mid-flight) and
    // `community_count` (a literal 0 standing in for the dropped `members` table).
    sqlx::query_as::<_, UserProfileRow>(
        r#"SELECT
            u.id, u.display_name, u.bio, u.avatar_path,
            u.encryption_public_key,
            u.role, u.created_at, u.last_seen, u.profile_json,
            COALESCE(p.post_count, 0) as post_count,
            COALESCE(v.verified_count, 0) as verified_post_count,
            COALESCE(e.endorsement_count, 0) as endorsement_count
        FROM users u
        LEFT JOIN LATERAL (
            SELECT COUNT(*)::bigint as post_count FROM posts WHERE author_id = u.id
        ) p ON true
        LEFT JOIN LATERAL (
            SELECT COUNT(*)::bigint as verified_count FROM posts WHERE author_id = u.id AND verified_by IS NOT NULL
        ) v ON true
        LEFT JOIN LATERAL (
            SELECT COUNT(*)::bigint as endorsement_count FROM endorsements WHERE endorsee_id = u.id
        ) e ON true
        WHERE u.id = $1"#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}
