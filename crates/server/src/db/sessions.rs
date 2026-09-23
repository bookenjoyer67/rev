//! Queries over `sessions` and `one_time_tokens` (A2a / SPEC Part 1.5).
//!
//! The raw session token never reaches this module's storage path: every function takes or
//! returns a `token_hash`, which is `SHA-256(raw token)`. A database dump therefore contains no
//! usable credential — the hash cannot be replayed as a bearer token.

use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// A session as the *owner* of the account may see it. `token_hash` is absent by construction, and
/// so is the raw `user_agent`: `device_label` is the readable summary derived from it at creation
/// time, and echoing the full header back adds nothing but fingerprinting surface.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SessionRow {
    pub id: Uuid,
    pub device_label: Option<String>,
    pub ip: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// What the middleware needs on every authenticated request: who the caller is, and what they are
/// allowed to do *right now*. `role` and `email_verified` come from `users` in the same round trip,
/// so a demotion or a revoked verification takes effect on the very next request rather than when
/// some cached token happens to expire.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuthenticatedSession {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub email_verified: bool,
    pub last_used_at: DateTime<Utc>,
}

/// Insert a session for an already-authenticated user. The caller generates the token and passes
/// only its hash; this function never sees the secret.
pub async fn create(
    pool: &PgPool,
    user_id: Uuid,
    token_hash: &[u8],
    lifetime_days: i64,
    device_label: Option<&str>,
    user_agent: Option<&str>,
    ip: Option<&str>,
) -> Result<SessionRow, sqlx::Error> {
    let id = Uuid::now_v7();
    let expires_at = Utc::now() + Duration::days(lifetime_days);

    sqlx::query_as::<_, SessionRow>(
        r#"INSERT INTO sessions (id, user_id, token_hash, device_label, user_agent, ip, expires_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7)
           RETURNING id, device_label, ip, created_at, last_used_at, expires_at"#,
    )
    .bind(id)
    .bind(user_id)
    .bind(token_hash)
    .bind(device_label)
    .bind(user_agent)
    .bind(ip)
    .bind(expires_at)
    .fetch_one(pool)
    .await
}

