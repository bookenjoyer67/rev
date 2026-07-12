# Komun Anonymous User Profiles — Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Give every Komun user a public-facing profile page that builds trust through activity proofs and community history — without email, phone numbers, OAuth, or any PII. Identity stays tied to the cryptographic keypair + server session.

**Architecture:** Profiles are key-based, not identity-based. A profile is the public view of `public_key` — display name, bio, community memberships, post history, verification badges, and endorsement counts. No email. No phone. No real names. Trust is derived from on-platform action, not external identity verification.

**Tech Stack:** Rust/Axum 0.7 backend, PostgreSQL (sqlx), SvelteKit 5 SPA frontend (adapter-static), Ed25519 keypair auth (already in place), WASM crypto (already in place).

---

## Current State

What already exists:
- **Auth:** Ed25519 keypair registration with optional passphrase recovery. No email/phone/OAuth.
- **Session binding:** Keys stored in `sessionStorage` — close the tab, lose the identity. JWT token returned on registration, stored in `sessionStorage` and optionally encrypted to `localStorage` if a passphrase is set.
- **Users table:** `id`, `display_name`, `public_key`, `encryption_public_key`, `encrypted_key_bundle`, `bundle_salt`, `recovery_id`, `recovery_code_hash`, `role`, `last_seen`, `created_at`
- **Posts table:** Has `verified_by UUID REFERENCES members(id)` and `verified_at` — post verification already modeled but unused in profiles
- **Members table:** Links users to communities with `role` and `joined_at`
- **Frontend routes:** Account settings page (`/account`) exists. No public profile page.

---

## Phase 1: Public Profile Page (core)

### Task 1: Add `bio` and `profile_json` columns to users table

**Objective:** Give users a bio field and an extensible profile metadata blob.

**Files:**
- Create: `migrations/011_user_profiles.sql`
- Modify: `crates/core/src/models/user.rs` (new file)

**Step 1: Write migration**

```sql
ALTER TABLE users ADD COLUMN bio TEXT;
ALTER TABLE users ADD COLUMN avatar_path TEXT;
ALTER TABLE users ADD COLUMN profile_json JSONB DEFAULT '{}'::jsonb;
```

**Step 2: Run migration, verify**

```bash
cd ~/rev && cargo run --bin komun-server  # triggers sqlx::migrate!
# Or: psql komun -c "\d users" to verify columns exist
```

**Step 3: Add User model to core crate**

```rust
// crates/core/src/models/user.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: Uuid,
    pub display_name: String,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub public_key: String,        // base64
    pub encryption_public_key: Option<String>,
    pub role: String,
    pub community_count: u32,
    pub post_count: u32,
    pub verified_post_count: u32,
    pub joined_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub profile_json: serde_json::Value,
}
```

**Step 4: Commit**

```bash
git add migrations/011_user_profiles.sql crates/core/src/models/user.rs
git commit -m "feat: add bio and profile_json to users, UserProfile model"
```

---

### Task 2: `GET /api/users/{id}` — public profile endpoint

**Objective:** Serve a user's public profile by UUID.

**Files:**
- Create: `crates/server/src/api/users.rs`
- Modify: `crates/server/src/api/mod.rs`
- Modify: `crates/server/src/db/mod.rs`
- Create: `crates/server/src/db/users.rs`

**Step 1: DB query module**

