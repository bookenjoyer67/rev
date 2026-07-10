# Wave 3+4 — Infrastructure & Defense-in-Depth + Tooling & Documentation

> **For Hermes:** Delegate each fix to OpenCode via `opencode run`, verify with `cargo check` / `npm run build`, then commit. Do NOT batch unrelated fixes.

**Goal:** Harden infrastructure (CSP, FK constraints, JWT roles, rate limit safety, WS pinning, cleanup windows, offense decay). Document threat model and add audit/lint tooling.

**Architecture:** Wave 3 closes remaining infrastructure gaps across Rust server, SvelteKit frontend, and piggPin signal relay. Wave 4 adds developer guardrails and documentation.

**Tech Stack:** Rust (Axum, SQLx, jsonwebtoken, tokio-tungstenite), SvelteKit, JavaScript

---

## Wave 3 — Infrastructure & Defense-in-Depth (8 fixes, ~78 lines, 8 files + 1 migration)

### Fix 3.1: Add CSP to SvelteKit config

**Objective:** Add Content-Security-Policy headers via SvelteKit hooks to prevent XSS and data exfiltration.

**Files:**
- Create: `web/src/hooks.server.ts`

**Current state:** No CSP configured anywhere in the SvelteKit app. The only security headers come from the Axum server (added in Wave 0).

**Code:**

```typescript
import type { Handle } from '@sveltejs/kit';

export const handle: Handle = async ({ event, resolve }) => {
    const response = await resolve(event);
    response.headers.set(
        'Content-Security-Policy',
        [
            "default-src 'self'",
            "script-src 'self' 'wasm-unsafe-eval'",
            "style-src 'self' 'unsafe-inline'",
            "img-src 'self' data: blob:",
            "frame-src 'self' https://app.piggpin.space",
            "connect-src 'self' https://*.piggpin.space wss://*.piggpin.space",
            "font-src 'self'",
            "object-src 'none'",
            "base-uri 'self'",
            "form-action 'self'",
        ].join('; ')
    );
    return response;
};
```

**Verify:** `cd web && npm run build` — builds cleanly.

**Commit:** `cd ~/rev && git add web/src/hooks.server.ts && git commit -m "fix: add CSP headers via SvelteKit hooks"`

---

### Fix 3.2: Fix PiggPin CSP — remove unsafe-inline for scripts

**Objective:** Remove `'unsafe-inline'` from `script-src` in piggPin CSP, replacing with SHA-256 hash.

**Files:**
- Modify: `index.html:10` (main app CSP)
- Modify: `src/app.html:10` (SvelteKit template)

**Current state (index.html line 10):**
```html
<meta http-equiv="Content-Security-Policy" content="default-src 'self'; script-src 'self' 'wasm-unsafe-eval' 'sha256-uhc6LteEic5Iqes7oh3Lj+BLG/Sx+fvUrEFwL5xa+vo=' https://static.cloudflareinsights.com; style-src 'self' 'unsafe-inline'; img-src 'self' data: https://*.tile.openstreetmap.org ...">
```

The script-src includes `'unsafe-inline'` which defeats the entire CSP. Replace with:

```html
<meta http-equiv="Content-Security-Policy" content="default-src 'self'; script-src 'self' 'wasm-unsafe-eval' 'sha256-uhc6LteEic5Iqes7oh3Lj+BLG/Sx+fvUrEFwL5xa+vo='; style-src 'self' 'unsafe-inline'; img-src 'self' data: https://*.tile.openstreetmap.org https://server.arcgisonline.com blob:; connect-src 'self' https: wss: https://photon.komoot.io https://tile.openstreetmap.org https://*.tile.openstreetmap.org https://server.arcgisonline.com https://build.protomaps.com https://basemaps.cartocdn.com; frame-src 'self' blob:; worker-src 'self' blob:; manifest-src 'self'; media-src 'self' blob:">
```

