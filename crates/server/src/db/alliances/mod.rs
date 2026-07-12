use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AllianceWithMeta {
    pub id: Uuid,
    pub remote_domain: String,
    pub remote_name: Option<String>,
    pub remote_public_key: Option<Vec<u8>>,
    pub status: String,
    pub initiated_by: String,
    pub last_synced_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list_alliances(pool: &PgPool) -> Result<Vec<AllianceWithMeta>, sqlx::Error> {
    sqlx::query_as::<_, AllianceWithMeta>(
        "SELECT id, remote_domain, remote_name, remote_public_key, status, initiated_by, last_synced_at, created_at FROM alliances ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await
}

pub async fn create_alliance(
    pool: &PgPool,
    remote_domain: &str,
    remote_name: Option<&str>,
    remote_public_key: Option<&[u8]>,
    initiated_by: &str,
) -> Result<AllianceWithMeta, sqlx::Error> {
    let id = Uuid::now_v7();
    sqlx::query_as::<_, AllianceWithMeta>(
        r#"INSERT INTO alliances (id, remote_domain, remote_name, remote_public_key, status, initiated_by)
           VALUES ($1, $2, $3, $4, 'pending', $5)
           RETURNING id, remote_domain, remote_name, remote_public_key, status, initiated_by, last_synced_at, created_at"#
    )
    .bind(id)
    .bind(remote_domain)
    .bind(remote_name)
    .bind(remote_public_key)
    .bind(initiated_by)
    .fetch_one(pool)
    .await
}

pub async fn get_alliance(pool: &PgPool, id: Uuid) -> Result<Option<AllianceWithMeta>, sqlx::Error> {
    sqlx::query_as::<_, AllianceWithMeta>(
        "SELECT id, remote_domain, remote_name, remote_public_key, status, initiated_by, last_synced_at, created_at FROM alliances WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn find_by_domain(pool: &PgPool, domain: &str) -> Result<Option<AllianceWithMeta>, sqlx::Error> {
    sqlx::query_as::<_, AllianceWithMeta>(
        "SELECT id, remote_domain, remote_name, remote_public_key, status, initiated_by, last_synced_at, created_at FROM alliances WHERE remote_domain = $1"
    )
    .bind(domain)
    .fetch_optional(pool)
    .await
}

pub async fn update_status(pool: &PgPool, id: Uuid, status: &str) -> Result<Option<AllianceWithMeta>, sqlx::Error> {
    sqlx::query_as::<_, AllianceWithMeta>(
        "UPDATE alliances SET status = $2 WHERE id = $1 RETURNING id, remote_domain, remote_name, remote_public_key, status, initiated_by, last_synced_at, created_at"
    )
    .bind(id)
    .bind(status)
    .fetch_optional(pool)
    .await
}

pub async fn update_sync_time(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE alliances SET last_synced_at = now() WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_alliance(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let rows = sqlx::query("DELETE FROM alliances WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(rows.rows_affected() > 0)
}

pub async fn list_accepted(pool: &PgPool) -> Result<Vec<AllianceWithMeta>, sqlx::Error> {
    sqlx::query_as::<_, AllianceWithMeta>(
        "SELECT id, remote_domain, remote_name, remote_public_key, status, initiated_by, last_synced_at, created_at FROM alliances WHERE status = 'accepted' ORDER BY remote_domain"
    )
    .fetch_all(pool)
    .await
}
