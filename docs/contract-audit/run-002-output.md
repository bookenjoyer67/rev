# AGENTS.md Contract Audit — `/workspace`

Read: `AGENTS.md` (125 lines). Below, every verdict cites the artifact that settles it.

---

## 1. Critical rules → "Never commit these"

**VERIFIED — all four entries are gitignored, and none is present in the working tree.**

`/workspace/.gitignore` in full (13 lines):
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
12	data/relay/
13	.hermes/
```
- `config.toml` → `.gitignore:8`. Glob `config*.toml` returns only `config.example.toml`.
- `.env` / `.env.local` → `.gitignore:6`, `.gitignore:7`. Glob `{.env,.env.local}`: "No files found".
- `crates/wasm/pkg/` → `.gitignore:5`. Glob `crates/wasm/pkg/*`: "No files found".
- `web/build/` → `.gitignore:3`. Glob `web/build/*`: "No files found".

Side note on the same file: `.gitignore:12  data/relay/` is live relay residue (see §11).

---

## 2. Build order

**PARTLY CONTRADICTED.** Steps 1→2 hold; the justification for step 3 does not.

- Step 2 dependency on step 1 — **verified**. `web/package.json:33`:
  ```
  "komun-wasm": "file:../crates/wasm/pkg",
  ```
- "the frontend before **the server can serve static files**" — **contradicted**. A whole-repo search of `crates/` for static-file serving (`web/build|ServeDir|fallback_service|include_dir|rust-embed`) returns exactly three hits, complete output:
  ```
  crates/server/src/main.rs:22:    services::ServeDir,
  crates/server/src/main.rs:125:        .nest_service("/avatars", ServeDir::new(&avatar_dir))
  crates/server/src/main.rs:126:        .nest_service("/post-images", ServeDir::new(&post_img_dir))
  ```
  The router (`main.rs:123-129`) mounts `/api`, `/avatars`, `/post-images` and nothing else. The SPA is served by nginx, per `deploy/nginx-komun.conf:30-34`:
  ```
      # SvelteKit static build (adapter-static) with an SPA fallback.
      location / {
          root /opt/komun/frontend;
          try_files $uri $uri/ /index.html;
      }
  ```
  `docker/Dockerfile` likewise never copies `web/` — it copies `Cargo.toml Cargo.lock`, `crates/`, `migrations/` (lines 6-8) and the built binary + `migrations/` (lines 20-21).
- Binary name `komun-server` — **verified**, `crates/server/Cargo.toml:2  name = "komun-server"`.

---

## 3. "sqlx uses runtime queries"

**VERIFIED.** Repo-wide grep for `sqlx::query!|sqlx::query_as!|query_scalar!|query_file` → "No matches found". Glob `.sqlx/**` → "No files found". Runtime API confirmed in `crates/server/Cargo.toml:21` (`features = [... "migrate"]`, no `macros`).

"Docker builds work as-is" is consistent: `docker/Dockerfile:10-11`
```
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin komun-server
```
(the flag is inert with no macros and no cache).

---

## 4. "Frontend is Svelte 5 runes only"

**VERIFIED.** Whole-of-`web/src` grep for `on:[a-zA-Z]+[=|]|export let|\$:` → **"No matches found"**. Positive control: `$state|$derived|$props|$effect` → **255 occurrences across 33 files**. Svelte version: `web/package.json:20  "svelte": "^5.0.0"`.

---

## 5. "Migrations are frozen at 001"

**VERIFIED.** `migrations/` contains exactly two files: `001_schema.sql`, `002_directory_open_registration.sql`. The additive one, `migrations/002_directory_open_registration.sql:3-7`:
```
-- Additive on purpose: 001_schema.sql is checksum-bookmarked in `_sqlx_migrations` (it was
-- hand-loaded), so it must not change. ...
ALTER TABLE directory_entries
    ADD COLUMN open_registration BOOLEAN NOT NULL DEFAULT true;
```
Checksum enforcement is sqlx's, invoked at `crates/server/src/main.rs:78  sqlx::migrate!("../../migrations")`. The cross-reference resolves: `docs/DEVELOPMENT.md:98` — `- **`migrations/001_schema.sql` is FROZEN.** It is checksum-bookmarked in every existing`.

---

## 6. Crypto boundaries

| Claim | Verdict |
|---|---|
| server stores public keys + wrapped bundles only | **VERIFIED.** `migrations/001_schema.sql:25-29`: `encryption_public_key BYTEA,` / `encrypted_key_bundle BYTEA,` / `bundle_salt BYTEA,` / `encrypted_recovery_bundle BYTEA,` / `recovery_bundle_salt BYTEA`. Wire types match (`auth/mod.rs:147-153`). |
| x25519 secret never leaves the client | **VERIFIED** as far as the wire types go — `crates/server/src/auth/mod.rs:148`: `/// x25519 secret wrapped under `wrap_key = Argon2id(password, bundle_salt)`.` |
| **"the password-derived key … never leave[s] the client"** | **IMPRECISE.** Two password-derived values exist. The *wrapping* key stays client-side, but a password-derived *verifier* is transmitted: `crates/server/src/auth/mod.rs:138-139` — `/// `Argon2id(password, auth_salt)`, base64. The password itself is never sent.` / `verifier: String,` (also `SigninRequest.verifier`, line 161). As written, the claim reads as covering both. |
| "no plaintext message column (`messages.ciphertext` only)" | **CONTRADICTED as stated.** `messages` is clean (`001_schema.sql:228-235`: `ciphertext BYTEA NOT NULL, nonce BYTEA` — no body column). But the schema does contain a plaintext message column: `001_schema.sql:212  message TEXT,` on `matches`. The code knows it — `crates/server/src/db/conversations.rs:144-146`: `// `matches.message` is left NULL on purpose. It is a plaintext TEXT column, and writing the / // opening message into it as well as into `messages` would put a readable copy of the one`. So the *practice* is right; the sentence "the schema has no plaintext message column" is not. |
| "Never log keys, bundles, passwords, derived keys, plaintext" | **VERIFIED, no counter-example found.** Repo-wide grep of logging macros combined with `password|plaintext|secret_key|private_key|bundle|derived_key|recovery_code` returns exactly two hits, both logging only an id and an error: `crates/server/src/auth/mod.rs:692` and `:695` — `tracing::warn!("password rehash for {} failed: {e}", row.id);` |
| "no ed25519 key and no JWT" | **VERIFIED in code.** No `ed25519` or `jsonwebtoken` dependency in any manifest (`crates/wasm/Cargo.toml:10-20`, `crates/server/Cargo.toml:7-32`). `crates/server/src/config.rs:59` — `/// A2a: the signing-key setting is gone with the JWTs. Sessions are opaque database rows, so`. Sessions are rows: `001_schema.sql:35-46` (`sessions` with `token_hash BYTEA NOT NULL UNIQUE`). **But shipped config files still carry JWT settings** — see §11. |

Cipher suite matches `crates/wasm/Cargo.toml`: `x25519-dalek`, `chacha20poly1305`, `argon2`; `XChaCha20Poly1305` used at `crates/wasm/src/lib.rs:4,64,107,137,174,232,271`; recovery codes at `crates/wasm/src/lib.rs:2343  pub fn generate_recovery_code() -> String {`.

---

## 7. Code layout table

| Row | Verdict |
|---|---|
| `crates/core/` — models + `db_enum!`; enum↔CHECK test in `crates/core/src/tests.rs` | **VERIFIED.** `crates/core/src/lib.rs:1-3` (`pub mod models;` + `mod tests;`); macro defined at `crates/core/src/models/mod.rs:7  macro_rules! db_enum {`; test reads the migration at `crates/core/src/tests.rs:15  let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../migrations/001_schema.sql");` |
| `crates/server/` — bootstrap `main.rs`, routes `api/mod.rs`, config `config.rs` | **VERIFIED.** All three exist and do what is claimed (`main.rs:43 #[tokio::main] async fn main`, `api/mod.rs:32 pub fn router`, `config.rs:290 pub fn load`). REPL at `crates/server/src/repl.rs:5`. |
| `crates/wasm/` | **VERIFIED** (§6). |
| `web/` — static adapter, `ssr = false` | **VERIFIED.** `web/svelte.config.js:1  import adapter from '@sveltejs/adapter-static';`; `web/src/routes/+layout.ts:1  export const ssr = false;` |
| `web/` — **"the API client `web/src/lib/api/**` is the single hub"** | **CONTRADICTED.** Grep `fetch\(`?[^`]*/api/` over `web/src`: **37 occurrences across 14 files**. Only two of those files are in the hub (`web/src/lib/api/categories.ts`: 1, `web/src/lib/api/discovery.ts`: 2) — **34 calls in 12 files bypass it**, including `web/src/lib/stores/auth.ts` (17), `web/src/routes/notifications/+page.svelte` (3), `web/src/routes/account/+page.svelte` (3), `web/src/routes/search/+page.svelte` (2), `web/src/routes/users/[id]/+page.ts` (2), `web/src/lib/components/RespondModal.svelte` (1), `web/src/routes/map/+page.svelte` (1), `web/src/routes/+layout.svelte` (1), `web/src/routes/messages/[id]/+page.svelte` (1), `web/src/lib/stores/server.ts` (1), `web/src/lib/stores/location.ts` (1), `web/src/lib/components/LinkPreview.svelte` (1). |
| `migrations/` — 001 frozen + additive | **VERIFIED** (§5). |
| `docs/` — ARCHITECTURE, CONVENTIONS, CRYPTO, DATABASE, DEVELOPMENT, DEPLOY | **VERIFIED, but the list is incomplete.** All six exist. `docs/` also holds `rubric.md`, `prd.md`, `clippy-report.md`, `agent-rubric.md`, `iteration-log.md`. |
| `deploy/` — nginx/OpenRC/setup/seed; "no relay/WebSocket proxy" | **VERIFIED.** Directory holds exactly `komun.initd`, `nginx-komun.conf`, `setup.sh`, `seed.sql`. OpenRC: `deploy/komun.initd:1  #!/sbin/openrc-run`. `deploy/nginx-komun.conf:6  # There is no relay and no WebSocket route to proxy — the server is plain HTTP + JSON.` |
| `config.example.toml` — sync with `config.rs` defaults; must boot | **PARTLY.** File exists. One deliberate divergence: `config.example.toml:98  require_email_verification = false` vs `crates/server/src/config.rs:158  require_email_verification: true`. Given `config.rs:316-321` fails startup when verification is on without SMTP, the two halves of this row ("in sync with defaults" / "it must boot") cannot both hold; the file chose *boot*. |
| `scripts/` | **VERIFIED** — `scripts/audit.sh`, `scripts/sync-server-repo.sh`. Note `scripts/sync-server-repo.sh:15` rsyncs `crates/relay/`, a directory that does not exist (workspace members are only `crates/server`, `crates/core`, `crates/wasm`, per `Cargo.toml:2`). |

