use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, FromRow, serde::Serialize)]
pub struct Report {
    pub id: Uuid,
    pub reporter_id: Uuid,
    pub post_id: Uuid,
    pub reason: String,
    pub status: String,
    pub admin_notes: Option<String>,
    pub resolved_by: Option<Uuid>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

pub async fn create_report(
    pool: &PgPool,
    reporter_id: Uuid,
    post_id: Uuid,
    reason: &str,
) -> Result<Report> {
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM reports WHERE reporter_id = $1 AND post_id = $2 AND status = 'pending'"
    )
    .bind(reporter_id)
    .bind(post_id)
    .fetch_one(pool)
    .await?;

    if existing > 0 {
        return Err(anyhow!("already reported this post"));
    }

    let id = Uuid::now_v7();
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO reports (id, reporter_id, post_id, reason, status, created_at) VALUES ($1, $2, $3, $4, 'pending', $5)"
    )
    .bind(id)
    .bind(reporter_id)
    .bind(post_id)
    .bind(reason)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(Report {
        id,
        reporter_id,
        post_id,
        reason: reason.to_string(),
        status: "pending".into(),
        admin_notes: None,
        resolved_by: None,
        resolved_at: None,
        created_at: now,
    })
}

pub async fn list_reports(pool: &PgPool) -> Result<Vec<Report>> {
    let rows = sqlx::query_as::<_, Report>(
        "SELECT id, reporter_id, post_id, reason, status, admin_notes, resolved_by, resolved_at, created_at FROM reports ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn resolve_report(
    pool: &PgPool,
    report_id: Uuid,
    status: &str,
    admin_notes: Option<&str>,
    resolved_by: Uuid,
) -> Result<()> {
    let now = Utc::now();
    sqlx::query(
        "UPDATE reports SET status = $1, admin_notes = $2, resolved_by = $3, resolved_at = $4 WHERE id = $5"
    )
    .bind(status)
    .bind(admin_notes)
    .bind(resolved_by)
    .bind(now)
    .bind(report_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn hide_post(pool: &PgPool, post_id: Uuid) -> Result<()> {
    sqlx::query("UPDATE posts SET status = 'hidden' WHERE id = $1")
        .bind(post_id)
        .execute(pool)
        .await?;
    Ok(())
}
