use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use base64::Engine;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: i64,
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    display_name: String,
    public_key: String,
    encryption_public_key: Option<String>,
}

#[derive(Serialize)]
pub struct AuthResponse {
    token: String,
    user_id: Uuid,
    display_name: String,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/register", post(register))
        .route("/me", axum::routing::get(me))
        .route("/users/{id}/keys", axum::routing::get(get_user_keys))
        .with_state(state)
}

fn create_token(jwt_secret: &str, lifetime_days: u32, user_id: Uuid) -> Result<String, jsonwebtoken::errors::Error> {
    let exp = Utc::now() + Duration::days(lifetime_days as i64);
    let claims = Claims {
        sub: user_id.to_string(),
        exp: exp.timestamp(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
}

pub fn verify_token(jwt_secret: &str, token: &str) -> Option<Uuid> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .ok()?;
    Uuid::parse_str(&data.claims.sub).ok()
}

async fn register(
    State(state): State<AppState>,
    Json(input): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<serde_json::Value>)> {
    let public_key = base64::engine::general_purpose::STANDARD
        .decode(&input.public_key)
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "invalid base64 public key"})),
            )
        })?;

    let encryption_pk = input.encryption_public_key.as_ref().and_then(|k| {
        base64::engine::general_purpose::STANDARD.decode(k).ok()
    });

    let existing = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM users WHERE public_key = $1"
    )
    .bind(&public_key)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;

    let user_id = if let Some(id) = existing {
        sqlx::query("UPDATE users SET display_name = $2, encryption_public_key = COALESCE($3, encryption_public_key) WHERE id = $1")
            .bind(id)
            .bind(&input.display_name)
            .bind(&encryption_pk)
            .execute(&state.pool)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": e.to_string()})),
                )
            })?;
        id
    } else {
        let id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO users (id, display_name, public_key, encryption_public_key) VALUES ($1, $2, $3, $4)"
        )
        .bind(id)
        .bind(&input.display_name)
        .bind(&public_key)
        .bind(&encryption_pk)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;
        id
    };

    let token = create_token(
        &state.config.auth.jwt_secret,
        state.config.auth.token_lifetime_days,
        user_id,
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;

    Ok(Json(AuthResponse {
        token,
        user_id,
        display_name: input.display_name,
    }))
}

async fn me(
    State(state): State<AppState>,
    request: Request,
) -> Result<Json<AuthResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = extract_user_id(&state.config.auth.jwt_secret, &request).ok_or((
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({"error": "not authenticated"})),
    ))?;

    let row = sqlx::query_as::<_, UserRow>(
        "SELECT id, display_name FROM users WHERE id = $1"
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?
    .ok_or((
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({"error": "user not found"})),
    ))?;

    Ok(Json(AuthResponse {
        token: String::new(),
        user_id: row.id,
        display_name: row.display_name,
    }))
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    display_name: String,
}

fn extract_user_id(jwt_secret: &str, request: &Request) -> Option<Uuid> {
    let header = request.headers().get(header::AUTHORIZATION)?;
    let value = header.to_str().ok()?;
    let token = value.strip_prefix("Bearer ")?;
    verify_token(jwt_secret, token)
}

#[derive(Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
}

pub async fn require_auth(
    mut request: Request,
    next: Next,
) -> Response {
    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "komun-dev-secret-change-in-production".into());

    let user_id = match extract_user_id(&jwt_secret, &request) {
        Some(id) => id,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "authentication required"})),
            )
                .into_response();
        }
    };

    request.extensions_mut().insert(AuthUser { user_id });
    next.run(request).await
}

#[derive(Serialize)]
struct UserKeysResponse {
    user_id: Uuid,
    public_key: String,
    encryption_public_key: Option<String>,
}

async fn get_user_keys(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<Json<UserKeysResponse>, (StatusCode, Json<serde_json::Value>)> {
    let row = sqlx::query_as::<_, UserKeysRow>(
        "SELECT id, public_key, encryption_public_key FROM users WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?
    .ok_or((StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "user not found"}))))?;

    use base64::Engine;
    Ok(Json(UserKeysResponse {
        user_id: row.id,
        public_key: base64::engine::general_purpose::STANDARD.encode(&row.public_key),
        encryption_public_key: row.encryption_public_key.map(|k| base64::engine::general_purpose::STANDARD.encode(&k)),
    }))
}

#[derive(sqlx::FromRow)]
struct UserKeysRow {
    id: Uuid,
    public_key: Vec<u8>,
    encryption_public_key: Option<Vec<u8>>,
}