---

## 8. Key architecture facts

- **UUIDv7 primary keys** — **VERIFIED.** `uuid = { version = "1", features = ["v7", "serde"] }` (`Cargo.toml:13`); `Uuid::now_v7()` appears in 13 places across `crates/server/src/db/**` and `api/posts.rs`. Repo-wide grep for `Uuid::new_v4` (excluding `node_modules`): **"No matches found"**.
- **Auth / middleware** — **VERIFIED, list incomplete.** `require_auth` (`auth/mod.rs:1640`), `require_admin` (`:1666`), `require_superadmin` (`:1686`). Role is read per request from the DB: `auth/mod.rs:1664` — `/// Admin *or* superadmin. The role comes from the database on this request, so a demotion that / /// happened a second ago is already in force.` AGENTS.md omits a fourth middleware, `auth/mod.rs:1619  pub async fn require_session(`.
- **"There are no `/api/alliances` and no `/api/communities` routes"** — **VERIFIED.** A whole-repo grep of `crates/server/src` for `.route("…")` returns 61 declarations (complete output reviewed); none is `alliances` or `communities`, and the only `nest`/`nest_service` calls are `/api`, `/avatars`, `/post-images`, `/auth`, `/users`, `/posts`. Glob `**/alliance*` → "No files found" — note this makes the code comment at `crates/server/src/api/mod.rs:21-23` ("The file is still on disk") stale, though AGENTS.md itself is right.
- **API path list** — **accurate but not exhaustive.** Every listed group exists. Unlisted mounted groups: `/api/geocode` (`api/mod.rs:34`), `/api/link-preview` (`:63`), `/api/health` (`api/health.rs:5`), `/api/node` (`api/node.rs:27`), `/api/directory*` (conditional, `api/mod.rs:65-67`), `/api/posts/{id}/report`+`/hide` (`api/reports.rs:16,18`), `/api/search/users` (`api/search.rs:17`).
- **Marketplace / price fields on `listing`+`want`** — **VERIFIED.** `001_schema.sql:160` `CONSTRAINT chk_posts_kind CHECK (kind IN ('resource', 'need', 'offer', 'listing', 'want'))` and `:168-171`:
  ```
      CONSTRAINT chk_posts_market_fields CHECK (
          kind IN ('listing', 'want')
          OR (market_listed = false AND price_cents IS NULL AND item_condition IS NULL)
      )
  ```