```rust
// crates/server/src/db/users.rs
use sqlx::PgPool;
use uuid::Uuid;

pub struct UserProfileRow {
    pub id: Uuid,
    pub display_name: String,
    pub bio: Option<String>,
    pub avatar_path: Option<String>,
    pub public_key: Vec<u8>,
    pub encryption_public_key: Option<Vec<u8>>,
    pub role: String,
    pub community_count: i64,
    pub post_count: i64,
    pub verified_post_count: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_seen: chrono::DateTime<chrono::Utc>,
    pub profile_json: serde_json::Value,
}

pub async fn get_profile(pool: &PgPool, user_id: Uuid) -> Result<Option<UserProfileRow>, sqlx::Error> {
    sqlx::query_as!(UserProfileRow,
        r#"SELECT
            u.id, u.display_name, u.bio, u.avatar_path, u.public_key, u.encryption_public_key,
            u.role, u.created_at, u.last_seen, u.profile_json,
            COALESCE(c.community_count, 0) as "community_count!",
            COALESCE(p.post_count, 0) as "post_count!",
            COALESCE(v.verified_count, 0) as "verified_post_count!"
        FROM users u
        LEFT JOIN LATERAL (
            SELECT COUNT(*)::bigint as community_count FROM members WHERE user_id = u.id
        ) c ON true
        LEFT JOIN LATERAL (
            SELECT COUNT(*)::bigint as post_count FROM posts WHERE author_id = u.id
        ) p ON true
        LEFT JOIN LATERAL (
            SELECT COUNT(*)::bigint as verified_count FROM posts WHERE author_id = u.id AND verified_by IS NOT NULL
        ) v ON true
        WHERE u.id = $1"#,
        user_id
    )
    .fetch_optional(pool)
    .await
}
```

**Step 2: API route handler**

```rust
// crates/server/src/api/users.rs
use axum::{
    extract::{Path, State},
    Json, Router, routing::get,
};
use crate::AppState;
use crate::auth;
use crate::db::users;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/{id}", get(profile))
        .with_state(state)
}

async fn profile(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let row = users::get_profile(&state.pool, id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?
        .ok_or((StatusCode::NOT_FOUND, Json(json!({"error": "user not found"}))))?;

    Ok(Json(json!({
        "id": row.id,
        "display_name": row.display_name,
        "bio": row.bio,
        "avatar_url": row.avatar_path.map(|p| format!("{}/avatars/{}", state.config.public_url(), p)),
        "public_key": auth::encode_b64(&row.public_key),
        "encryption_public_key": row.encryption_public_key.map(|k| auth::encode_b64(&k)),
        "role": row.role,
        "community_count": row.community_count,
        "post_count": row.post_count,
        "verified_post_count": row.verified_post_count,
        "joined_at": row.created_at,
        "last_seen": row.last_seen,
        "profile_json": row.profile_json,
    })))
}
```

**Step 3: Wire into router**

In `crates/server/src/api/mod.rs`:
```rust
mod users;
// in router():
.nest("/users", users::router(state.clone()))
```

In `crates/server/src/db/mod.rs`:
```rust
pub mod users;
```

**Step 4: Verify**

```bash
cargo check
# Start server, curl:
curl http://localhost:3000/api/users/<uuid> | jq
```

**Step 5: Commit**

```bash
git add crates/server/src/api/users.rs crates/server/src/db/users.rs crates/server/src/api/mod.rs crates/server/src/db/mod.rs
git commit -m "feat: GET /api/users/{id} public profile endpoint"
```

---

### Task 3: `PUT /api/auth/me` — extend to accept bio and profile_json

**Objective:** Let users update their own bio and profile metadata.

**Files:**
- Modify: `crates/server/src/auth/mod.rs`

**Step 1: Extend UpdateProfileRequest**

In `auth/mod.rs`, add to `UpdateProfileRequest`:
```rust
pub struct UpdateProfileRequest {
    display_name: Option<String>,
    bio: Option<String>,
    profile_json: Option<serde_json::Value>,
    // ... existing fields unchanged
}
```

**Step 2: Extend the UPDATE query in update_profile handler**

After the existing COALESCE chain, add bio and profile_json handling:
```rust
// After the existing query, add bio + profile_json update
if input.bio.is_some() || input.profile_json.is_some() {
    sqlx::query(
        "UPDATE users SET bio = COALESCE($1, bio),
         profile_json = COALESCE($2, profile_json)
         WHERE id = $3"
    )
    .bind(&input.bio)
    .bind(&input.profile_json)
    .bind(user_id)
    .execute(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;
}
```

**Step 3: Verify**

```bash
cargo check
# curl PUT with auth
```

**Step 4: Commit**

```bash
git add crates/server/src/auth/mod.rs
git commit -m "feat: bio and profile_json fields on profile update"
```

---

### Task 4: `POST /api/auth/me/avatar` — profile picture upload

**Objective:** Let users upload a profile picture. Serve avatars from a public directory.

