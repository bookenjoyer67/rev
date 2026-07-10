# Integration Verification & Auth Tests

> **For Hermes:** Delegate build tasks to OpenCode. Implement tests directly.

**Goal:** Verify all 44 security hardening changes work end-to-end, fix the `initAuth()` gap, and add auth tests to prevent regressions.

**Architecture:** Full pipeline build (WASM → frontend → server) for both projects, then targeted auth tests covering the most heavily refactored module (challenge-response, recovery codes, JWT roles, rate limits).

**Tech Stack:** Rust (Axum, SQLx, jsonwebtoken, ed25519-dalek), SvelteKit, WASM (wasm-pack)

---

## Critical Finding: initAuth() never called

`web/src/lib/stores/auth.ts` exports `initAuth()` but **nothing calls it**. The auth store initializes to `{keypair: null, servers: {}}` and never loads from storage. Every component reading auth synchronously will see empty state.

This was introduced in Wave 2 (fix 2.1 — encrypt auth localStorage). The old code initialized the store with `writable(loadFromStorage())` which ran synchronously. The new code uses `initAuth()` which is async and must be called explicitly.

**Fix required:** Call `initAuth()` from `+layout.svelte` on mount.

---

## Phase 1: Fix initAuth gap

### Task 1.1: Call initAuth from root layout

**Objective:** Wire `initAuth()` into the app lifecycle so auth loads on page load.

**Files:**
- Modify: `web/src/routes/+layout.svelte`

**Current state:** The layout file needs to be checked. If it exists, add `initAuth()` call. If it doesn't exist, create it.

**Code (in `<script>` tag):**
```typescript
import { onMount } from 'svelte';
import { initAuth } from '$lib/stores/auth';

onMount(() => {
    initAuth(); // load encrypted auth from localStorage if available
});
```

This loads auth on every page load. If the user has a passphrase set, auth will be encrypted — they'll need to call `unlockAuth(passphrase)` via the onboarding flow. If no passphrase, `initAuth()` gracefully returns empty state (matching the old behavior).

**Verify:** `cd web && npm run build` — builds cleanly.

**Commit:** `git add web/src/routes/+layout.svelte && git commit -m "fix: call initAuth on layout mount to restore auth loading"`

---

## Phase 2: Full pipeline build

### Task 2.1: Build Komun WASM

**Objective:** Verify the 2,070-line WASM changes compile and produce a valid package.

**Steps:**
```bash
cd /home/computing/rev
wasm-pack build crates/wasm --target web
```
Expected: compiles without errors, produces `crates/wasm/pkg/`.

### Task 2.2: Build Komun frontend

**Objective:** Verify all SvelteKit changes (CSP hooks, auth, crypto, service worker) build.

**Steps:**
```bash
cd /home/computing/rev/web
npm install
npm run build
```
Expected: builds cleanly, produces `build/` directory. Watch for CSP header warnings.

### Task 2.3: Build Komun server

**Objective:** Verify Rust server compiles with all security changes.

**Steps:**
```bash
cd /home/computing/rev
cargo build --release --bin komun-server
```
Expected: compiles cleanly. SQLx offline mode handles migrations.

### Task 2.4: Build Piggpin WASM + frontend

**Objective:** Verify piggpin frontend builds after CSP changes.

**Steps:**
```bash
cd /home/computing/team-pins
npm install
npm run build
```
Expected: builds cleanly. Service worker updated.

### Task 2.5: Build Piggpin signal-server

**Objective:** Verify signal-server compiles with all security changes.

**Steps:**
```bash
cd /home/computing/team-pins/signal-server
cargo build --release
```
Expected: compiles cleanly.

---

## Phase 3: Auth module tests

The auth module (`crates/server/src/auth/mod.rs`, 467 lines) was the most heavily changed — challenge-response, recovery codes, JWT role claims, rate limit error handling. Zero tests exist.

### Task 3.1: Set up test infrastructure

**Objective:** Create test skeleton with database setup.

**Files:**
- Create: `crates/server/tests/auth_tests.rs`

**Code:**
```rust
use sqlx::PgPool;
use std::sync::Arc;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use serde_json::{json, Value};

// Helper: create test database and run migrations
async fn setup_test_db() -> PgPool {
    let db_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://komun:komun@localhost:5432/komun_test".into());
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("failed to connect to test database");
    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");
    // Clean test data
    sqlx::query("DELETE FROM messages")
        .execute(&pool).await.ok();
    sqlx::query("DELETE FROM matches")
        .execute(&pool).await.ok();
    sqlx::query("DELETE FROM posts")
        .execute(&pool).await.ok();
    sqlx::query("DELETE FROM members")
        .execute(&pool).await.ok();
    sqlx::query("DELETE FROM communities")
        .execute(&pool).await.ok();
    sqlx::query("DELETE FROM users")
        .execute(&pool).await.ok();
    pool
}
```

### Task 3.2: Test JWT role claims (fix 3.4)

**Objective:** Verify JWT tokens contain role claim and require_superadmin reads it without DB query.

**Test cases:**
1. `create_token` includes `role` in claims
2. `verify_token` returns UUID + role from valid token
3. Token without `role` claim fails decode gracefully (backward compat)
4. `require_superadmin` rejects non-superadmin role
5. `require_superadmin` allows superadmin role