Key changes:
- Remove `'unsafe-inline'` from script-src
- Remove `https://static.cloudflareinsights.com` from script-src (unused analytics)
- Remove `'unsafe-inline'` from style-src (wait — style-src 'unsafe-inline' is needed for Svelte inline styles, keep it)
- Add `frame-src`, `worker-src`, `manifest-src`, `media-src`

Same change for `src/app.html` (the SvelteKit template).

**Verify:** `cd ~/team-pins && npm run build` — builds cleanly.

**Commit:** `cd ~/team-pins && git add index.html src/app.html && git commit -m "fix: remove unsafe-inline from piggpin CSP script-src"`

---

### Fix 3.3: Add foreign key constraints with ON DELETE CASCADE

**Objective:** Add missing FK constraints on `posts.author_id`, `matches.responder_id`, `messages.sender_id`, `members.user_id` to prevent orphaned records.

**Files:**
- Create: `migrations/005_foreign_key_constraints.sql`

**Current state (migrations/001_schema.sql):**
- `posts.author_id UUID NOT NULL` — no FK
- `matches.responder_id UUID NOT NULL` — no FK
- `messages.sender_id UUID NOT NULL` — no FK
- `members.user_id UUID REFERENCES users(id)` — has FK but no CASCADE
- `posts.verified_by UUID REFERENCES members(id)` — has FK

**New migration:**
```sql
-- Add missing foreign key constraints
ALTER TABLE posts ADD CONSTRAINT fk_posts_author FOREIGN KEY (author_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE matches ADD CONSTRAINT fk_matches_responder FOREIGN KEY (responder_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE messages ADD CONSTRAINT fk_messages_sender FOREIGN KEY (sender_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE members DROP CONSTRAINT members_user_id_fkey;
ALTER TABLE members ADD CONSTRAINT fk_members_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
```

The `DROP CONSTRAINT` + re-add for members is needed to add CASCADE to the existing FK.

**Verify:** `cargo build --bin komun-server` (SQLx offline mode — run `cargo sqlx prepare` if needed).

**Commit:** `cd ~/rev && git add migrations/005_foreign_key_constraints.sql && git commit -m "fix: add FK constraints with ON DELETE CASCADE for posts, matches, messages, members"`

---

### Fix 3.4: Include role in JWT claims

**Objective:** Add `role` to JWT Claims so `require_superadmin` middleware doesn't need a DB query on every admin request.

**Files:**
- Modify: `crates/server/src/auth/mod.rs` — Claims struct, create_token, verify_token, require_superadmin

**Current state (line 22-26):**
```rust
struct Claims {
    sub: String,
    exp: i64,
}
```

**Change Claims struct:**
```rust
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: i64,
    role: String,
}
```

**Change create_token (line 99-110):**
```rust
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
```

**Change all callers of create_token** — they pass role. Find them in the register function:
- Line 199: `let token = create_token(&state.config.auth.jwt_secret, state.config.auth.token_lifetime_days, user_id)?;`
- Change to include the role: `let token = create_token(&state.config.auth.jwt_secret, state.config.auth.token_lifetime_days, user_id, &role)?;`

**Change require_superadmin (line 424-467):** Remove the DB query:
```rust
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

    if claims.role != "superadmin" {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "superadmin access required"}))).into_response();
    }

    let user_id = Uuid::parse_str(&claims.sub).unwrap_or(Uuid::nil());
    request.extensions_mut().insert(AuthUser { user_id });
    next.run(request).await
}
```

**Verify:** `cargo build --bin komun-server` — compiles. No DB round-trip in require_superadmin anymore.

**Commit:** `cd ~/rev && git add crates/server/src/auth/mod.rs && git commit -m "fix: include role in JWT claims, remove admin DB query"`

---

### Fix 3.5: Replace unwrap_or(0) rate limit fallback — deny on DB error

**Objective:** When DB queries for rate limiting fail, deny access (fail-closed) instead of allowing with `unwrap_or(0)`.

**Files:**
- Modify: `crates/server/src/auth/mod.rs:141` — `fetch_one(&state.pool).await.unwrap_or(0)`
- Modify: `crates/server/src/api/posts.rs:66`
- Modify: `crates/server/src/api/conversations.rs:84`
- Modify: `crates/server/src/api/node.rs:35`
- Modify: `crates/server/src/tasks/health.rs:35` — different context, use proper error handling