**Files:**
- Modify: `crates/server/src/auth/mod.rs`
- Modify: `crates/server/src/main.rs` (add static file serving for avatars)
- Modify: `crates/server/src/config.rs` (add `avatar_dir` config or use `data_dir`)

**Constraints for anonymous safety:**
- Max file size: 1MB
- Allowed types: PNG, JPEG, WebP
- No EXIF metadata preserved — server strips it (use `image` crate to re-encode)
- Images stored under random filenames (UUID) — not traceable to user
- Served from `data/avatars/{uuid}.webp` under `/avatars/` path
- Rate limited: 5 uploads per hour per user

**Step 1: Add avatar_dir to config**

```rust
// In config.rs ServerConfig or a new MediaConfig
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct MediaConfig {
    pub avatar_dir: String,
    pub max_avatar_bytes: u64,
}

impl Default for MediaConfig {
    fn default() -> Self {
        Self {
            avatar_dir: "data/avatars".into(),
            max_avatar_bytes: 1_048_576, // 1MB
        }
    }
}
```

**Step 2: Add avatar upload handler in auth/mod.rs**

```rust
use axum::extract::Multipart;
use image::GenericImageView;

async fn upload_avatar(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Auth check
    let token = headers.get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    let user_id = token
        .and_then(|t| verify_token(&state.config.auth.jwt_secret, t))
        .ok_or((StatusCode::UNAUTHORIZED, Json(json!({"error": "not authenticated"}))))?;

    // Rate limit: 5 uploads/hour
    let recent: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM avatar_uploads WHERE user_id = $1 AND uploaded_at > now() - interval '1 hour'"
    )
    .bind(user_id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);
    if recent >= 5 {
        return Err((StatusCode::TOO_MANY_REQUESTS, Json(json!({"error": "rate limited: 5 uploads per hour"}))));
    }

    // Parse multipart
    let mut field = multipart.next_field().await
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "invalid multipart"}))))?
        .ok_or((StatusCode::BAD_REQUEST, Json(json!({"error": "no file"}))))?;

    let content_type = field.content_type().unwrap_or("").to_string();
    if !matches!(content_type.as_str(), "image/png" | "image/jpeg" | "image/webp") {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "only PNG, JPEG, WebP"}))));
    }

    let data = field.bytes().await
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "read error"}))))?;
    if data.len() > state.config.media.max_avatar_bytes as usize {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "file too large (max 1MB)"}))));
    }

    // Strip EXIF by re-encoding to WebP (also anonymizes)
    let img = image::load_from_memory(&data)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error": "invalid image"}))))?;
    if img.width() > 512 || img.height() > 512 {
        let img = img.resize(512, 512, image::imageops::FilterType::Lanczos3);
    }
    let avatar_id = Uuid::now_v7();
    let filename = format!("{}.webp", avatar_id);
    let dir = std::path::Path::new(&state.config.media.avatar_dir);
    std::fs::create_dir_all(dir).ok();
    let path = dir.join(&filename);
    img.save(&path)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "save failed"}))))?;

    // Track upload for rate limiting
    sqlx::query("INSERT INTO avatar_uploads (user_id, uploaded_at) VALUES ($1, now())")
        .bind(user_id)
        .execute(&state.pool)
        .await
        .ok();

    // Update user's avatar
    sqlx::query("UPDATE users SET avatar_path = $1 WHERE id = $2")
        .bind(&filename)
        .bind(user_id)
        .execute(&state.pool)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "db error"}))))?;

    let url = format!("{}/avatars/{}", state.config.public_url(), filename);
    Ok(Json(json!({"avatar_url": url})))
}
```

**Step 3: Add avatar uploads tracking table**

```sql
-- migrations/011_user_profiles.sql (extend)
CREATE TABLE avatar_uploads (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    uploaded_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_avatar_uploads_user_time ON avatar_uploads(user_id, uploaded_at DESC);
```

**Step 4: Register the route in auth router**

```rust
.route("/me/avatar", post(upload_avatar))
```

**Step 5: Serve avatar files from main.rs**

