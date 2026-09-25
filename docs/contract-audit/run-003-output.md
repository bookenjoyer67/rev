# Contract audit — `AGENTS.md` vs. the repository

Read-only pass. Every number below is printed together with the tool call that produced it. I cannot run `cargo`, `npm`, `docker` or `wasm-pack`, so claims that require executing those are flagged as unresolved rather than confirmed.

---

## 1. Critical rules → "Never commit these"

**Verdict: all five bullets hold.** `/workspace/.gitignore` in full:

```
1	target/
2	node_modules/
3	web/build/
4	web/.svelte-kit/
5	crates/wasm/pkg/
6	.env
7	.env.local
8	config.toml
9	*.swp
10	*.swo
11	dist/
12	data/avatars/
13	data/post-images/
14	.hermes/
```

- `config.toml` → `.gitignore:8` `config.toml`
- `.env` / `.env.local` → `.gitignore:6` `.env`, `.gitignore:7` `.env.local`
- `crates/wasm/pkg/` → `.gitignore:5` `crates/wasm/pkg/`
- `web/build/` → `.gitignore:3` `web/build/`
- `data/avatars/`, `data/post-images/` → `.gitignore:12` `data/avatars/`, `.gitignore:13` `data/post-images/`

Note: `crates/wasm/pkg` and `web/build` do not exist on disk right now — glob for `{.sqlx,.sqlx/**,crates/wasm/pkg,web/build}` returned `No files found`. `data/avatars/` does exist and holds untracked-but-ignored `.webp` uploads.

---

## 2. Build order

**Verdict: holds.**

- "package.json needs pkg/ to exist" → `web/package.json:33` `"komun-wasm": "file:../crates/wasm/pkg",`
- "The server does not serve the SPA … the router mounts only `/api`, `/avatars`, `/post-images`" → `crates/server/src/main.rs:123-129`, the complete `Router::new()` chain:

```
123	    let app = Router::new()
124	        .nest("/api", api::router(state.clone()))
125	        .nest_service("/avatars", ServeDir::new(&avatar_dir))
126	        .nest_service("/post-images", ServeDir::new(&post_img_dir))
127	        .layer(cors)
128	        .layer(middleware::from_fn(security_headers::security_headers))
129	        .layer(TraceLayer::new_for_http());
```

  A repo-wide search of `crates/server/src` for `nest(|nest_service(|ServeDir|.route("/` returned the full route table (printed in §5 below); the only `nest_service` / `ServeDir` uses are lines 125-126 above.
- The nginx quote is verbatim — `deploy/nginx-komun.conf:30` `    # SvelteKit static build (adapter-static) with an SPA fallback.`, backed by `deploy/nginx-komun.conf:33` `        try_files $uri $uri/ /index.html;`

---

## 3. "sqlx uses runtime queries"

**Verdict: the macro claim holds; the Docker *build/run* claim is unverifiable here.**

Repo-wide grep for `sqlx::query!|query_as!|query_scalar!|\.sqlx|sqlx-data` (whole `/workspace`):

```
No matches found
```

Glob for `.sqlx` / `.sqlx/**`: `No files found`. So there are no compile-time macros and no offline cache anywhere in the repo.

Toolchain pin confirmed — `docker/Dockerfile:5` `FROM rust:1.95-slim-bookworm AS builder`, and the `1.82` history is recorded verbatim at `docker/Dockerfile:2-3`:

```
2	# 1.82 until 2026-09-25, when an actual image build failed: `aligned 0.4.3` (via `image` → `sqlx`)
3	# needs the `edition2024` Cargo feature, which Cargo 1.82 cannot parse. bookworm matches the runtime
```

`docs/DEVELOPMENT.md:12` corroborates the workspace toolchain: `| Rust | 1.95.0 (`rustc`/`cargo`) | one Cargo workspace: `komun-core`, `komun-server`, `komun-wasm` |`.

**Unresolved:** "builds in ~2 min", "provisions a fresh database (`001` → `003`)" and "answers `/api/health`" require running `docker build`/`docker run`. I did not run them.

**Minor observation (not a contradiction):** `docker/Dockerfile:14` sets `ENV SQLX_OFFLINE=true` even though there is no offline cache — inert, but it reads as if there were one.

---

## 4. "Frontend is Svelte 5 runes only"