- **Append-only `match_offers`** — **VERIFIED as a table** (`001_schema.sql:240-252`, kinds `'offer','counter','accept','decline'`). "Append-only" is enforced by convention, not by a DB rule: there is no trigger or revoke in `001_schema.sql` preventing UPDATE/DELETE on it.
- **Review only against a `completed` deal** — **VERIFIED.** `crates/server/src/db/reviews.rs:62-68`:
  ```
  pub fn check_reviewable(current: MatchStatus) -> Result<(), String> {
      if current == MatchStatus::Completed {
          return Ok(());
  ```
  Enforced under the row lock per `api/reviews.rs:95-97`.
- **Categories: seeded, runtime-editable table, 23 rows, not an enum** — **VERIFIED.** `001_schema.sql:101` begins `INSERT INTO categories (slug, label, scope, sort_order) VALUES` and the value list runs lines 102–124 = **23 rows** (`electronics` … `transport`). Runtime-editable: `crates/server/src/api/categories.rs:40-43` (`POST`/`PATCH /admin/categories`). The only `db_enum!` nearby is the *scope*, not the taxonomy: `crates/core/src/models/category.rs:13-14` — `/// A row of the `categories` table. The taxonomy is seed data, not a Rust enum:`. `docs/DEVELOPMENT.md:89` independently says `# the 23 categories above are the migration's, not this file's:`.
- **Config from `config.toml` / `KOMUN_CONFIG`, env overrides, migrations at startup** — **VERIFIED.** `config.rs:291-292` (`std::env::var("KOMUN_CONFIG").unwrap_or_else(|_| "config.toml".into())`), `config.rs:353 fn apply_env_overrides`, `main.rs:78 sqlx::migrate!`.
- **`[market] default_currency` optional, unset by default** — **VERIFIED.** `config.rs:180  pub default_currency: Option<String>,` on a `#[derive(..., Default)]` struct; `config.example.toml:129  # default_currency = "USD"` (commented out).
- **Background tasks: expiry, health, directory registration, bundle cleanup** — **VERIFIED.** `crates/server/src/tasks/mod.rs:1-4` declares exactly `registration`, `health`, `expiry`, `bundle_cleanup`; all four spawned in `spawn_background_tasks` (two of them conditionally, lines 11 and 16 — AGENTS.md does not mention the conditions).
- **REPL when stdin is a terminal, `help`** — **VERIFIED.** `main.rs:150  if std::io::IsTerminal::is_terminal(&std::io::stdin()) {` → `repl::run_repl(state)`; `repl.rs:26  "help" | "?" => print_help(),`.
- **Service worker caches assets and API responses; PWA, standalone** — **VERIFIED.** `web/src/service-worker.ts:10  const ASSETS = [...build, ...files];` and `:43  const CACHEABLE_API_PATHS = ['/api/node', '/api/health', '/api/posts', '/api/directory'];`; `web/static/manifest.json:6  "display": "standalone",`.

