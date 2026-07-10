use axum::{
    extract::Query,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::time::Duration;

#[derive(Deserialize)]
pub struct GeocodeParams {
    q: String,
}

pub async fn geocode(
    Query(params): Query<GeocodeParams>,
) -> Result<Json<serde_json::Value>, GeocodeError> {
    let query = params.q.trim().to_string();
    if query.is_empty() {
        return Err(GeocodeError {
            status: axum::http::StatusCode::BAD_REQUEST,
            message: "q parameter is required".into(),
        });
    }

    let client = reqwest::Client::new();
    let res = client
        .get("https://nominatim.openstreetmap.org/search")
        .header("User-Agent", "Komun/0.1 (nominatim proxy; mutual-aid app)")
        .query(&[("q", query.as_str()), ("format", "json"), ("limit", "1")])
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| GeocodeError {
            status: axum::http::StatusCode::BAD_GATEWAY,
            message: format!("geocoding service unreachable: {}", e),
        })?;

    if !res.status().is_success() {
        return Err(GeocodeError {
            status: axum::http::StatusCode::BAD_GATEWAY,
            message: format!("geocoding service returned {}", res.status()),
        });
    }

    let results: Vec<serde_json::Value> = res.json().await.map_err(|e| GeocodeError {
        status: axum::http::StatusCode::BAD_GATEWAY,
        message: format!("failed to parse geocoding response: {}", e),
    })?;

    match results.first() {
        Some(r) => Ok(Json(serde_json::json!({
            "lat": r["lat"],
            "lon": r["lon"],
            "display_name": r["display_name"],
        }))),
        None => Err(GeocodeError {
            status: axum::http::StatusCode::NOT_FOUND,
            message: "location not found".into(),
        }),
    }
}

pub(crate) struct GeocodeError {
    status: axum::http::StatusCode,
    message: String,
}

impl IntoResponse for GeocodeError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(serde_json::json!({ "error": self.message })),
        )
            .into_response()
    }
}