```rust
#[tokio::test]
async fn test_create_token_includes_role() {
    let secret = "test-secret-with-at-least-32-chars!!";
    let user_id = Uuid::now_v7();
    let token = create_token(secret, 7, user_id, "superadmin").unwrap();
    
    let claims = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    ).unwrap().claims;
    
    assert_eq!(claims.sub, user_id.to_string());
    assert_eq!(claims.role, "superadmin");
}

#[tokio::test]
async fn test_require_superadmin_rejects_user_role() {
    // Setup: register user with role "user", get token
    // Verify: request to admin endpoint returns 403
}

#[tokio::test]
async fn test_require_superadmin_allows_superadmin() {
    // Setup: register user with role "superadmin", get token
    // Verify: request to admin endpoint returns 200
}
```

### Task 3.3: Test challenge-response auth (fix 1.1)

**Objective:** Verify the challenge-response flow works end-to-end.

**Test cases:**
1. POST /auth/challenge returns a challenge string
2. POST /auth/verify-challenge with valid signature returns `verified: true`
3. POST /auth/verify-challenge with invalid signature returns `verified: false`
4. Challenge expires after timeout

### Task 3.4: Test recovery code flow (fix 1.2)

**Objective:** Verify BIP39 recovery code generation, hashing, and recovery.

**Test cases:**
1. Register with passphrase + recovery code → server stores recovery_code_hash
2. Recover with correct recovery code → returns key bundle
3. Recover with wrong recovery code → returns error
4. Register without recovery code works (backward compat)

### Task 3.5: Test rate limit denial (fix 3.5)

**Objective:** Verify DB errors return 500 instead of silently allowing.

**Test cases:**
1. Normal registration under limit → success
2. Registration over limit → 429 Too Many Requests
3. Simulated DB error → 500, not 200 (fail-closed)

### Task 3.6: Test re-registration conflict (fix 1.5)

**Objective:** Verify duplicate registration returns 409.

**Test cases:**
1. Register → 200 OK
2. Register same public key again → 409 Conflict
3. Different public key → 200 OK

---

## Phase 4: Manual smoke tests

These require a running server and browser. Run manually, document results.

### Test 4.1: Auth flow end-to-end
1. Start server: `cargo run --bin komun-server`
2. Open `http://localhost:3000`
3. Register a new account with passphrase
4. Verify recovery code is displayed
5. Log out, recover with passphrase
6. Verify JWT token has role claim (check localStorage — should be encrypted now)

### Test 4.2: Piggpin iframe with CSP
1. Navigate to a community with a map
2. Verify piggpin iframe loads (no CSP errors in console)
3. Verify postMessage communication works
4. Verify `allow-same-origin` removal doesn't break map interaction

### Test 4.3: Service worker cache whitelist
1. Open DevTools → Application → Cache Storage
2. Verify only `/api/node`, `/api/health`, `/api/communities`, `/api/directory` are cached
3. Verify `/api/posts` and `/api/conversations` are NOT cached

---

## Phase 5: Database migration verification

### Task 5.1: Verify migrations apply cleanly

```bash
docker compose up db -d
# Wait for healthy
cargo run --bin komun-server
# Check logs for migration errors
```
Expected: All 7 migrations apply in order (001 through 007).

### Task 5.2: Verify FK constraints work

```sql
-- Should fail: insert post with non-existent author
INSERT INTO posts (id, community_id, author_id, kind, category, title) 
VALUES (gen_random_uuid(), '<valid_community>', '00000000-0000-0000-0000-000000000000', 'offer', 'goods', 'test');
-- Expected: foreign key violation error
```

### Task 5.3: Verify CHECK constraints work

```sql
-- Should fail: invalid role
INSERT INTO users (id, display_name, public_key, role) 
VALUES (gen_random_uuid(), 'test', '\x00', 'hacker');
-- Expected: check constraint violation
```

---

## Summary

| Phase | Tasks | Effort |
|-------|-------|--------|
| 1: initAuth gap | 1 fix | 5 min |
| 2: Full builds | 5 builds | 15 min |
| 3: Auth tests | 6 test files | 45 min |
| 4: Smoke tests | 3 manual tests | 15 min |
| 5: DB verification | 3 verifications | 10 min |
| **Total** | **18 tasks** | **~90 min** |

## Execution Order

1. **1.1** — Fix initAuth (critical gap)
2. **2.1-2.5** — Full pipeline builds (catch compile errors)
3. **3.1** — Test infrastructure (prerequisite for all tests)
4. **3.2-3.6** — Auth tests (highest blast radius)
5. **4.1-4.3** — Manual smoke tests
6. **5.1-5.3** — DB verification

## Risks

- **Test database required:** Auth tests need a running PostgreSQL. Can use Docker: `docker compose up db -d` with a separate test database.
- **WASM build may fail:** The 2,070-line WASM changes haven't been compiled since Wave 1. wasm-pack may flag issues.
- **Frontend build may have CSP issues:** New CSP headers could break piggpin iframe. Smoke test will catch.
- **initAuth call location:** If `+layout.svelte` doesn't exist, need to create it or find alternative entry point.
