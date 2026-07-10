use std::sync::LazyLock;
use std::time::Instant as StdInstant;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    middleware,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tokio::sync::Mutex as TokioMutex;

use crate::AppState;
use crate::auth::require_auth;
use super::communities::StatusError;

static REGISTRATIONS: LazyLock<TokioMutex<Vec<StdInstant>>> =
    LazyLock::new(|| TokioMutex::new(Vec::new()));

pub fn router(state: AppState) -> Router {
    let mut public = Router::new()
        .route("/directory", get(list_servers));

    if state.config.discovery.registration_mode == "open" {
        public = public.route("/directory/register", post(register_server));
    }

    let protected = Router::new()
        .route("/directory/{url}", delete(remove_server))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    let protected_register = if state.config.discovery.registration_mode != "open" {
        Some(Router::new()
            .route("/directory/register", post(register_server))
            .layer(middleware::from_fn_with_state(state.clone(), require_auth)))
    } else {
        None
    };

    let mut router = public.merge(protected);
    if let Some(r) = protected_register {
        router = router.merge(r);
    }
    router.with_state(state)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityLocation {
    pub slug: String,
    pub name: String,
    pub location_name: Option<String>,
    pub location_lat: Option<f64>,
    pub location_lon: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MatchedCommunity {
    pub slug: String,
    pub name: String,
    pub location_name: Option<String>,
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    url: String,
    name: String,
    description: Option<String>,
    location_name: Option<String>,
    location_lat: Option<f64>,
    location_lon: Option<f64>,
    communities_count: Option<i64>,
    version: Option<String>,
    communities: Option<Vec<CommunityLocation>>,
}

#[derive(Serialize, FromRow)]
pub struct DirectoryEntry {
    pub url: String,
    pub name: String,
    pub description: Option<String>,
    pub location_name: Option<String>,
    pub location_lat: Option<f64>,
    pub location_lon: Option<f64>,
    pub communities_count: Option<i64>,
    pub version: Option<String>,
    pub last_seen: DateTime<Utc>,
    pub registered_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct DirectoryEntryWithDistance {
    #[serde(flatten)]
    pub entry: DirectoryEntry,
    pub distance_km: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_community: Option<MatchedCommunity>,
}

#[derive(Deserialize)]
pub struct SearchParams {
    q: Option<String>,
    lat: Option<f64>,
    lon: Option<f64>,
    radius: Option<f64>,
}

async fn register_server(
    State(state): State<AppState>,
    Json(input): Json<RegisterRequest>,
) -> Result<Json<serde_json::Value>, StatusError> {
    {
        let mut registrations = REGISTRATIONS.lock().await;
        let window_start = StdInstant::now() - std::time::Duration::from_secs(3600);
        registrations.retain(|t| *t > window_start);
        if registrations.len() >= 20 {
            return Err(StatusError::with_status(
                StatusCode::TOO_MANY_REQUESTS,
                "rate limit exceeded: max 20 server registrations per hour",
            ));
        }
        registrations.push(StdInstant::now());
    }

    let url = input.url.trim_end_matches('/').to_string();

    let community_locations = input.communities.as_ref()
        .and_then(|c| if c.is_empty() { None } else { serde_json::to_value(c).ok() });

    sqlx::query(
        r#"INSERT INTO directory_entries (url, name, description, location_name, location_lat, location_lon, communities_count, version, community_locations, last_seen)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, now())
           ON CONFLICT (url) DO UPDATE SET
             name = EXCLUDED.name,
             description = EXCLUDED.description,
             location_name = EXCLUDED.location_name,
             location_lat = EXCLUDED.location_lat,
             location_lon = EXCLUDED.location_lon,
             communities_count = EXCLUDED.communities_count,
             version = EXCLUDED.version,
             community_locations = EXCLUDED.community_locations,
             last_seen = now()"#,
    )
    .bind(&url)
    .bind(&input.name)
    .bind(&input.description)
    .bind(&input.location_name)
    .bind(input.location_lat)
    .bind(input.location_lon)
    .bind(input.communities_count.unwrap_or(0))
    .bind(&input.version)
    .bind(&community_locations)
    .execute(&state.pool)
    .await?;

    Ok(Json(serde_json::json!({"status": "registered", "url": url})))
}

async fn list_servers(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<Vec<DirectoryEntryWithDistance>>, StatusError> {
    let entries = if let (Some(lat), Some(lon)) = (params.lat, params.lon) {
        let radius = params.radius.unwrap_or(50.0);

        let server_rows = sqlx::query_as::<_, DirectoryEntryWithDist>(
            r#"SELECT url, name, description, location_name, location_lat, location_lon,
               communities_count, version, last_seen, registered_at,
               NULL::text AS matched_slug,
               NULL::text AS matched_community_name,
               NULL::text AS matched_community_location_name,
               (6371 * acos(
                 LEAST(1.0, GREATEST(-1.0,
                   cos(radians($1)) * cos(radians(location_lat)) *
                   cos(radians(location_lon) - radians($2)) +
                   sin(radians($1)) * sin(radians(location_lat))
                 ))
               )) AS distance_km
               FROM directory_entries
               WHERE location_lat IS NOT NULL AND location_lon IS NOT NULL
               AND (6371 * acos(
                 LEAST(1.0, GREATEST(-1.0,
                   cos(radians($1)) * cos(radians(location_lat)) *
                   cos(radians(location_lon) - radians($2)) +
                   sin(radians($1)) * sin(radians(location_lat))
                 ))
               )) < $3
               ORDER BY distance_km
               LIMIT 20"#,
        )
        .bind(lat)
        .bind(lon)
        .bind(radius)
        .fetch_all(&state.pool)
        .await?;

        let community_rows = sqlx::query_as::<_, DirectoryEntryWithDist>(
            r#"SELECT d.url, d.name, d.description, d.location_name, d.location_lat, d.location_lon,
               d.communities_count, d.version, d.last_seen, d.registered_at,
               c.slug AS matched_slug,
               c.name AS matched_community_name,
               c.location_name AS matched_community_location_name,
               (6371 * acos(
                 LEAST(1.0, GREATEST(-1.0,
                   cos(radians($1)) * cos(radians(c.location_lat)) *
                   cos(radians(c.location_lon) - radians($2)) +
                   sin(radians($1)) * sin(radians(c.location_lat))
                 ))
               )) AS distance_km
               FROM directory_entries d
               CROSS JOIN jsonb_to_recordset(d.community_locations) AS c(
                   slug TEXT, name TEXT, location_name TEXT,
                   location_lat DOUBLE PRECISION, location_lon DOUBLE PRECISION
               )
               WHERE d.community_locations IS NOT NULL
               AND c.location_lat IS NOT NULL AND c.location_lon IS NOT NULL
               AND (6371 * acos(
                 LEAST(1.0, GREATEST(-1.0,
                   cos(radians($1)) * cos(radians(c.location_lat)) *
                   cos(radians(c.location_lon) - radians($2)) +
                   sin(radians($1)) * sin(radians(c.location_lat))
                 ))
               )) < $3
               ORDER BY distance_km
               LIMIT 20"#,
        )
        .bind(lat)
        .bind(lon)
        .bind(radius)
        .fetch_all(&state.pool)
        .await?;

        let mut seen = std::collections::HashSet::new();
        let mut entries: Vec<DirectoryEntryWithDistance> = Vec::new();
        let mut urls_with_community = std::collections::HashSet::new();

        for r in community_rows {
            let mc = MatchedCommunity {
                slug: r.matched_slug.clone().unwrap_or_default(),
                name: r.matched_community_name.clone().unwrap_or_default(),
                location_name: r.matched_community_location_name.clone(),
            };
            let key = format!("{}:{}", r.url, mc.slug);
            if seen.insert(key.clone()) {
                urls_with_community.insert(r.url.clone());
                entries.push(DirectoryEntryWithDistance {
                    entry: DirectoryEntry {
                        url: r.url,
                        name: r.name,
                        description: r.description,
                        location_name: r.location_name,
                        location_lat: r.location_lat,
                        location_lon: r.location_lon,
                        communities_count: r.communities_count,
                        version: r.version,
                        last_seen: r.last_seen,
                        registered_at: r.registered_at,
                    },
                    distance_km: r.distance_km,
                    matched_community: Some(mc),
                });
            }
        }

        for r in server_rows {
            if urls_with_community.contains(&r.url) {
                continue;
            }
            let key = r.url.clone();
            if seen.insert(key) {
                entries.push(DirectoryEntryWithDistance {
                    entry: DirectoryEntry {
                        url: r.url,
                        name: r.name,
                        description: r.description,
                        location_name: r.location_name,
                        location_lat: r.location_lat,
                        location_lon: r.location_lon,
                        communities_count: r.communities_count,
                        version: r.version,
                        last_seen: r.last_seen,
                        registered_at: r.registered_at,
                    },
                    distance_km: r.distance_km,
                    matched_community: None,
                });
            }
        }

        entries.sort_by(|a, b| {
            a.distance_km.unwrap_or(f64::MAX)
                .partial_cmp(&b.distance_km.unwrap_or(f64::MAX))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        entries.truncate(20);

        entries
    } else if let Some(ref q) = params.q {
        let pattern = format!("%{}%", q);
        let rows = sqlx::query_as::<_, DirectoryEntry>(
            r#"SELECT url, name, description, location_name, location_lat, location_lon,
               communities_count, version, last_seen, registered_at
               FROM directory_entries
               WHERE name ILIKE $1 OR location_name ILIKE $1 OR description ILIKE $1
               ORDER BY last_seen DESC
               LIMIT 20"#,
        )
        .bind(&pattern)
        .fetch_all(&state.pool)
        .await?;

        rows.into_iter().map(|e| DirectoryEntryWithDistance { entry: e, distance_km: None, matched_community: None }).collect()
    } else {
        let rows = sqlx::query_as::<_, DirectoryEntry>(
            r#"SELECT url, name, description, location_name, location_lat, location_lon,
               communities_count, version, last_seen, registered_at
               FROM directory_entries
               ORDER BY last_seen DESC
               LIMIT 20"#,
        )
        .fetch_all(&state.pool)
        .await?;

        rows.into_iter().map(|e| DirectoryEntryWithDistance { entry: e, distance_km: None, matched_community: None }).collect()
    };

    Ok(Json(entries))
}

async fn remove_server(
    State(state): State<AppState>,
    axum::extract::Path(url): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusError> {
    sqlx::query("DELETE FROM directory_entries WHERE url = $1")
        .bind(&url)
        .execute(&state.pool)
        .await?;
    Ok(Json(serde_json::json!({"status": "removed"})))
}

#[derive(FromRow)]
struct DirectoryEntryWithDist {
    url: String,
    name: String,
    description: Option<String>,
    location_name: Option<String>,
    location_lat: Option<f64>,
    location_lon: Option<f64>,
    communities_count: Option<i64>,
    version: Option<String>,
    last_seen: DateTime<Utc>,
    registered_at: DateTime<Utc>,
    distance_km: Option<f64>,
    matched_slug: Option<String>,
    matched_community_name: Option<String>,
    matched_community_location_name: Option<String>,
}