/// Look a session up by token hash, rejecting revoked and expired rows in SQL so no caller can
/// forget the check. Returns `None` for "no such session" and for "session no longer valid"
/// alike — the distinction is not the client's business.
pub async fn lookup(
    pool: &PgPool,
    token_hash: &[u8],
) -> Result<Option<AuthenticatedSession>, sqlx::Error> {
    sqlx::query_as::<_, AuthenticatedSession>(
        r#"SELECT s.id AS session_id,
                  s.user_id,
                  u.role,
                  (u.email_verified_at IS NOT NULL) AS email_verified,
                  s.last_used_at
           FROM sessions s
           JOIN users u ON u.id = s.user_id
           WHERE s.token_hash = $1
             AND s.revoked_at IS NULL
             AND s.expires_at > now()"#,
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await
}

/// Sliding expiry. Throttled by the caller to at most one write per minute per session, so a
/// chatty client does not turn every read into a write.
pub async fn touch(pool: &PgPool, session_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE sessions SET last_used_at = now() WHERE id = $1")
        .bind(session_id)
        .execute(pool)
        .await
        .map(|_| ())
}

/// Every live session for a user, newest first — the "where am I signed in?" list.
pub async fn list_for_user(pool: &PgPool, user_id: Uuid) -> Result<Vec<SessionRow>, sqlx::Error> {
    sqlx::query_as::<_, SessionRow>(
        r#"SELECT id, device_label, ip, created_at, last_used_at, expires_at
           FROM sessions
           WHERE user_id = $1 AND revoked_at IS NULL AND expires_at > now()
           ORDER BY last_used_at DESC"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

/// Revoke one session, but only if it belongs to `user_id` — otherwise any authenticated user
/// could sign out any other by guessing a session id. Returns false when nothing matched.
pub async fn revoke(pool: &PgPool, user_id: Uuid, session_id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE sessions SET revoked_at = now() WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL",
    )
    .bind(session_id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// Revoke every session for a user except the one making the request. Used after a password
/// change and by the "sign out everywhere else" control.
pub async fn revoke_all_except(
    pool: &PgPool,
    user_id: Uuid,
    keep_session_id: Uuid,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE sessions SET revoked_at = now() WHERE user_id = $1 AND id <> $2 AND revoked_at IS NULL",
    )
    .bind(user_id)
    .bind(keep_session_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

/// Revoke every session for a user, including the current one. Used when an admin disables an
/// account and when a password reset completes.
pub async fn revoke_all(pool: &PgPool, user_id: Uuid) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE sessions SET revoked_at = now() WHERE user_id = $1 AND revoked_at IS NULL",
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

/// Delete rows that can never authenticate again: expired, or revoked long enough ago that they
/// are no longer interesting for an audit trail. Run periodically by the background task.
pub async fn delete_stale(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM sessions WHERE expires_at < now() - interval '30 days'
            OR (revoked_at IS NOT NULL AND revoked_at < now() - interval '30 days')",
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

// ---------------------------------------------------------------------------
// one_time_tokens — email verification and password reset
// ---------------------------------------------------------------------------

/// The two kinds the `chk_one_time_tokens_kind` CHECK allows. Spelling them once here keeps a typo
/// from becoming a runtime constraint violation.
pub const KIND_EMAIL_VERIFY: &str = "email_verify";
pub const KIND_PASSWORD_RESET: &str = "password_reset";

/// Mint a one-time token row. As with sessions, only the hash is stored.
pub async fn create_one_time_token(
    pool: &PgPool,
    user_id: Uuid,
    kind: &str,
    token_hash: &[u8],
    ttl_minutes: i64,
) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::now_v7();
    let expires_at = Utc::now() + Duration::minutes(ttl_minutes);
    sqlx::query(
        r#"INSERT INTO one_time_tokens (id, user_id, kind, token_hash, expires_at)
           VALUES ($1, $2, $3, $4, $5)"#,
    )
    .bind(id)
    .bind(user_id)
    .bind(kind)
    .bind(token_hash)
    .bind(expires_at)
    .execute(pool)
    .await?;
    Ok(id)
}

/// Atomically claim a one-time token: the `used_at IS NULL` predicate and the `SET used_at` live
/// in the same statement, so two concurrent requests cannot both succeed. Single-use is enforced
/// by the database, not by a check-then-act in Rust.
pub async fn consume_one_time_token(
    pool: &PgPool,
    kind: &str,
    token_hash: &[u8],
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar::<_, Uuid>(
        r#"UPDATE one_time_tokens
           SET used_at = now()
           WHERE token_hash = $1
             AND kind = $2
             AND used_at IS NULL
             AND expires_at > now()
           RETURNING user_id"#,
    )
    .bind(token_hash)
    .bind(kind)
    .fetch_optional(pool)
    .await
}

/// Invalidate any outstanding tokens of a kind for a user, so that issuing a new verification
/// link retires the old one instead of leaving several live at once.
pub async fn invalidate_one_time_tokens(
    pool: &PgPool,
    user_id: Uuid,
    kind: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE one_time_tokens SET used_at = now() WHERE user_id = $1 AND kind = $2 AND used_at IS NULL",
    )
    .bind(user_id)
    .bind(kind)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

/// How many tokens of a kind were minted for a user recently — the per-account half of the
/// resend rate limit (the other half is by IP, in `rate_limit`).
pub async fn count_recent_one_time_tokens(
    pool: &PgPool,
    user_id: Uuid,
    kind: &str,
    within_minutes: i64,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM one_time_tokens
         WHERE user_id = $1 AND kind = $2 AND created_at > now() - make_interval(mins => $3)",
    )
    .bind(user_id)
    .bind(kind)
    .bind(within_minutes as i32)
    .fetch_one(pool)
    .await
}

/// Drop one-time tokens that are spent or long expired.
pub async fn delete_stale_one_time_tokens(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM one_time_tokens WHERE expires_at < now() - interval '7 days'
            OR (used_at IS NOT NULL AND used_at < now() - interval '7 days')",
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}