---

## 9. "What this is" prose

- **PostgreSQL 16** — **VERIFIED.** `docker-compose.yml:3  image: postgres:16-alpine`; `docs/DEVELOPMENT.md:16  | PostgreSQL | 16 | the server runs migrations itself at startup |`.
- **AGPL-3.0** — **VERIFIED.** `Cargo.toml:8  license = "AGPL-3.0-or-later"`, `LICENSE` present.
- **No payment rails** — **VERIFIED.** Repo-wide grep for `stripe|paypal|payment|checkout` (excluding `node_modules`, `.dispatch`) finds no payment integration; the only code hit is `crates/wasm/src/lib.rs:1585  "payment",` — an entry in the recovery-code wordlist (neighbours at 1584/1586 are `"pave"`, `"peace"`).
- **"no federation"** — **contradicted by a shipped user-visible string.** `web/static/manifest.json:4  "description": "Federated mutual aid for intercommunal survival"`.

---

## 10. Security model (short)

**MOSTLY VERIFIED, one overstatement.** Argon2id verifier hashing: `crates/server/src/auth/password.rs:4-5` — `//! `verifier = Argon2id(password, auth_salt)` and sends *that*; this module puts a second, / //! independent Argon2id over the verifier before it is stored`. TLS terminated at a proxy: `deploy/nginx-komun.conf:3  # Komun does not terminate TLS itself; run this behind a TLS listener`.