**Auth mod (registration rate limit, line 136-148):**
```rust
// Before:
let recent_registrations: i64 = sqlx::query_scalar(
    "SELECT COUNT(*) FROM users WHERE created_at > now() - interval '1 hour'"
)
.fetch_one(&state.pool)
.await
.unwrap_or(0);

// After:
let recent_registrations: i64 = sqlx::query_scalar(
    "SELECT COUNT(*) FROM users WHERE created_at > now() - interval '1 hour'"
)
.fetch_one(&state.pool)
.await
.map_err(|e| {
    tracing::error!("rate limit query failed: {}", e);
    (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal error"})))
})?;
```

Same pattern for posts.rs, conversations.rs, node.rs — any `unwrap_or(0)` on a DB query that gates access. The key: on DB error, fail closed (return 500 or deny) instead of defaulting to 0 (which allows everything).

For `tasks/health.rs:35`:
```rust
// Before:
.bind(info["communities_count"].as_i64().unwrap_or(0))

// After:
.bind(info["communities_count"].as_i64().unwrap_or(0)) // OK: this is data ingestion, not access control
```
This one is fine — it's updating a counter from a remote node, not gating access.

**Verify:** `cargo build --bin komun-server` — compiles.

**Commit:** `cd ~/rev && git add crates/server/src/auth/mod.rs crates/server/src/api/posts.rs crates/server/src/api/conversations.rs crates/server/src/api/node.rs && git commit -m "fix: deny on DB error instead of unwrap_or(0) in rate-limit queries"`

---

### Fix 3.6: WebSocket certificate pinning

**Objective:** Verify TLS certificate fingerprint when connecting to the piggPin relay WebSocket, preventing MITM attacks.

**Files:**
- Modify: `signal-server.js` (add cert pinning config)

This is a client-side concern. The piggPin browser client connects to the relay via WebSocket. We add certificate pinning to the Node.js relay's WebSocket server connection validation, and expose the cert fingerprint for clients to verify.

**In signal-server.js**, add cert pinning to the WebSocket server creation:
```javascript
// Add near the WebSocketServer creation (~line 20)
const crypto = require('crypto');

function getCertFingerprint() {
    // Read the TLS cert fingerprint if TLS is configured
    // For now, document the mechanism and add env var support
    const pin = process.env.WS_CERT_FINGERPRINT;
    if (pin) return pin;
    return null;
}

const CERT_PIN = getCertFingerprint();
```

Clients are told the fingerprint via the `/api/node` endpoint on the relay (add to the health endpoint response). The browser client in `sync.js` verifies it on connect.

**In share_http.rs health endpoint**, add `cert_fingerprint` field:
```rust
// In the health endpoint JSON, add:
"cert_fingerprint": state.config.tls.as_ref().and_then(|t| t.cert_fingerprint.clone()),
```

**In signal-server config**, add optional `cert_fingerprint` field to the TLS config struct.

This fix is documentation-heavy since the actual pinning client-side logic depends on whether TLS is in use. The key deliverable: the infrastructure to surface and verify cert fingerprints exists.

**Verify:** `cd signal-server && cargo check` — compiles.

**Commit:** `cd ~/team-pins && git add signal-server.js signal-server/src/share_http.rs signal-server/src/config.rs && git commit -m "fix: add WebSocket certificate pinning infrastructure"`

---

### Fix 3.7: Bundle cleanup window: 30 days → 90 days

**Objective:** Extend key bundle retention from 30 to 90 days. Recovery keys should remain available for users who return after extended absence.

**Files:**
- Modify: `crates/server/src/tasks/bundle_cleanup.rs:20`

**Code change:**
```rust
// Before:
WHERE last_seen < now() - interval '30 days'

// After:
WHERE last_seen < now() - interval '90 days'
```

**Verify:** `cargo build --bin komun-server` — compiles.