```rust
// In main.rs, add before the Router::new():
let avatar_dir = state.config.media.avatar_dir.clone();
// ...
let app = Router::new()
    .nest("/api", api::router(state.clone()))
    .nest_service("/avatars", ServeDir::new(avatar_dir))
    // ...
```

**Step 6: Verify**

```bash
cargo check
# curl -X POST -F "file=@photo.png" -H "Authorization: Bearer $TOKEN" http://localhost:3000/api/auth/me/avatar
# curl http://localhost:3000/avatars/<uuid>.webp > /dev/null
```

**Step 7: Commit**

```bash
git add crates/server/src/auth/mod.rs crates/server/src/main.rs crates/server/src/config.rs migrations/
git commit -m "feat: avatar upload with EXIF stripping, rate limited, served from /avatars/"
```

---

### Task 5: Frontend — public profile page at `/users/{id}`

**Objective:** Render a user's public profile with their stats, bio, and avatar.

**Files:**
- Create: `web/src/routes/users/[id]/+page.svelte`
- Create: `web/src/routes/users/[id]/+page.ts`

**Step 1: Page load function**

```typescript
// web/src/routes/users/[id]/+page.ts
export const ssr = false;
export async function load({ params, fetch }) {
    const server = getActiveServer(); // from $lib/stores/server
    if (!server) return { profile: null, error: 'Not connected to a server' };
    const res = await fetch(`${server}/api/users/${params.id}`);
    if (!res.ok) return { profile: null, error: 'User not found' };
    return { profile: await res.json() };
}
```

**Step 2: Page component**

Display:
- Display name (large)
- Bio (if set)
- Public key fingerprint (truncated, with copy button)
- Stats row: communities (N), posts (N), verified posts (N)
- Joined date (relative: "3 months ago")
- Last seen (relative: "2 hours ago")
- Role badge if superadmin

**Step 3: Commit**

```bash
git add web/src/routes/users/
git commit -m "feat: public user profile page at /users/{id}"
```

---

### Task 6: Link author names to profile pages throughout the app

**Objective:** Everywhere a display name appears (post cards, messages, community members), make it a clickable link to `/users/{id}`.

**Files:**
- Modify: `web/src/lib/components/AidCard.svelte`
- Modify: `web/src/routes/messages/[id]/+page.svelte`
- Modify: `web/src/routes/c/[slug]/+page.svelte` (member list if exists)

**Step 1: Audit all display-name render points**

```bash
cd ~/rev/web && grep -rn "author\|display_name\|displayName" src/ --include="*.svelte" --include="*.ts"
```

**Step 2: Wrap each in `<a href="/users/{authorId}">`**

**Step 3: Verify links work end-to-end**

**Step 4: Commit**

```bash
git add web/src/
git commit -m "feat: link author names to /users/{id} profiles"
```

---

## Phase 2: Trust Signals (verification without PII)

### Task 7: Endorsement system — users endorse other users

**Objective:** Build a web-of-trust where users can endorse each other. "Alice vouches for Bob" is visible on Bob's profile as "Endorsed by 3 people."

**Files:**
- Create: `migrations/012_endorsements.sql`
- Create: `crates/core/src/models/endorsement.rs`
- Create: `crates/server/src/db/endorsements.rs`
- Create: `crates/server/src/api/endorsements.rs`
- Modify: `crates/server/src/api/mod.rs`

**Schema:**

```sql
CREATE TABLE endorsements (
    id UUID PRIMARY KEY,
    endorser_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    endorsee_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    note TEXT,                          -- optional note: "met at food distro"
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(endorser_id, endorsee_id)   -- one endorsement per pair
);

CREATE INDEX idx_endorsements_endorsee ON endorsements(endorsee_id);
CREATE INDEX idx_endorsements_endorser ON endorsements(endorser_id);
```

**API endpoints:**
- `POST /api/users/{id}/endorse` — auth required. Returns 409 if already endorsed.
- `DELETE /api/users/{id}/endorse` — remove endorsement
- `GET /api/users/{id}/endorsements` — list endorsers (public)
- `GET /api/users/{id}/endorsements/count` — just the count

**Profile response extends:** Add `endorsement_count` and `endorsed_by_me` (if logged in).