**Verdict: holds.** Grep over `/workspace/web/src` for `(^|\s)\$:|export let |on:(click|change|submit|input|keydown)`:

```
No matches found
```

---

## 5. Migrations frozen at 001

**Verdict: holds.** `migrations/` contains exactly:

```
migrations/001_schema.sql
migrations/002_directory_open_registration.sql
migrations/003_drop_matches_message.sql
```

`002` is additive — `migrations/002_directory_open_registration.sql:6-7`:
```
6	ALTER TABLE directory_entries
7	    ADD COLUMN open_registration BOOLEAN NOT NULL DEFAULT true;
```
The checksum-freeze rule is in the named doc: `docs/DEVELOPMENT.md:98-99` `- **`migrations/001_schema.sql` is FROZEN.** It is checksum-bookmarked in every existing` / `  database; changing one byte makes every server refuse to boot with a checksum mismatch.`

---

## 6. Crypto boundaries

**Verdict: holds on every point the repo can settle.**

- Server stores public keys + wrapped bundles only — `migrations/001_schema.sql:25-29`:
```
25	    encryption_public_key BYTEA,
26	    encrypted_key_bundle BYTEA,
27	    bundle_salt BYTEA,
28	    encrypted_recovery_bundle BYTEA,
29	    recovery_bundle_salt BYTEA,
```
  The password-derived *verifier* is what is sent: `crates/server/src/auth/mod.rs:138` `    /// `Argon2id(password, auth_salt)`, base64. The password itself is never sent.`
- No plaintext message column — `migrations/001_schema.sql:227-235`:
```
227	-- Message content is never readable by the server: ciphertext only, no plaintext body.
228	CREATE TABLE messages (
229	    id UUID PRIMARY KEY,
230	    match_id UUID NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
231	    sender_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
232	    ciphertext BYTEA NOT NULL,
233	    nonce BYTEA,
234	    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
235	);
```
  and `matches.message` (still present at `migrations/001_schema.sql:212` `    message TEXT,`) is removed by `migrations/003_drop_matches_message.sql:22` `ALTER TABLE matches DROP COLUMN message;`
- "Never log" — grep over `/workspace/crates` for logging macros carrying `verifier|password|secret|bundle|plaintext|ciphertext|recovery_code|wrap_key|token` returned 9 hits; none logs a value:
```
crates/server/src/sessions.rs:107:            Ok(n) if n > 0 => tracing::info!("session cleanup removed {n} spent one-time tokens"),
crates/server/src/sessions.rs:109:            Err(e) => tracing::warn!("one-time token cleanup failed: {e}"),
crates/server/src/auth/mod.rs:447:        tracing::warn!("could not retire previous verification tokens: {e}");
crates/server/src/auth/mod.rs:460:        tracing::error!("could not store verification token: {e}");
crates/server/src/auth/mod.rs:692:                    tracing::warn!("password rehash for {} failed: {e}", row.id);
crates/server/src/auth/mod.rs:695:            Err(e) => tracing::warn!("password rehash for {} failed: {e}", row.id),
crates/server/src/auth/mod.rs:806:        tracing::warn!("could not retire spent verification tokens: {e}");
crates/server/src/auth/mod.rs:904:                tracing::warn!("could not retire previous reset tokens: {e}");
crates/server/src/auth/mod.rs:933:                Err(e) => tracing::error!("could not store reset token: {e}");
```
- No ed25519, no JWT — grep of `Cargo.lock` for `^name = "(jsonwebtoken|ed25519|ed25519-dalek|x25519-dalek|chacha20poly1305|argon2|sqlx)"` returned only:
```
99:name = "argon2"        100-version = "0.5.3"
429:name = "chacha20poly1305"  430-version = "0.10.1"
2817:name = "sqlx"         2818-version = "0.8.0"
4054:name = "x25519-dalek" 4055-version = "2.0.1"
```
  No `jsonwebtoken` and no `ed25519*` entry in the lockfile. Remaining `JWT`/`ed25519` hits in `crates/` are removal comments, e.g. `crates/server/src/config.rs:59` `/// A2a: the signing-key setting is gone with the JWTs. Sessions are opaque database rows, so`. (The strings `'jwt'` in `web/src/tests/auth.test.ts` are fixture token values, not a JWT implementation.)

---

## 7. Code layout table

