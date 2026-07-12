use axum::{
    extract::{Multipart, Request, State},
    http::{header, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use base64::Engine;
use chrono::{Duration, Utc};
use ed25519_dalek::{Signature, VerifyingKey};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::Instant as StdInstant;
use tokio::sync::Mutex as TokioMutex;
use uuid::Uuid;

use crate::AppState;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: i64,
    role: String,
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    display_name: String,
    public_key: String,
    challenge: String,
    signature: String,
    encryption_public_key: Option<String>,
    encrypted_key_bundle: Option<String>,
    bundle_salt: Option<String>,
    recovery_id: Option<String>,
    #[serde(default)]
    recovery_code_hash: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateProfileRequest {
    display_name: Option<String>,
    bio: Option<String>,
    profile_json: Option<serde_json::Value>,
    encryption_public_key: Option<String>,
    encrypted_key_bundle: Option<String>,
    bundle_salt: Option<String>,
    recovery_id: Option<String>,
    recovery_code_hash: Option<String>,
}

#[derive(Serialize)]
pub struct AuthResponse {
    token: String,
    user_id: Uuid,
    display_name: String,
    bio: Option<String>,
    avatar_url: Option<String>,
    role: String,
}

#[derive(Deserialize)]
pub struct RecoverRequest {
    recovery_id: String,
    #[serde(default)]
    recovery_code_hash: Option<String>,
}

#[derive(Serialize)]
pub struct RecoverResponse {
    encrypted_key_bundle: String,
    bundle_salt: String,
    display_name: String,
    public_key: String,
    encryption_public_key: Option<String>,
}

#[derive(Deserialize)]
struct ChallengeRequest {
    user_id: Uuid,
}

#[derive(Serialize)]
struct ChallengeResponse {
    challenge: String,
}

#[derive(Deserialize)]
struct VerifyChallengeRequest {
    user_id: Uuid,
    challenge: String,
    signature: String,
}

static CHALLENGES: LazyLock<TokioMutex<HashMap<Uuid, (Vec<u8>, StdInstant)>>> =
    LazyLock::new(|| TokioMutex::new(HashMap::new()));

pub fn router(state: AppState) -> Router {
    let public = Router::new()
        .route("/register", post(register))
        .route("/recover", post(recover))
        .route("/me", get(me))
        .route("/me", axum::routing::put(update_profile))
        .route("/me/avatar", post(upload_avatar))
        .route("/challenge", post(create_challenge))
        .route("/verify-challenge", post(verify_challenge));

    let protected = Router::new()
        .route("/users/{id}/keys", get(get_user_keys))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    public.merge(protected).with_state(state)
}

fn create_token(jwt_secret: &str, lifetime_days: u32, user_id: Uuid, role: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let exp = Utc::now() + Duration::days(lifetime_days as i64);
    let claims = Claims {
        sub: user_id.to_string(),
        exp: exp.timestamp(),
        role: role.to_string(),
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

pub fn encode_b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

async fn register(
    State(state): State<AppState>,
    Json(input): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<serde_json::Value>)> {
    let recent_registrations: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE created_at > now() - interval '1 hour'"
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("rate limit query failed: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"})))
    })?;

    if recent_registrations >= state.config.auth.max_registrations_per_hour as i64 {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({"error": "too many registrations, try again later"})),
        ));
    }

    let public_key = decode_b64(&input.public_key)?;
    let _challenge = decode_b64(&input.challenge)?;
    let sig_bytes = decode_b64(&input.signature)?;

    let pk: [u8; 32] = public_key.clone().try_into()
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "invalid public key"}))))?;
    let vk = VerifyingKey::from_bytes(&pk)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "invalid public key"}))))?;
    let sig = Signature::from_slice(&sig_bytes)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "invalid signature format"}))))?;

    let payload = format!("komun-register:{}", input.challenge);
    if vk.verify_strict(payload.as_bytes(), &sig).is_err() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "invalid signature — key ownership proof failed"})),
        ));
    }

    let encryption_pk = input.encryption_public_key.as_ref().and_then(|k| decode_b64(k).ok());
    let bundle = input.encrypted_key_bundle.as_ref().and_then(|k| decode_b64(k).ok());
    let salt = input.bundle_salt.as_ref().and_then(|k| decode_b64(k).ok());
    let recovery = input.recovery_id.as_ref().and_then(|k| decode_b64(k).ok());
    let recovery_code_hash_bytes = input.recovery_code_hash.as_ref().and_then(|k| decode_b64(k).ok());

    let err = |e: sqlx::Error| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()})))
    };

    let existing = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE public_key = $1")
        .bind(&public_key)
        .fetch_optional(&state.pool)
        .await
        .map_err(err)?;

    let user_id = if let Some(id) = existing {
        sqlx::query("UPDATE users SET display_name = $1, last_seen = now() WHERE id = $2")
            .bind(&input.display_name)
            .bind(id)
            .execute(&state.pool)
            .await
            .map_err(err)?;

        if let Some(ref epk_bytes) = encryption_pk {
            sqlx::query("UPDATE users SET encryption_public_key = $1 WHERE id = $2")
                .bind(epk_bytes)
                .bind(id)
                .execute(&state.pool)
                .await
                .map_err(err)?;
        }

        if bundle.is_some() || salt.is_some() || recovery.is_some() {
            sqlx::query(
                "UPDATE users SET encrypted_key_bundle = $1, bundle_salt = $2, recovery_id = $3, recovery_code_hash = $4 WHERE id = $5"
            )
            .bind(&bundle)
            .bind(&salt)
            .bind(&recovery)
            .bind(&recovery_code_hash_bytes)
            .bind(id)
            .execute(&state.pool)
            .await
            .map_err(err)?;
        }

        id
    } else {
        let id = Uuid::now_v7();
        sqlx::query(
            r#"INSERT INTO users (id, display_name, public_key, encryption_public_key,
               encrypted_key_bundle, bundle_salt, recovery_id, recovery_code_hash)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#
        )
        .bind(id)
        .bind(&input.display_name)
        .bind(&public_key)
        .bind(&encryption_pk)
        .bind(&bundle)
        .bind(&salt)
        .bind(&recovery)
        .bind(&recovery_code_hash_bytes)
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
        &role,
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?;

    Ok(Json(AuthResponse {
        token,
        user_id,
        display_name: input.display_name,
        bio: None,
        avatar_url: None,
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
           encrypted_key_bundle, bundle_salt, recovery_code_hash
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

    if let Some(ref stored_code_hash) = row.recovery_code_hash {
        let provided = input.recovery_code_hash.as_ref()
            .and_then(|h| decode_b64(h).ok())
            .ok_or((StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "recovery code required"}))))?;
        if provided != *stored_code_hash {
            return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "invalid recovery code"}))));
        }
    }

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
    recovery_code_hash: Option<Vec<u8>>,
}

