use std::time::Duration;
use tokio::time;

use crate::AppState;

pub async fn expiry_loop(state: AppState) {
    time::sleep(Duration::from_secs(120)).await;

    loop {
        if let Err(e) = expire_posts(&state).await {
            tracing::warn!("post expiry error: {}", e);
        }
        time::sleep(Duration::from_secs(3600)).await;
    }
}

async fn expire_posts(state: &AppState) -> anyhow::Result<()> {
    let result = sqlx::query(
        "UPDATE posts SET status = 'expired', updated_at = now() WHERE expires_at < now() AND status = 'active'"
    )
    .execute(&state.pool)
    .await?;

    if result.rows_affected() > 0 {
        tracing::info!("expired {} posts", result.rows_affected());
    }

    Ok(())
}