**Step 1: Migration → model → db → api → wire**

**Step 2: Update profile endpoint to include endorsement counts**

**Step 3: Commit**

---

### Task 8: Profile badge — "Verified by Community"

**Objective:** Use the existing `verified_by` field on posts to compute a "verified contributor" badge on profiles. When a community member verifies your post, it's a trust signal.

**What already exists:**
- `posts.verified_by UUID REFERENCES members(id)` — a community member verified this post
- `posts.verified_at TIMESTAMPTZ`

**What to add:**
- A `GET /api/users/{id}/verified-posts` endpoint — list of posts verified by others
- Profile page shows: "✓ Verified contributor — 8 verified posts across 3 communities"
- Badge is computed, not set manually. If `verified_post_count > 0`, show the badge.

**Step 1: Add verified-posts list endpoint**

**Step 2: Add badge to profile page component**

**Step 3: Commit**

---

### Task 9: Cross-server identity proof (federation readiness)

**Objective:** Since identity is a keypair, the same person can exist on multiple Komun servers. Add a "same key across servers" proof so profiles can link to each other across nodes.

This is a **Phase 2 stretch goal** — depends on federation being wired up. For now, add the data model:

**Schema addition:**
```sql
ALTER TABLE users ADD COLUMN linked_servers TEXT[] DEFAULT '{}';
-- e.g. {"food-not-bombs.komun.buzz", "eastside-mutual-aid.komun.buzz"}
```

**Profile JSON field already exists** — users can manually add links to their other profiles. The `profile_json` field (JSONB) can hold `{"links": [{"url": "https://other.server/users/uuid", "label": "also on Eastside Mutual Aid"}]}`.

**Step 1: Add linked_servers column migration**

**Step 2: Document how cross-server identity works in AGENTS.md**

**Step 3: Commit**

---

## Phase 3: Frontend Polish

### Task 10: Account settings — bio, avatar, and profile editing

**Objective:** Let users set their bio, upload an avatar, and edit profile_json from the `/account` page.

**Files:**
- Modify: `web/src/routes/account/+page.svelte`
- Modify: `web/src/lib/stores/auth.ts` (add `updateBio` and `uploadAvatar` functions)

**Step 1: Add bio textarea and avatar upload to account page**

```svelte
<section class="section">
    <h2>Profile Picture</h2>
    <div class="avatar-section">
        {#if avatarUrl}
            <img src={avatarUrl} alt="Your avatar" class="avatar-preview" />
        {:else}
            <div class="avatar-placeholder">{displayName[0]?.toUpperCase()}</div>
        {/if}
        <input type="file" accept="image/png,image/jpeg,image/webp" on:change={handleAvatarUpload} />
    </div>
</section>

<section class="section">
    <h2>Bio</h2>
    <textarea bind:value={bio} maxlength="500" placeholder="Tell communities about yourself..."></textarea>
    <button on:click={saveBio}>Save Bio</button>
</section>
```

**Step 2: Add avatar upload handler**

```typescript
async function handleAvatarUpload(e: Event) {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;
    const formData = new FormData();
    formData.append('file', file);
    const token = getToken();
    const server = getActiveServer();
    const res = await fetch(`${server}/api/auth/me/avatar`, {
        method: 'POST',
        headers: { 'Authorization': `Bearer ${token}` },
        body: formData,
    });
    if (res.ok) {
        const data = await res.json();
        avatarUrl = data.avatar_url;
    }
}
```

**Step 3: Verify — upload avatar, visit `/users/{id}`, confirm it renders**

**Step 4: Commit**

---

### Task 11: Profile page styling — match Komun design system

**Objective:** Make the profile page look good with the existing theme system.

**Files:**
- Modify: `web/src/routes/users/[id]/+page.svelte` (styles)