| Claim | Verdict | Evidence |
|---|---|---|
| `crates/core/` has `db_enum!` | ✅ | `crates/core/src/models/mod.rs:7` `macro_rules! db_enum {` — used at `models/user.rs:5`, `models/post.rs:5,23,32,44,52`, `models/match_thread.rs:5,14`, `models/category.rs:4` |
| enum↔CHECK test in `crates/core/src/tests.rs` | ✅ | `crates/core/src/tests.rs:15` `        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../migrations/001_schema.sql");` |
| `crates/server/` bootstrap `main.rs`, routes `api/mod.rs`, config `config.rs` | ✅ | all three files present; `main.rs:123` builds the router, `api/mod.rs:31` `pub fn router(state: AppState) -> Router {`, `config.rs` present |
| `crates/wasm/` = x25519, XChaCha20Poly1305, Argon2, recovery codes | ✅ | `crates/wasm/Cargo.toml:15-18` `x25519-dalek = { version = "2", features = ["static_secrets"] }` / `chacha20poly1305 = "0.10"` / `sha2 = "0.10"` / `argon2 = "0.5"`; `crates/wasm/src/lib.rs:4` `    XChaCha20Poly1305, XNonce,` |
| `web/` static adapter, `ssr = false` | ✅ | `web/svelte.config.js:1` `import adapter from '@sveltejs/adapter-static';`; `web/src/routes/+layout.ts:1` `export const ssr = false;` |
| **37 `fetch()` calls to `/api/` in 14 files, 34 outside `lib/api/`, `auth.ts` alone 17** | ✅ as a line-count | see below |
| `migrations/` 001 frozen + additive | ✅ | §5 |
| `docs/` prose docs | ✅ | `docs/ARCHITECTURE.md`, `CONVENTIONS.md`, `CRYPTO.md`, `DATABASE.md`, `DEVELOPMENT.md`, `DEPLOY.md` all present |
| **`docs/` quality-control artifacts** | ❌ **partly wrong** | see below |
| `deploy/` no relay/WebSocket proxy | ✅ | `deploy/nginx-komun.conf:6` `# There is no relay and no WebSocket route to proxy — the server is plain HTTP + JSON.` |
| `config.example.toml` `false` vs `config.rs` `true` | ✅ | `config.example.toml:98` `require_email_verification = false`; `crates/server/src/config.rs:158` `            require_email_verification: true,`; startup refusal at `config.rs:316-320` `if self.registration.require_email_verification && !self.email.is_configured() {` |
| `scripts/` utility scripts | ✅ | `scripts/audit.sh`, `scripts/sync-server-repo.sh` |

### The `fetch()` numbers

My first regex (`fetch\([^)]*/api/`) under-counted at 33/13 because `[^)]*` stops at the `)` inside `${getActiveServer()}`. Corrected pattern `fetch\(.*/api/` over `/workspace/web/src`:

```
web/src/lib/stores/server.ts:1
web/src/routes/map/+page.svelte:1
web/src/routes/notifications/+page.svelte:3
web/src/lib/stores/auth.ts:17
web/src/lib/stores/location.ts:1
web/src/routes/users/[id]/+page.ts:2
web/src/routes/messages/[id]/+page.svelte:1
web/src/lib/components/RespondModal.svelte:1
web/src/routes/+layout.svelte:1
web/src/routes/search/+page.svelte:2
web/src/lib/components/LinkPreview.svelte:1
web/src/lib/api/discovery.ts:2
web/src/lib/api/categories.ts:1
web/src/routes/account/+page.svelte:3

Found 37 total occurrences across 14 files.
```

37 / 14 files ✅. `lib/stores/auth.ts` = 17 ✅. Under `lib/api/`: `discovery.ts:2` + `categories.ts:1` = 3, so 37 − 3 = **34 outside `lib/api/`** ✅.

**Caveat the reader should know:** all three numbers are *line*-grep counts, and they undercount actual calls. A full `fetch\(` listing over `web/src` shows at least three more `/api` calls the pattern cannot see:
- `web/src/lib/stores/auth.ts:434-435` — a multiline call whose URL is on the next line: `				`${server}/api/auth/password-reset/bundle?token=${encodeURIComponent(input.token)}``
- `web/src/lib/api/reviews.ts:65` `	const res = await fetch(`${server}/api${path}`, {` (`/api` without trailing slash)
- `web/src/lib/api/offers.ts:72` `	const res = await fetch(`${server}/api${path}`, {`

