use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post},
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
    encrypted_key_bundle: Option<String>,
    bundle_salt: Option<String>,
    recovery_id: Option<String>,
}

#[derive(Serialize)]
pub struct AuthResponse {
    token: String,
    user_id: Uuid,
    display_name: String,
    role: String,
}

#[derive(Deserialize)]
pub struct RecoverRequest {
    recovery_id: String,
}

#[derive(Serialize)]
pub struct RecoverResponse {
    encrypted_key_bundle: String,
    bundle_salt: String,
    display_name: String,
    public_key: String,
    encryption_public_key: Option<String>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/register", post(register))
        .route("/me", get(me))
        .route("/recover", post(recover))
        .route("/users/{id}/keys", get(get_user_keys))
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

fn decode_b64(s: &str) -> Result<Vec<u8>, (StatusCode, Json<serde_json::Value>)> {
    base64::engine::general_purpose::STANDARD.decode(s).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "invalid base64"})))
    })
}

fn encode_b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

async fn register(
    State(state): State<AppState>,
    Json(input): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<serde_json::Value>)> {
    let public_key = decode_b64(&input.public_key)?;
    let encryption_pk = input.encryption_public_key.as_ref().and_then(|k| decode_b64(k).ok());
    let bundle = input.encrypted_key_bundle.as_ref().and_then(|k| decode_b64(k).ok());
    let salt = input.bundle_salt.as_ref().and_then(|k| decode_b64(k).ok());
    let recovery = input.recovery_id.as_ref().and_then(|k| decode_b64(k).ok());

    let err = |e: sqlx::Error| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()})))
    };

    let existing = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE public_key = $1")
        .bind(&public_key)
        .fetch_optional(&state.pool)
        .await
        .map_err(err)?;

    let user_id = if let Some(id) = existing {
        sqlx::query(
            r#"UPDATE users SET display_name = $2,
               encryption_public_key = COALESCE($3, encryption_public_key),
               encrypted_key_bundle = COALESCE($4, encrypted_key_bundle),
               bundle_salt = COALESCE($5, bundle_salt),
               recovery_id = COALESCE($6, recovery_id),
               last_seen = now()
               WHERE id = $1"#
        )
        .bind(id)
        .bind(&input.display_name)
        .bind(&encryption_pk)
        .bind(&bundle)
        .bind(&salt)
        .bind(&recovery)
        .execute(&state.pool)
        .await
        .map_err(err)?;
        id
    } else {
        let id = Uuid::now_v7();
        sqlx::query(
            r#"INSERT INTO users (id, display_name, public_key, encryption_public_key,
               encrypted_key_bundle, bundle_salt, recovery_id)
               VALUES ($1, $2, $3, $4, $5, $6, $7)"#
        )
        .bind(id)
        .bind(&input.display_name)
        .bind(&public_key)
        .bind(&encryption_pk)
        .bind(&bundle)
        .bind(&salt)
        .bind(&recovery)
        .execute(&state.pool)
        .await
        .map_err(err)?;
        id
    };

    let role = sqlx::query_scalar::<_, String>("SELECT role FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&state.pool)
        .await
        .map_err(err)?;

    let token = create_token(
        &state.config.auth.jwt_secret,
        state.config.auth.token_lifetime_days,
        user_id,
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?;

    Ok(Json(AuthResponse {
        token,
        user_id,
        display_name: input.display_name,
        role,
    }))
}

async fn recover(
    State(state): State<AppState>,
    Json(input): Json<RecoverRequest>,
) -> Result<Json<RecoverResponse>, (StatusCode, Json<serde_json::Value>)> {
    let recovery_id = decode_b64(&input.recovery_id)?;

    let row = sqlx::query_as::<_, RecoverRow>(
        r#"SELECT display_name, public_key, encryption_public_key,
           encrypted_key_bundle, bundle_salt
           FROM users WHERE recovery_id = $1"#
    )
    .bind(&recovery_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?
    .ok_or((StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "no identity found for this passphrase"}))))?;

    let bundle = row.encrypted_key_bundle
        .ok_or((StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "no recovery bundle stored"}))))?;
    let salt = row.bundle_salt
        .ok_or((StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "no recovery salt stored"}))))?;

    Ok(Json(RecoverResponse {
        encrypted_key_bundle: encode_b64(&bundle),
        bundle_salt: encode_b64(&salt),
        display_name: row.display_name,
        public_key: encode_b64(&row.public_key),
        encryption_public_key: row.encryption_public_key.map(|k| encode_b64(&k)),
    }))
}

#[derive(sqlx::FromRow)]
struct RecoverRow {
    display_name: String,
    public_key: Vec<u8>,
    encryption_public_key: Option<Vec<u8>>,
    encrypted_key_bundle: Option<Vec<u8>>,
    bundle_salt: Option<Vec<u8>>,
}

async fn me(
    State(state): State<AppState>,
    request: Request,
) -> Result<Json<AuthResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = extract_user_id(&state.config.auth.jwt_secret, &request).ok_or((
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({"error": "not authenticated"})),
    ))?;

    let row = sqlx::query_as::<_, UserRow>("SELECT id, display_name, role FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?
        .ok_or((StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "user not found"}))))?;

    Ok(Json(AuthResponse {
        token: String::new(),
        user_id: row.id,
        display_name: row.display_name,
        role: row.role,
    }))
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    display_name: String,
    role: String,
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

    Ok(Json(UserKeysResponse {
        user_id: row.id,
        public_key: encode_b64(&row.public_key),
        encryption_public_key: row.encryption_public_key.map(|k| encode_b64(&k)),
    }))
}

#[derive(sqlx::FromRow)]
struct UserKeysRow {
    id: Uuid,
    public_key: Vec<u8>,
    encryption_public_key: Option<Vec<u8>>,
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

pub async fn require_superadmin(
    State(state): State<AppState>,
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

    let role = sqlx::query_scalar::<_, String>("SELECT role FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();

    if role != "superadmin" {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "superadmin access required"})),
        )
            .into_response();
    }

    request.extensions_mut().insert(AuthUser { user_id });
    next.run(request).await
}
