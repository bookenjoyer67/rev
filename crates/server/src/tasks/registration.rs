use std::time::Duration;
use tokio::time;

use crate::AppState;

pub async fn registration_loop(state: AppState) {
    loop {
        if let Err(e) = register_with_directory(&state).await {
            tracing::warn!("directory registration failed: {}", e);
        }
        time::sleep(Duration::from_secs(3600)).await;
    }
}

async fn register_with_directory(state: &AppState) -> anyhow::Result<()> {
    let config = &state.config;
    let directory_url = config.discovery.directory_url.as_ref()
        .ok_or_else(|| anyhow::anyhow!("no directory_url configured"))?;

    let communities_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM communities")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);

    let communities = sqlx::query_as::<_, CommunityLoc>(
        r#"SELECT slug, name, location_name, location_lat, location_lon
           FROM communities
           WHERE location_lat IS NOT NULL AND location_lon IS NOT NULL"#,
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let communities_json: Vec<serde_json::Value> = communities
        .into_iter()
        .map(|c| serde_json::json!({
            "slug": c.slug,
            "name": c.name,
            "location_name": c.location_name,
            "location_lat": c.location_lat,
            "location_lon": c.location_lon,
        }))
        .collect();

    let payload = serde_json::json!({
        "url": config.public_url(),
        "name": config.node.name,
        "description": config.node.description,
        "location_name": config.node.location_name,
        "location_lat": config.node.location_lat,
        "location_lon": config.node.location_lon,
        "communities_count": communities_count,
        "version": env!("CARGO_PKG_VERSION"),
        "communities": communities_json,
    });

    let client = reqwest::Client::new();
    let url = format!("{}/api/directory/register", directory_url.trim_end_matches('/'));

    let res = client.post(&url)
        .json(&payload)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;

    if res.status().is_success() {
        tracing::info!("registered with directory at {}", directory_url);
    } else {
        tracing::warn!("directory registration returned {}", res.status());
    }

    Ok(())
}

#[derive(sqlx::FromRow)]
struct CommunityLoc {
    slug: String,
    name: String,
    location_name: Option<String>,
    location_lat: Option<f64>,
    location_lon: Option<f64>,
}