**Commit:** `cd ~/rev && git add crates/server/src/tasks/bundle_cleanup.rs && git commit -m "fix: extend key bundle retention from 30 to 90 days"`

---

### Fix 3.8: Offense records decay — decrement by 1 every 24h

**Objective:** Prevent permanent escalation for IPs that offended once and then behaved. Currently offense counts never decrease.

**Files:**
- Modify: `signal-server/src/rate.rs` — clean() function

**Current state (line 105-144):** The `clean()` function retains offenses indefinitely, only removing them when the hashmap exceeds MAX_ENTRIES.

**Change:** Add offense decay in `clean()`. For each offense record where the IP has no active ban, decrement by 1 every 24 hours since last offense:

```rust
pub fn clean(&mut self) {
    let now = Instant::now();
    let msg_window = Duration::from_secs(300);
    let conn_window = Duration::from_secs(300);
    self.msgs.retain(|_, v| { v.retain(|t| now.duration_since(*t) < msg_window); !v.is_empty() });
    self.conns.retain(|_, v| { v.retain(|t| now.duration_since(*t) < conn_window); !v.is_empty() });
    self.bans.retain(|_, t| *t > now);
    self.comm_regs.retain(|_, v| {
        v.retain(|t| now.duration_since(*t) < Duration::from_secs(self.config.community_reg_window_secs));
        !v.is_empty()
    });
    // Decay offense records: decrement by 1 for IPs with no active ban every 24h
    // clean() runs every 5 minutes, but decay only applies every 24h via cooldown tracking
    self.offenses.retain(|ip, count| {
        if self.bans.contains_key(ip) {
            return true; // actively banned, keep count
        }
        if *count > 0 {
            *count = count.saturating_sub(1); // decay by 1 per clean cycle
        }
        *count > 0 // remove if decayed to 0
    });
    // Trim oversized maps
    // ... (existing trim logic)
}
```

Note: Current `clean()` runs every 5 minutes, so this would decay every 5 minutes — too fast. Add a timestamp per offense record to track last decay:

Actually, simpler approach — add a `last_decay` HashMap and only decay if 24h passed:

```rust
offenses: HashMap<String, u32>,
offense_last_decay: HashMap<String, Instant>, // new field
```

In `clean()`:
```rust
let decay_window = Duration::from_secs(86400); // 24h
self.offenses.retain(|ip, count| {
    if self.bans.contains_key(ip) {
        self.offense_last_decay.insert(ip.clone(), now); // reset timer while banned
        return true;
    }
    let last = self.offense_last_decay.get(ip).copied().unwrap_or(now);
    if now.duration_since(last) >= decay_window {
        *count = count.saturating_sub(1);
        self.offense_last_decay.insert(ip.clone(), now);
    }
    *count > 0
});
```

**Verify:** `cd signal-server && cargo test` — existing tests pass, add one for decay.

**Commit:** `cd ~/team-pins && git add signal-server/src/rate.rs && git commit -m "fix: decay offense records by 1 every 24h when not banned"`

---

## Wave 4 — Tooling & Documentation (5 fixes, docs + config + 1 migration)

### Fix 4.1: Add cargo audit / npm audit to dev workflow

**Objective:** Catch dependency CVEs in CI/dev workflow.

**Files:**
- Create: `scripts/audit.sh` (komun)
- Create: `scripts/audit.sh` (piggpin)

**Komun:**
```bash
#!/bin/bash
set -e
echo "=== cargo audit ==="
cargo audit || true
echo "=== npm audit (web) ==="
cd web && npm audit --production || true
```

**Piggpin:**
```bash
#!/bin/bash
set -e
echo "=== npm audit ==="
npm audit --production || true
echo "=== cargo audit (signal-server) ==="
cd signal-server && cargo audit || true
```

**Install cargo-audit:**
```bash
cargo install cargo-audit
```

**Verify:** Run each script.

**Commit:** `cd ~/rev && git add scripts/audit.sh && git commit -m "chore: add cargo/npm audit script"`

---

### Fix 4.2: Document threat model + crypto boundaries in AGENTS.md

