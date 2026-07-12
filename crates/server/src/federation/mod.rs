use std::sync::Arc;
use std::time::Duration;

use sqlx::PgPool;
use tracing::{info, warn};

use crate::db::alliances;

/// Fetch remote node info and store in directory_entries for discovery
pub async fn handshake_with_peer(
    pool: &PgPool,
    domain: &str,
) -> Result<(), String> {
    let url = format!("https://{}/api/node", domain);
    let resp = reqwest::get(&url)
        .await
        .map_err(|e| format!("fetch node info failed: {}", e))?;

    let info: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("parse node info failed: {}", e))?;

    let name = info.get("name").and_then(|v| v.as_str()).unwrap_or(domain);
    let desc = info.get("description").and_then(|v| v.as_str()).map(String::from);
    let location = info.get("location");
    let loc_name = location.and_then(|l| l.get("name")).and_then(|v| v.as_str()).map(String::from);
    let loc_lat = location.and_then(|l| l.get("lat")).and_then(|v| v.as_f64());
    let loc_lon = location.and_then(|l| l.get("lon")).and_then(|v| v.as_f64());
    let communities_count = info.get("communities_count").and_then(|v| v.as_i64()).unwrap_or(0);

    sqlx::query(
        r#"INSERT INTO directory_entries (url, name, description, location_name, location_lat, location_lon, communities_count, last_seen)
           VALUES ($1, $2, $3, $4, $5, $6, $7, now())
           ON CONFLICT (url) DO UPDATE SET
             name = EXCLUDED.name,
             description = EXCLUDED.description,
             location_name = EXCLUDED.location_name,
             location_lat = EXCLUDED.location_lat,
             location_lon = EXCLUDED.location_lon,
             communities_count = EXCLUDED.communities_count,
             last_seen = now()"#
    )
    .bind(domain)
    .bind(name)
    .bind(&desc)
    .bind(&loc_name)
    .bind(loc_lat)
    .bind(loc_lon)
    .bind(communities_count)
    .execute(pool)
    .await
    .map_err(|e| format!("insert directory entry failed: {}", e))?;

    Ok(())
}

/// Background task: sync posts from all accepted alliances
pub async fn sync_federated_posts(pool: Arc<PgPool>) {
    let accepted = match alliances::list_accepted(&pool).await {
        Ok(list) => list,
        Err(e) => {
            warn!("[federation] failed to list accepted alliances: {}", e);
            return;
        }
    };

    for alliance in &accepted {
        let domain = &alliance.remote_domain;
        info!("[federation] syncing from {}...", domain);

        // Fetch communities from remote
        let communities_url = format!("https://{}/api/communities", domain);
        let communities: Vec<serde_json::Value> = match reqwest::get(&communities_url).await {
            Ok(resp) => resp.json().await.unwrap_or_default(),
            Err(e) => {
                warn!("[federation] failed to fetch communities from {}: {}", domain, e);
                continue;
            }
        };

        for community in &communities {
            let slug = match community.get("slug").and_then(|v| v.as_str()) {
                Some(s) => s,
                None => continue,
            };

            let posts_url = format!("https://{}/api/communities/{}/posts", domain, slug);
            let posts: Vec<serde_json::Value> = match reqwest::get(&posts_url).await {
                Ok(resp) => resp.json().await.unwrap_or_default(),
                Err(e) => {
                    warn!("[federation] failed to fetch posts from {}/{}: {}", domain, slug, e);
                    continue;
                }
            };

            for post in &posts {
                let fed_id = match post.get("id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => continue,
                };

                // Skip if we already have this post
                let exists: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM posts WHERE federated_id = $1)"
                )
                .bind(fed_id)
                .fetch_one(&*pool)
                .await
                .unwrap_or(false);

                if exists {
                    continue;
                }

                let title = post.get("title").and_then(|v| v.as_str()).unwrap_or("");
                let body = post.get("body").and_then(|v| v.as_str());
                let kind = post.get("kind").and_then(|v| v.as_str()).unwrap_or("resource");
                let category = post.get("category").and_then(|v| v.as_str()).unwrap_or("other");
                let community_name = community.get("name").and_then(|v| v.as_str()).unwrap_or(slug);
                let location_name = post.get("location_name").and_then(|v| v.as_str());

                // Create a local community slug for federated posts if needed
                let local_slug = format!("{}-{}", domain.replace('.', "-"), slug);

                // Upsert a placeholder community for federated posts
                let community_id: Option<uuid::Uuid> = sqlx::query_scalar(
                    r#"INSERT INTO communities (id, slug, name, description, visibility)
                       VALUES (gen_random_uuid(), $1, $2, $3, 'federated')
                       ON CONFLICT (slug) DO UPDATE SET name = EXCLUDED.name
                       RETURNING id"#
                )
                .bind(&local_slug)
                .bind(format!("{} @ {}", community_name, domain))
                .bind(format!("Federated community from {}", domain))
                .fetch_optional(&*pool)
                .await
                .unwrap_or(None);

                if let Some(cid) = community_id {
                    let _ = sqlx::query(
                        r#"INSERT INTO posts (id, community_id, author_id, kind, category, title, body,
                           location_name, status, visibility, federated_id, origin_node)
                           VALUES (gen_random_uuid(), $1, '00000000-0000-0000-0000-000000000000', $2, $3, $4, $5, $6, 'active', 'federated', $7, $8)"#
                    )
                    .bind(cid)
                    .bind(kind)
                    .bind(category)
                    .bind(title)
                    .bind(body)
                    .bind(location_name)
                    .bind(fed_id)
                    .bind(domain)
                    .execute(&*pool)
                    .await;

                    info!("[federation] imported post '{}' from {}", title, domain);
                }
            }
        }

        let _ = alliances::update_sync_time(&pool, alliance.id).await;
    }
}

/// Start the periodic federation sync loop — spawn in background
pub fn start_sync_loop(pool: Arc<PgPool>, interval_secs: u64) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(interval_secs)).await;
            sync_federated_posts(pool.clone()).await;
        }
    });
}