async fn create_challenge(
    State(_state): State<AppState>,
    Json(input): Json<ChallengeRequest>,
) -> Result<Json<ChallengeResponse>, (StatusCode, Json<serde_json::Value>)> {
    let challenge_bytes: [u8; 32] = rand::random();
    let challenge = encode_b64(&challenge_bytes);

    let mut challenges = CHALLENGES.lock().await;
    challenges.insert(input.user_id, (challenge_bytes.to_vec(), StdInstant::now() + std::time::Duration::from_secs(300)));

    Ok(Json(ChallengeResponse { challenge }))
}

async fn verify_challenge(
    State(state): State<AppState>,
    Json(input): Json<VerifyChallengeRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let (stored, expires) = {
        let mut challenges = CHALLENGES.lock().await;
        challenges.remove(&input.user_id)
            .ok_or((StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "no challenge found"}))))?
    };

    if StdInstant::now() > expires {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "challenge expired"}))));
    }

    let challenge_bytes = decode_b64(&input.challenge)?;
    if challenge_bytes != stored {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "challenge mismatch"}))));
    }

    let sig_bytes = decode_b64(&input.signature)?;

    let public_key: Vec<u8> = sqlx::query_scalar("SELECT public_key FROM users WHERE id = $1")
        .bind(input.user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?
        .ok_or((StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "user not found"}))))?;

    let key: [u8; 32] = public_key.try_into()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "invalid public key"}))))?;

    let vk = VerifyingKey::from_bytes(&key)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "invalid public key"}))))?;

    let sig = Signature::from_slice(&sig_bytes)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "invalid signature format"}))))?;

    let verified = vk.verify_strict(&challenge_bytes, &sig).is_ok();

    Ok(Json(serde_json::json!({"verified": verified})))
}

async fn me(
    State(state): State<AppState>,
    request: Request,
) -> Result<Json<AuthResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = extract_user_id(&state.config.auth.jwt_secret, &request).ok_or((
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({"error": "not authenticated"})),
    ))?;

    let row = sqlx::query_as::<_, MeRow>(
        "SELECT id, display_name, bio, avatar_path, role FROM users WHERE id = $1"
    )
        .bind(user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?
        .ok_or((StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "user not found"}))))?;

    Ok(Json(AuthResponse {
        token: String::new(),
        user_id: row.id,
        display_name: row.display_name,
        bio: row.bio,
        avatar_url: row.avatar_path.map(|p| format!("/avatars/{}", p)),
        role: row.role,
    }))
}