So `auth.ts` really holds 18. The documented figures reproduce exactly, but as a grep artifact rather than a true call count.

### `docs/` quality-control artifacts — **finding**

`AGENTS.md:72` claims: ``Plus the quality-control artifacts (`prd.md`, `rubric.md`, `agent-rubric.md`, `iteration-log.md`, `clippy-report.md`, `contract-audit/`)``

Glob over the whole repo for `**/{agent-rubric.md,contract-audit,contract-audit/**,prd.md,rubric.md,iteration-log.md,clippy-report.md}`:

```
docs/clippy-gate/prd.md
docs/clippy-gate/rubric.md
docs/clippy-report.md
docs/clippy-gate/iteration-log.md
```

Repo-wide grep for `agent-rubric|contract-audit`:

```
AGENTS.md:72:| `docs/` | ARCHITECTURE, CONVENTIONS, CRYPTO, DATABASE, DEVELOPMENT, DEPLOY | Plus the quality-control artifacts (`prd.md`, `rubric.md`, `agent-rubric.md`, `iteration-log.md`, `clippy-report.md`, `contract-audit/`); keep the prose docs in sync with the code |
.claude/agents/komun-contract-auditor.md:2:name: komun-contract-auditor
.claude/agents/komun-contract-auditor.md:12:# komun-contract-auditor
```