"Server-side crypto is **limited to** Argon2id verifier hashing and TLS termination" is narrower than the code: the server also does SHA-256 session-token hashing (`crates/server/Cargo.toml:31  sha2 = "0.10"`; `auth/mod.rs:168  /// The raw session token. Shown exactly once — only its SHA-256 is stored.`) and generates a per-process salt pepper (`main.rs:89  salt_pepper: Arc::new(sessions::generate_pepper()),`).

---

## 11. Stale artifacts contradicting "no relay / no JWT" (findings, not AGENTS.md errors)

AGENTS.md's code claims hold, but the shipped operator-facing files were not cleaned:

- `.env.example:10-16` — `# JWT secret (generate with: openssl rand -base64 48)` / `JWT_SECRET=generate-a-random-string-and-paste-it-here` / `# Relay — enable in config.toml with:` / `# [relay]` / `# enabled = true` / `# external_url = "wss://relay.yourdomain.com"` (no `[relay]` section exists in `crates/server/src/config.rs:6-20`).
- `docker-compose.yml:30  JWT_SECRET: ${JWT_SECRET}`; `:34  - "9001:9001"`; `:40  - relay_data:/app/data/relay`; `:63  relay_data:`.
- `docker/Dockerfile:23  RUN mkdir -p /app/data/relay && chown komun:komun /app/data/relay`; `:25  EXPOSE 3000 9001`; `:27` comment `openssl-sys (axum/jsonwebtoken chain) link deps.`
- `.dockerignore:18  data/relay/` and `.gitignore:12  data/relay/`.
- `web/svelte.config.js:28-29` CSP still allows `https://app.piggpin.space`, `https://*.piggpin.space`, `wss://*.piggpin.space` — a WebSocket origin for the removed relay.
- `web/vite.config.ts:14  '/community-images': 'http://localhost:3001',` — a proxy for the removed community layer; also proxies the API to **:3001** while the server default and AGENTS.md quickstart say **:3000** (`config.example.toml:8  port = 3000`).
- Root docs outside AGENTS.md's scope still describe the old shape: `agent-summary.md:14  │   └── relay/ # piggPin WebSocket map relay …`, `setup.md:12  a WASM crate holding the client-side crypto, and a WebSocket map relay.`

---

## 12. Could not resolve

These require executing build/test tooling, which is outside my read-only mandate:

1. **"The frontend is currently at 0 svelte-check errors/warnings and all vitest suites pass."** Not settleable by reading. Tooling is present and coherent: `web/package.json:9  "check": "svelte-kit sync && svelte-check --tsconfig ./tsconfig.json"`, `web/vitest.config.ts:7-14` (jsdom, `include: ['src/**/*.test.ts']`, `setupFiles: ['./src/tests/setup.ts']`), 6 test files under `web/src/tests/`. Note there is **no `test` script** in `package.json` — `npx vitest run` as documented is the only entry point.
2. **"`cargo clippy --release -- -D warnings` must stay at zero warnings."** Cannot run cargo. `docs/clippy-report.md` exists but is a document, not evidence of current state.
3. **"`cargo test --workspace` — komun-core + komun-server unit tests."** The scoping is consistent with the tree (`#[cfg(test)]`/`#[test]` appears in `crates/core` and `crates/server` only; grep of `crates/wasm/src` returns 0), but pass/fail is unverifiable here.
4. **"`config.example.toml` … it must boot"** — depends on a live PostgreSQL; only the static divergence in §7 is checkable.
5. **"editing one byte makes every existing server refuse to boot"** — a runtime property of sqlx checksum verification against provisioned databases; the mechanism is wired up (`main.rs:78`) but no database here can confirm it.
6. **The full "never leaves the client" guarantee** — I verified the server's wire types and schema accept no secret key material, but proving no client code path ever transmits one would require auditing all client crypto call sites beyond what the schema settles.