**Design spec:**
- Large avatar (128px circle, letter placeholder if no avatar) centered at top
- Display name (large) below avatar
- Bio section below name
- Stats in a horizontal row (communities | posts | verified | endorsements)
- Public key shown as truncated hex with copy button
- Community membership list (communities they're in, linked)
- Recent posts preview (last 5 posts)

---

### Task 12: Self-profile view — "this is you" indicator

**Objective:** When a logged-in user visits their own profile page, show an edit link and a "This is your profile" banner.

**Files:**
- Modify: `web/src/routes/users/[id]/+page.svelte`
- Modify: `web/src/routes/users/[id]/+page.ts`

**Logic:**
```typescript
const myUserId = getActiveAuth()?.userId;
const isOwnProfile = myUserId === profile.id;
```

When `isOwnProfile`: show "Edit Profile" link to `/account`, and a subtle "This is you" indicator.

---

## Verification / Testing

After each phase:
```bash
cd ~/rev
cargo check                                          # Rust compiles
cargo test --workspace                               # all tests pass
fuser -k 3000/tcp && cargo run --bin komun-server &  # start server
sleep 2
curl -s http://localhost:3000/api/health | jq        # server alive
curl -s http://localhost:3000/api/users/<uuid> | jq   # profile works
```

Frontend:
```bash
cd web && npm run build   # SvelteKit static build passes
```

---

## Files Summary

| Phase | File | Action |
|-------|------|--------|
| 1 | `migrations/011_user_profiles.sql` | Create |
| 1 | `crates/core/src/models/user.rs` | Create |
| 1 | `crates/server/src/db/users.rs` | Create |
| 1 | `crates/server/src/api/users.rs` | Create |
| 1 | `crates/server/src/api/mod.rs` | Modify |
| 1 | `crates/server/src/db/mod.rs` | Modify |
| 1 | `crates/server/src/auth/mod.rs` | Modify |
| 1 | `crates/server/src/main.rs` | Modify (ServeDir for /avatars) |
| 1 | `crates/server/src/config.rs` | Modify (MediaConfig) |
| 1 | `web/src/routes/users/[id]/+page.svelte` | Create |
| 1 | `web/src/routes/users/[id]/+page.ts` | Create |
| 2 | `migrations/012_endorsements.sql` | Create |
| 2 | `crates/core/src/models/endorsement.rs` | Create |
| 2 | `crates/server/src/db/endorsements.rs` | Create |
| 2 | `crates/server/src/api/endorsements.rs` | Create |
| 3 | `web/src/routes/account/+page.svelte` | Modify |
| 3 | `web/src/lib/stores/auth.ts` | Modify |

Dependencies added (Cargo.toml):
- `image` crate (for avatar EXIF stripping + resize + WebP re-encode)
- `axum` feature `multipart` (already likely enabled)

---

## Risks / Open Questions

1. **sqlx query_as! macro** — the profile query uses LATERAL joins. `query_as!` needs a live DB connection at compile time (or `SQLX_OFFLINE=true` with prepared queries). If this breaks, fall back to `query_as::<_, UserProfileRow>` (runtime check, no compile-time macro).

2. **Public key as profile URL** — should profiles be addressable by public key fingerprint instead of UUID? UUIDs are opaque; key fingerprints are self-verifiable. Consider adding `GET /api/users/by-key/{base64_pubkey}` as an alias.

3. **Session-only identity loss** — currently if you don't set a passphrase, closing the tab loses your identity forever. The onboarding could warn more prominently: "Without a passphrase, closing this tab permanently destroys your identity." This is a feature (plausible deniability) but should be communicated clearly.

4. **Endorsement spam** — a rate limit on endorsements (e.g., 20/hour) prevents mass-endorsement attacks. Same pattern as existing `max_registrations_per_hour`.

5. **Profile privacy** — currently profiles are fully public. Should there be a visibility toggle? "public" vs "community-only" vs "hidden"? This can be Phase 4 if needed.

6. **Avatar abuse** — images are re-encoded to WebP (stripping EXIF), resized to 512px max, and rate-limited at 5/hour. The `image` crate handles arbitrary input safely, but maliciously crafted images (zip bombs disguised as PNG) could still hit the 1MB size check. The 1MB cap + `load_from_memory` (which decodes the image, defeating compression bombs) makes this low-risk.

7. **avatar_path column is nullable** — if a user deletes their avatar, set the path to NULL. Old files on disk stay (no cleanup needed; disk is cheap and the files are anonymized UUIDs).