Therefore:
- **`agent-rubric.md` does not exist anywhere in the repository.**
- **`contract-audit/` does not exist anywhere in the repository.** (The only other occurrence of the string is this auditor's own definition file.)
- `prd.md`, `rubric.md`, `iteration-log.md` exist but live in **`docs/clippy-gate/`**, not directly in `docs/`. Only `clippy-report.md` is at `docs/` top level.

---

## 8. Key architecture facts

**UUIDv7 primary keys** — ✅. `docs/DATABASE.md:3` `PostgreSQL 16. All primary keys are UUIDv7 except `avatar_uploads.id` (BIGSERIAL). The`. Grep for `now_v7|Uuid::new_v4` over `/workspace/crates` returned `Found 38 total occurrences across 11 files` — and `avatar_uploads` is indeed the BIGSERIAL exception in `001_schema.sql:315`. The AGENTS.md bullet omits that exception; `docs/DATABASE.md` states it.

**Auth / middleware** — ✅. Grep for `fn require_session|fn require_auth|fn require_admin|fn require_superadmin` over `crates/server/src`, complete output:
```
crates/server/src/auth/mod.rs:1619:pub async fn require_session(
crates/server/src/auth/mod.rs:1640:pub async fn require_auth(State(state): State<AppState>, mut request: Request, next: Next) -> Response {
crates/server/src/auth/mod.rs:1666:pub async fn require_admin(State(state): State<AppState>, mut request: Request, next: Next) -> Response {
crates/server/src/auth/mod.rs:1686:pub async fn require_superadmin(
```
"loads the role from the DB on every request" → `crates/server/src/auth/mod.rs:1664` `/// Admin *or* superadmin. The role comes from the database on this request, so a demotion that`.

**API surface** — ✅, every listed route found. Full route table from grep over `crates/server/src`:
```
crates/server/src/api/mod.rs:33:        .route("/geocode", …)
crates/server/src/api/mod.rs:62:        .route("/link-preview", …)
crates/server/src/api/health.rs:5:    Router::new().route("/health", get(health_check))
crates/server/src/api/node.rs:27:        .route("/node", get(get_node_info))
crates/server/src/api/search.rs:16:        .route("/search", get(search))
crates/server/src/api/search.rs:17:        .route("/search/users", get(search_users))
crates/server/src/api/categories.rs:36:    let public = Router::new().route("/categories", get(list_categories));
crates/server/src/api/reports.rs:16:        .route("/posts/{post_id}/report", post(report_post))
crates/server/src/api/reports.rs:18:        .route("/posts/{post_id}/hide", post(hide_post))
crates/server/src/api/reviews.rs:44:        .route("/matches/{match_id}/reviews", post(create_review))
crates/server/src/api/conversations.rs:24:        .route("/me/conversations", get(list_conversations))
crates/server/src/api/notifications.rs:15:        .route("/me/notifications", get(list_notifications))
crates/server/src/api/directory.rs:30:        .route("/directory", get(list_servers));
crates/server/src/api/admin.rs:20:        .route("/admin/stats", get(stats))
crates/server/src/auth/mod.rs:113:        .route("/me", get(me).put(update_profile))
```
(abridged to the ones AGENTS.md names; the unabridged output also covers `/posts`, `/users/{id}`, `/auth/**`, `/admin/**`, `/conversations/**`.)

Directory conditionality — ✅ `crates/server/src/api/mod.rs:64-66`:
```
64	    if state.config.discovery.directory_enabled {
65	        r = r.merge(directory::router(state));
66	    }
```
and `config.example.toml:32-34` `# Whether THIS server acts as a directory for other servers. When false the` / `# `/api/directory*` routes are not mounted at all and therefore return 404.` / `directory_enabled = false`

No `/api/alliances`, no `/api/communities` — ✅. Neither string appears as a `.route(...)` in the full route-table grep above, and `crates/server/src/api/mod.rs:21-22` records it:
```
21	// `alliances` is gone: no `mod` declaration, no route, and no file on disk. `GET /api/alliances`
22	// therefore 404s, which is the intended shape.
```

**Marketplace** — ✅.
- `listing`/`want` kinds and price fields: `migrations/001_schema.sql:160` `    CONSTRAINT chk_posts_kind CHECK (kind IN ('resource', 'need', 'offer', 'listing', 'want')),`; `:151-152` `    price_cents BIGINT,` / `    currency TEXT,`
- `match_offers` append-only **by convention** — ✅. Grep for `TRIGGER|RULE |REVOKE` over the whole `migrations/` directory returned exactly two hits, both on the posts search index:
```
migrations/001_schema.sql:185:CREATE OR REPLACE FUNCTION posts_search_update() RETURNS TRIGGER AS $$
migrations/001_schema.sql:199:CREATE TRIGGER trg_posts_search
```
  No trigger, rule or revoke touches `match_offers`.
- Review only against a completed deal — `crates/server/src/db/reviews.rs:59` `/// SPEC B4: "writable only against a completed deal". A proposal anyone can open, and a thread`
- **`categories` table, 23 rows** — ✅. `migrations/001_schema.sql:101-124` is one `INSERT … VALUES` with 23 tuples (`electronics`, `furniture`, `appliances`, `tools`, `clothing`, `bikes-vehicles`, `books-media`, `garden-outdoors`, `sports`, `toys-games`, `baby-kids`, `building-materials`, `art-craft`, `household`, `free`, `services`, `food`, `health`, `education`, `legal`, `other`, `shelter`, `transport`), ending `migrations/001_schema.sql:124` `    ('transport',          'Transport',               'aid',    230);`. It is a table with a CHECK on `scope`, not an enum: `:96` `    CONSTRAINT chk_categories_scope CHECK (scope IN ('aid', 'market', 'both'))`

**Config loading / migrations on startup / `default_currency`** — ✅.
- `crates/server/src/config.rs:291` `        let config_path = std::env::var("KOMUN_CONFIG")`
- env overrides at `config.rs:354,357,360,363,366` (`KOMUN_BIND_ADDRESS`, `KOMUN_PORT`, `DATABASE_URL`, `KOMUN_NODE_NAME`, `BIND_ADDR`)
- `crates/server/src/main.rs:78` `    sqlx::migrate!("../../migrations")`
- optional and unset: `crates/server/src/config.rs:180` `    pub default_currency: Option<String>,`; `config.example.toml:129` `# default_currency = "USD"` (commented out)

**Background tasks: four, two spawned conditionally** — ✅. `crates/server/src/tasks/mod.rs:8-30` in full:
```
 8	pub fn spawn_background_tasks(state: AppState) {
 9	    let config = &state.config;
10	
11	    if config.discovery.listed && config.discovery.directory_url.is_some() {
12	        let s = state.clone();
13	        tokio::spawn(registration::registration_loop(s));
14	    }
15	
16	    if config.discovery.directory_enabled {
17	        let s = state.clone();
18	        tokio::spawn(health::health_check_loop(s));
19	    }
20	
21	    {
22	        let s = state.clone();
23	        tokio::spawn(expiry::expiry_loop(s));
24	    }
25	
26	    {
27	        let s = state.clone();
28	        tokio::spawn(bundle_cleanup::user_cleanup_loop(s));
29	    }
30	}
```

**REPL on a terminal, `help`** — ✅. `crates/server/src/main.rs:150-151`:
```
150	    if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
151	        repl::run_repl(state).await;
```
and `crates/server/src/repl.rs:26` `            "help" | "?" => print_help(),`

**Service worker + PWA standalone** — ✅. `web/src/service-worker.ts:43` `	const CACHEABLE_API_PATHS = ['/api/node', '/api/health', '/api/posts', '/api/directory'];`; asset precache at `:14-15` `		caches.open(CACHE_NAME)` / `			.then((cache) => cache.addAll(ASSETS))`; `web/static/manifest.json:6` `	"display": "standalone",`

---

## 9. Tests section

**Rust counts — reproduce exactly (as `#[test]`/`#[tokio::test]` attribute counts).**

`crates/core`:
```
crates/core/src/tests.rs:20

Found 20 total occurrences across 1 file.
```

`crates/server`:
```
crates/server/src/sessions.rs:6
crates/server/src/rate_limit.rs:8
crates/server/src/tests/market.rs:80
crates/server/src/tests/mod.rs:17
crates/server/src/db/posts.rs:3
crates/server/src/auth/mod.rs:6
crates/server/src/auth/email.rs:7
crates/server/src/api/geocode/mod.rs:8
crates/server/src/api/geocode/limiter.rs:1
crates/server/src/api/geocode/cache.rs:2

Found 138 total occurrences across 10 files.
```

20 + 138 = **158**, matching "158 passed … (20 in `komun-core`, 138 in `komun-server`)". `crates/wasm` returned `Found 0 total occurrences across 0 files.`

**Vitest — reproduces exactly.** Grep for `^\s*(it|test)\(` over `web/src` restricted to `*.test.ts`:
```
web/src/tests/deals.test.ts:16
web/src/tests/AidCard.test.ts:13
web/src/tests/market.test.ts:14
web/src/tests/auth.test.ts:13
web/src/tests/locationMap.test.ts:4
web/src/tests/crypto.test.ts:19
web/src/routes/map/map.test.ts:3

Found 82 total occurrences across 7 files.
```
**82 tests in 7 files** ✅.

**`cargo clippy` "zero warnings" / "only line is a future-incompat note about `sqlx-postgres`"** — I cannot run cargo, so I did not verify this. The repo *records* it: `docs/clippy-report.md:71` `warning: the following packages contain code that will be rejected by a future version of Rust: sqlx-postgres v0.8.0` and `:62` `**Proceed.** The documented lint gate passes at zero warnings across `komun-core`, `komun-server``. That is a documentation artifact, not an execution.

**"three zero-test suites"** — not settled. `crates/wasm` has zero `#[test]`s (output above), which accounts for one. Which other two targets `cargo test --workspace` enumerates (doc-test targets, most likely) cannot be determined without running cargo.

**`npm run check` / `npm run build` results** — not verified; both require running npm. Note `web/package.json:5-10` has no `test` script, consistent with AGENTS.md invoking `npx vitest run` directly.

**"The enum↔CHECK agreement test reads `migrations/001_schema.sql` at test time"** — ✅ `crates/core/src/tests.rs:11` `    // read out of migrations/001_schema.sql at test time — never copied here.` and `:15` shown in §7.

---

## 10. Security model

**Verdict: holds.**
- Second, independent Argon2id over the verifier — `crates/server/src/auth/password.rs:4-5`:
```
4	//! `verifier = Argon2id(password, auth_salt)` and sends *that*; this module puts a second,
5	//! independent Argon2id over the verifier before it is stored, so a database dump alone does not
```
  Params at `password.rs:23` `    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)`
- SHA-256 session-token hashing — `crates/server/src/sessions.rs:13` `use sha2::{Digest, Sha256};`, `:47-48` `pub fn hash_token(raw: &str) -> Vec<u8> {` / `    let mut hasher = Sha256::new();`
- Per-process salt pepper — `crates/server/src/sessions.rs:115` `/// A per-deployment pepper, generated at startup.`, `:120` `pub fn generate_pepper() -> Vec<u8> {`, wired at `crates/server/src/main.rs:89` `        salt_pepper: Arc::new(sessions::generate_pepper()),`
- TLS at a proxy — `deploy/nginx-komun.conf` terminates and proxies to `        proxy_pass http://127.0.0.1:3000;` (lines 14, 21, 26).

The "honest limitation" paragraph is a threat-model statement, not a repository fact; nothing contradicts it.

---

## 11. Preamble claims ("What this is")

- PostgreSQL 16 — ✅ `docker-compose.yml:3` `    image: postgres:16-alpine`; `docs/DATABASE.md:3` `PostgreSQL 16. …`
- AGPL-3.0 — ✅ `Cargo.toml:8` `license = "AGPL-3.0-or-later"`; `LICENSE:1` `GNU AFFERO GENERAL PUBLIC LICENSE — for intercommunal software.`
- No federation / no relay crate — ✅ workspace is exactly three members, `Cargo.toml:2` `members = ["crates/server", "crates/core", "crates/wasm"]`, and glob for `**/Cargo.toml` found only `crates/core`, `crates/server`, `crates/wasm` + root. Grep for `relay|piggpin|federation` over `/workspace/crates` (case-insensitive) returned 7 hits, all either SMTP-relay code (`crates/server/src/auth/email.rs:57,59,61`) or removal/prose comments (`crates/core/src/models/match_thread.rs:46`, `crates/server/src/db/mod.rs:1`, `crates/server/src/auth/mod.rs:441`).
- No payment rails — not directly verified; the schema carries `price_cents`/`currency` bookkeeping only (`001_schema.sql:151-152, 214-215, 246`) and no payment-processor dependency appears in the `Cargo.lock` grep. I did not run an exhaustive search for payment SDKs.

---

## Findings summary

**Wrong / unsupported in `AGENTS.md`:**

1. **`AGENTS.md:72` — `agent-rubric.md` does not exist** anywhere in the repository. Repo-wide grep for the string found it only inside `AGENTS.md` itself.
2. **`AGENTS.md:72` — `contract-audit/` does not exist** anywhere in the repository. Same grep; the only other hit is `.claude/agents/komun-contract-auditor.md`, an unrelated agent definition.
3. **`AGENTS.md:72` — wrong directory for three artifacts.** `prd.md`, `rubric.md` and `iteration-log.md` are listed as `docs/` contents but live in **`docs/clippy-gate/`**. Only `clippy-report.md` is at `docs/` top level.

**Accurate but fragile / worth knowing:**

4. **`AGENTS.md:70` — the `fetch()` figures are line-grep artifacts.** 37/14/34/17 all reproduce exactly with `grep -E 'fetch\(.*/api/'`, but the real call count is higher: `lib/stores/auth.ts:434-435` is a multiline call the pattern misses (so `auth.ts` holds 18, not 17), and `lib/api/reviews.ts:65` and `lib/api/offers.ts:72` both use `` `${server}/api${path}` `` without a trailing slash.
5. **`AGENTS.md:93` — "UUIDv7 primary keys"** omits the exception that `docs/DATABASE.md:3` states: `avatar_uploads.id` is `BIGSERIAL` (`migrations/001_schema.sql:315`).

**Could not resolve (read-only agent; running these is out of scope):**

6. Docker build time (~2 min), the `001 → 003` provisioning run, and the live `/api/health` response — require `docker build` / `docker run`.
7. `cargo test --workspace` actually passing 158/0/0, and the "three zero-test suites" breakdown — only one zero-test target (`crates/wasm`, 0 `#[test]`) is visible statically.
8. `cargo clippy --release -- -D warnings` exiting 0 with the single `sqlx-postgres` future-incompat line — recorded in `docs/clippy-report.md:62,71` but not executed here.
9. `npm run check` (0 errors / 0 warnings) and `npm run build` green — require npm.
10. "There are no payment rails" — no payment dependency surfaced, but I did not run an exhaustive search for payment-processor code.

**Adjacent inconsistency (outside the audited sections, flagged because it will mislead an agent):** two root-level docs still describe the pre-reshape architecture and contradict `AGENTS.md:11-12`. `agent-summary.md:3` `Federated mutual aid discovery platform: communities post needs/offers/resources and match via encrypted conversations.` and `agent-summary.md:11` `│   ├── core/                # Shared data models (Community, Member, Post, MatchThread, User)`; `setup.md:386` references `community::Community`. `AGENTS.md` does not mention `agent-summary.md`, `setup.md`, `session_tasks.md` or `.dispatch/` at all.