async fn update_profile(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(input): Json<UpdateProfileRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    let user_id = token
        .and_then(|t| verify_token(&state.config.auth.jwt_secret, t))
        .ok_or((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "not authenticated"})),
        ))?;

    let display_name = input.display_name;
    let encryption_pk = input.encryption_public_key
        .as_ref()
        .and_then(|k| if k.is_empty() { None } else { decode_b64(k).ok() });
    let bundle = input.encrypted_key_bundle
        .as_ref()
        .and_then(|k| if k.is_empty() { None } else { decode_b64(k).ok() });
    let salt = input.bundle_salt
        .as_ref()
        .and_then(|k| if k.is_empty() { None } else { decode_b64(k).ok() });
    let recovery = input.recovery_id
        .as_ref()
        .and_then(|k| if k.is_empty() { None } else { decode_b64(k).ok() });
    let recovery_code_hash = input.recovery_code_hash
        .as_ref()
        .and_then(|k| if k.is_empty() { None } else { decode_b64(k).ok() });

    sqlx::query(
        "UPDATE users SET display_name = COALESCE($1, display_name),
         bio = COALESCE($2, bio),
         profile_json = COALESCE($3, profile_json),
         encryption_public_key = COALESCE($4, encryption_public_key),
         encrypted_key_bundle = COALESCE($5, encrypted_key_bundle),
         bundle_salt = COALESCE($6, bundle_salt),
         recovery_id = COALESCE($7, recovery_id),
         recovery_code_hash = COALESCE($8, recovery_code_hash),
         last_seen = now()
         WHERE id = $9"
    )
    .bind(&display_name)
    .bind(&input.bio)
    .bind(&input.profile_json)
    .bind(&encryption_pk)
    .bind(&bundle)
    .bind(&salt)
    .bind(&recovery)
    .bind(&recovery_code_hash)
    .bind(user_id)
    .execute(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?;

    Ok(Json(serde_json::json!({"ok": true})))
}

async fn upload_avatar(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    let user_id = token
        .and_then(|t| verify_token(&state.config.auth.jwt_secret, t))
        .ok_or((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "not authenticated"})),
        ))?;

    let recent: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM avatar_uploads WHERE user_id = $1 AND uploaded_at > now() - interval '1 hour'",
    )
    .bind(user_id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);
    if recent >= 5 {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({"error": "rate limited: 5 uploads per hour"})),
        ));
    }

    let field = multipart
        .next_field()
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "invalid multipart"}))))?
        .ok_or((StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "no file"}))))?;

    let content_type = field.content_type().unwrap_or("").to_string();
    if !matches!(
        content_type.as_str(),
        "image/png" | "image/jpeg" | "image/webp"
    ) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "only PNG, JPEG, WebP"})),
        ));
    }

    let data = field
        .bytes()
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "read error"}))))?;
    if data.len() > state.config.media.max_avatar_bytes as usize {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "file too large (max 1MB)"})),
        ));
    }

    let img = image::load_from_memory(&data)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "invalid image"}))))?;
    let img = if img.width() > 512 || img.height() > 512 {
        img.resize(512, 512, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };

    let avatar_id = Uuid::now_v7();
    let filename = format!("{}.webp", avatar_id);
    let dir = std::path::Path::new(&state.config.media.avatar_dir);
    std::fs::create_dir_all(dir).ok();
    let path = dir.join(&filename);
    img.save(&path)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "save failed"}))))?;

    sqlx::query("INSERT INTO avatar_uploads (user_id, uploaded_at) VALUES ($1, now())")
        .bind(user_id)
        .execute(&state.pool)
        .await
        .ok();

    sqlx::query("UPDATE users SET avatar_path = $1 WHERE id = $2")
        .bind(&filename)
        .bind(user_id)
        .execute(&state.pool)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "db error"})),
            )
        })?;

    let url = format!("/avatars/{}", filename);
    Ok(Json(serde_json::json!({"avatar_url": url})))
}

#[derive(sqlx::FromRow)]
struct MeRow {
    id: Uuid,
    display_name: String,
    bio: Option<String>,
    avatar_path: Option<String>,
    role: String,
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
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let jwt_secret = match std::env::var("JWT_SECRET") {
        Ok(s) => s,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "server configuration error"})))
                .into_response();
        }
    };

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

    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)")
        .bind(user_id)
        .fetch_one(&state.pool)
        .await
        .unwrap_or(false);

    if !exists {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "user not found"})),
        )
            .into_response();
    }

    let pool = state.pool.clone();
    tokio::spawn(async move {
        let _ = sqlx::query("UPDATE users SET last_seen = now() WHERE id = $1")
            .bind(user_id)
            .execute(&pool)
            .await;
    });

    request.extensions_mut().insert(AuthUser { user_id });
    next.run(request).await
}

pub async fn require_superadmin(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let jwt_secret = match std::env::var("JWT_SECRET") {
        Ok(s) => s,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "server configuration error"})))
                .into_response();
        }
    };

    let header = request.headers().get(header::AUTHORIZATION);
    let token = header.and_then(|h| h.to_str().ok()).and_then(|v| v.strip_prefix("Bearer "));
    let token = match token {
        Some(t) => t,
        None => return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "authentication required"}))).into_response(),
    };

    let claims = match decode::<Claims>(token, &DecodingKey::from_secret(jwt_secret.as_bytes()), &Validation::default()) {
        Ok(data) => data.claims,
        Err(_) => return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "invalid token"}))).into_response(),
    };

    let user_id = Uuid::parse_str(&claims.sub).unwrap_or(Uuid::nil());

    let db_role: Option<String> = sqlx::query_scalar("SELECT role FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(&state.pool)
        .await
        .unwrap_or(None);

    match db_role {
        Some(ref role) if role == "superadmin" => {}
        _ => {
            return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "superadmin access required"}))).into_response();
        }
    }

    request.extensions_mut().insert(AuthUser { user_id });
    next.run(request).await
}
