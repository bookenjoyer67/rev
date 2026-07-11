use anyhow::Result;
use sqlx::PgPool;

use komun_core::models::{Alliance, AllianceStatus};

pub async fn list_alliances(pool: &PgPool) -> Result<Vec<Alliance>> {
    let rows = sqlx::query_as::<_, AllianceRow>(
        "SELECT id, remote_domain, remote_name, status, initiated_by, created_at FROM alliances ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| Alliance {
        id: r.id,
        remote_domain: r.remote_domain,
        remote_name: r.remote_name,
        status: match r.status.as_str() {
            "pending" => AllianceStatus::Pending,
            "accepted" => AllianceStatus::Accepted,
            "rejected" => AllianceStatus::Rejected,
            "severed" => AllianceStatus::Severed,
            _ => AllianceStatus::Pending,
        },
        initiated_by: r.initiated_by,
        created_at: r.created_at,
    }).collect())
}

#[derive(sqlx::FromRow)]
struct AllianceRow {
    id: uuid::Uuid,
    remote_domain: String,
    remote_name: Option<String>,
    status: String,
    initiated_by: String,
    created_at: chrono::DateTime<chrono::Utc>,
}
