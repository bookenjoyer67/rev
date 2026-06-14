use std::time::Duration;
use tokio::time;

use crate::AppState;

pub async fn bundle_cleanup_loop(state: AppState) {
    time::sleep(Duration::from_secs(300)).await;

    loop {
        if let Err(e) = cleanup_bundles(&state).await {
            tracing::warn!("bundle cleanup error: {}", e);
        }
        time::sleep(Duration::from_secs(86400)).await;
    }
}

async fn cleanup_bundles(state: &AppState) -> anyhow::Result<()> {
    let result = sqlx::query(
        r#"UPDATE users SET encrypted_key_bundle = NULL, bundle_salt = NULL
           WHERE last_seen < now() - interval '90 days'
           AND encrypted_key_bundle IS NOT NULL"#
    )
    .execute(&state.pool)
    .await?;

    if result.rows_affected() > 0 {
        tracing::info!("cleaned up {} stale key bundles", result.rows_affected());
    }

    Ok(())
}