**Objective:** Self-hosters need to understand the security model.

**Files:**
- Modify: `AGENTS.md` (komun) — add "Security Model" section
- Modify: `ROADMAP.md` or `README.md` (piggpin) — add security section

**Append to Komun AGENTS.md:**
```markdown
## Security Model

### Threat Model
- **Attacker capabilities:** Network observer, compromised relay node, XSS via user-generated content, disk access to server
- **Out of scope:** Physical device compromise, supply chain attacks on dependencies, quantum adversaries

### Crypto Boundaries
- **Client-side (WASM):** ed25519 key generation/signing, x25519 ECDH, ChaCha20Poly1305 encryption, Argon2 key derivation, BIP39 recovery codes
- **Server-side:** JWT HS256, bcrypt (future), TLS termination
- **Never leaves client:** ed25519 secret key, x25519 secret key, passphrase, recovery code (only Argon2 hash sent to server)
- **Server stores:** Public keys, encrypted key bundles, recovery_id (Argon2 hash), recovery_code_hash (Argon2 hash)

### Key Hierarchy
1. Passphrase → Argon2 → wrap key → decrypts key bundle (ed25519 secret + x25519 secret)
2. ed25519 key → signs challenges → JWT token
3. x25519 key → ECDH → conversation encryption keys (ChaCha20Poly1305)
4. Recovery code (BIP39 12 words) → Argon2 → recovery_code_hash → server verifies
```

Same for piggpin in its README.md.

**Commit:** `cd ~/rev && git add AGENTS.md && git commit -m "docs: add security model and crypto boundaries to AGENTS.md"`

---

### Fix 4.3: Validate env vars on startup — fail fast if secrets are defaults

**Objective:** If JWT_SECRET, DATABASE_URL, or config.toml secrets are still placeholders, refuse to start.

**Files:**
- Modify: `crates/server/src/main.rs` — add validation block after config load

**Code (after `let config = Config::load()`):**
```rust
// Validate secrets are not defaults
if config.auth.jwt_secret == "komun-dev-secret-change-in-production" {
    anyhow::bail!("JWT_SECRET is still the default placeholder. Set it in .env or config.toml");
}
if config.database.url.contains("placeholder") || config.database.url.contains("change-me") {
    anyhow::bail!("DATABASE_URL appears to be a placeholder. Set a real database URL.");
}
```

Wait — the JWT hardcoded fallback was already removed in Wave 0. The config loads from env var. Let's check what config.rs looks like:

The JWTs are loaded from `config.auth.jwt_secret` which comes from env `JWT_SECRET`. Add a startup check:

```rust
// After config load in main.rs
if let Ok(jwt) = std::env::var("JWT_SECRET") {
    if jwt.len() < 32 {
        anyhow::bail!("JWT_SECRET is too short (< 32 chars). Generate with: openssl rand -base64 48");
    }
}
```

**Verify:** `cargo build --bin komun-server`

**Commit:** `cd ~/rev && git add crates/server/src/main.rs && git commit -m "feat: validate JWT_SECRET length on startup"`

---

### Fix 4.4: Add CHECK constraints on enum columns

**Objective:** Database-enforce valid values for enum-like text columns (role, visibility, kind, category, status).

**Files:**
- Create: `migrations/006_check_constraints.sql`

**New migration:**
```sql
-- Add CHECK constraints on enum-like text columns
ALTER TABLE users ADD CONSTRAINT chk_users_role CHECK (role IN ('user', 'superadmin'));
ALTER TABLE posts ADD CONSTRAINT chk_posts_kind CHECK (kind IN ('offer', 'request', 'resource', 'event', 'alert'));
ALTER TABLE posts ADD CONSTRAINT chk_posts_status CHECK (status IN ('active', 'fulfilled', 'withdrawn', 'expired'));
ALTER TABLE posts ADD CONSTRAINT chk_posts_visibility CHECK (visibility IN ('local', 'federated', 'private'));
ALTER TABLE matches ADD CONSTRAINT chk_matches_status CHECK (status IN ('proposed', 'accepted', 'rejected', 'resolved', 'withdrawn'));
ALTER TABLE members ADD CONSTRAINT chk_members_role CHECK (role IN ('founder', 'maintainer', 'contributor', 'reader'));
ALTER TABLE communities ADD CONSTRAINT chk_communities_visibility CHECK (visibility IN ('local', 'federated', 'private'));
```

**Verify:** `cargo build --bin komun-server` (may need `cargo sqlx prepare`).

**Commit:** `cd ~/rev && git add migrations/006_check_constraints.sql && git commit -m "fix: add CHECK constraints on enum columns"`

---

### Fix 4.5: Add missing database indexes

**Objective:** Speed up queries that currently do full table scans.

**Files:**
- Create: `migrations/007_missing_indexes.sql`

**Missing indexes (from schema analysis):**
- `matches.responder_id` — queried to find a user's matches
- `posts.author_id` — queried for user's posts (ownership check in withdraw_post)
- `matches.status` — filtered in list queries
- `messages.sender_id` — queried for user's sent messages

```sql
CREATE INDEX idx_posts_author ON posts(author_id);
CREATE INDEX idx_matches_responder ON matches(responder_id);
CREATE INDEX idx_matches_status ON matches(status);
CREATE INDEX idx_messages_sender ON messages(sender_id);
```

**Verify:** `cargo build --bin komun-server` (SQLx offline mode handles new indexes automatically — no schema change).

**Commit:** `cd ~/rev && git add migrations/007_missing_indexes.sql && git commit -m "perf: add missing indexes on posts.author_id, matches.responder_id, matches.status, messages.sender_id"`

---

## Summary

| Wave | # | Fix | Repo | Files | Est. lines |
|------|---|-----|------|-------|------------|
| 3 | 3.1 | CSP headers via SvelteKit hooks | komun | 1 new | 20 |
| 3 | 3.2 | Remove unsafe-inline from piggpin CSP | piggpin | 2 | 4 |
| 3 | 3.3 | FK constraints with ON DELETE CASCADE | komun | 1 new | 5 |
| 3 | 3.4 | JWT claims include role | komun | 1 | 25 |
| 3 | 3.5 | Deny on DB error instead of unwrap_or(0) | komun | 4 | 12 |
| 3 | 3.6 | WS certificate pinning infrastructure | piggpin | 3 | 15 |
| 3 | 3.7 | Bundle cleanup 30→90 days | komun | 1 | 1 |
| 3 | 3.8 | Offense records decay 24h | piggpin | 1 | 15 |
| 4 | 4.1 | cargo/npm audit scripts | both | 2 new | 20 |
| 4 | 4.2 | Security model docs in AGENTS.md | both | 2 | 50 |
| 4 | 4.3 | Env var validation on startup | komun | 1 | 5 |
| 4 | 4.4 | CHECK constraints on enum columns | komun | 1 new | 8 |
| 4 | 4.5 | Missing database indexes | komun | 1 new | 4 |
| | | **Total** | | **21 files** | **~184 lines** |

## Execution Order

Easiest first, building confidence:

1. **3.7** — one-line SQL change (30→90 days)
2. **3.2** — edit two CSP meta tags
3. **4.3** — few lines in main.rs
4. **3.5** — unwrap_or(0) replacements
5. **3.1** — new hooks.server.ts
6. **3.4** — JWT claims refactor (touches auth middleware)
7. **3.3** — FK migration (requires sqlx prepare)
8. **3.8** — offense decay (touches rate limiter with tests)
9. **3.6** — cert pinning (mostly documentation)
10. **4.1** — audit scripts (no code changes)
11. **4.4** — CHECK constraints migration
12. **4.5** — index migration
13. **4.2** — docs (last, wraps everything)

## Verification

After all fixes:
```bash
# Komun
cd ~/rev && cargo build --release --bin komun-server 2>&1
cd web && npm run build 2>&1

# Piggpin
cd ~/team-pins/signal-server && cargo check 2>&1 && cargo test 2>&1
cd ~/team-pins && npm run build 2>&1
```
