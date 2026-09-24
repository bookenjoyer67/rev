# AGENTS.md Contract Audit — `/workspace`

Read-only audit performed 2026-09-24. Every verdict below names the artifact that settled it.

---

## 1. Critical rules

### "Never commit these"

| Claim | Artifact | Verdict |
|---|---|---|
| `config.toml` gitignored | `.gitignore:8` | **Confirmed.** Also absent from the tree (only `config.example.toml` exists). |
| `config.toml` holds DB URL + optional SMTP creds | `config.example.toml:10-14`, `:79-90`; `config.rs:29-34`, `:118-130` | **Confirmed.** `[database] url` and an optional `[email]` block. |
| `.env` / `.env.local` gitignored | `.gitignore:6-7` | **Confirmed.** (`.env.example` is committed, which is consistent.) |
| `crates/wasm/pkg/` gitignored | `.gitignore:5` | **Confirmed.** Directory is absent from the tree, as expected. |
| `web/build/` gitignored | `.gitignore:3` | **Confirmed.** |

### Build order

| Claim | Artifact | Verdict |
|---|---|---|
| wasm package must exist before `npm install` | `web/package.json:33` — `"komun-wasm": "file:../crates/wasm/pkg"`; `docs/DEVELOPMENT.md:114-117` | **Confirmed.** The local `file:` dependency cannot resolve without `pkg/`. |
| "…and the frontend before the server can serve static files" | `crates/server/src/main.rs:123-129` | **Contradicted.** The router is `nest("/api", …)`, `nest_service("/avatars", …)`, `nest_service("/post-images", …)`. There is no `ServeDir`/`ServeFile`/`fallback_service` for `web/build`, and a repo-wide grep for `web/build|fallback|index.html` in `crates/server` returns only those two media mounts. Static frontend files are served by nginx instead: `deploy/nginx-komun.conf:30-34` (`root /opt/komun/frontend; try_files … /index.html`). `docs/DEVELOPMENT.md:138` agrees with the code ("serves the API under `/api`"), while its own inline comment at `:111` repeats the same wrong claim. |

### sqlx uses runtime queries

| Claim | Artifact | Verdict |
|---|---|---|
| No compile-time macros | Repo-wide grep for `sqlx::query!` / `query_as!` / `query_scalar!` / `query_file!` → **no matches**; `crates/server/Cargo.toml:21` sqlx features are `runtime-tokio, tls-rustls, postgres, uuid, chrono, json, migrate` — **no `macros`** | **Confirmed.** |
| No offline query cache | `.sqlx/` does not exist | **Confirmed.** |
| "Docker builds work as-is" | `docker/Dockerfile:10-11` | **Confirmed but odd.** Build succeeds because no macro needs a DB; however the Dockerfile still sets `ENV SQLX_OFFLINE=true`, a leftover from the macro era that now does nothing. |

### Frontend is Svelte 5 runes only

| Claim | Artifact | Verdict |
|---|---|---|
| No `$:`, no `export let`, no `on:click` | Grep over `web/src` for `export let `, `on:click`, `on:submit`, `on:change`, `^\s*\$:` → **no matches** | **Confirmed.** |
| Uses `$state`/`$derived`/`$props`/`onclick` | Same tree: 328 occurrences across 33 files | **Confirmed.** |

### Migrations are frozen at 001

| Claim | Artifact | Verdict |
|---|---|---|
| `migrations/001_schema.sql` exists and is the baseline | `migrations/001_schema.sql:1-6` | **Confirmed.** |
| Schema changes are additive `002_*.sql` files | `migrations/002_directory_open_registration.sql:1-7` — an `ALTER TABLE … ADD COLUMN`, with a header restating the checksum reason | **Confirmed.** Exactly two migration files exist. |
| Documented in `docs/DEVELOPMENT.md` | `docs/DEVELOPMENT.md:96-102` | **Confirmed.** |
| "checksum-bookmarked in every provisioned database — editing one byte makes every existing server refuse to boot" | `main.rs:78-81` runs `sqlx::migrate!`; sqlx does checksum applied migrations | **Consistent, not fully settled by the repo** — it depends on the state of deployed databases, which no file here records. |

### Crypto boundaries

| Claim | Artifact | Verdict |
|---|---|---|
| Server stores public keys and wrapped bundles only | `migrations/001_schema.sql:25-29` — `encryption_public_key`, `encrypted_key_bundle`, `bundle_salt`, `encrypted_recovery_bundle`, `recovery_bundle_salt`. No secret-key or recovery-code column. | **Confirmed at the schema level.** |
| No plaintext message column | `001_schema.sql:227-235` — `messages(id, match_id, sender_id, ciphertext BYTEA, nonce, created_at)` | **Confirmed.** |
| Never log keys/bundles/passwords/plaintext | Grep of all `tracing::{info,debug,warn,error}!` lines containing `password\|bundle\|secret\|ciphertext\|plaintext\|token`: 9 hits, all of which log a row id or an error (`auth/mod.rs:692` "password rehash for {id} failed", `sessions.rs:107` a count, etc.) | **Confirmed — no counterexample found.** A negative over all code paths cannot be fully proven by grep. |
| There is no ed25519 key | `crates/wasm/Cargo.toml:10-20` — `x25519-dalek`, `chacha20poly1305`, `argon2`, `sha2`; no ed25519 crate anywhere. The only `ed25519` hits are obituary comments (`wasm/src/lib.rs:12`, `server/src/auth/mod.rs:9`). | **Confirmed.** |
| No JWT; sessions are opaque DB rows | No `jsonwebtoken` dependency; `001_schema.sql:35-46` `sessions` table with `token_hash BYTEA UNIQUE`; `auth/mod.rs:1582-1616` bearer → SHA-256 → session row | **Confirmed in code.** See finding F3 for a stale `JWT_SECRET` in `docker-compose.yml`. |
| x25519/password/recovery keys never leave the client | `crates/wasm/src/lib.rs:212-289`, `:2343-2358` (recovery code derives a wrapping key client-side); `crates/core/src/tests.rs:383-409` asserts `UserProfile` serialization leaks no `password`/`auth_salt`/`encrypted_key_bundle`/`recovery` | **Supported.** The strong form ("never leave") is a whole-system property the repository only partially settles. |

---

## 2. Code layout table

All named paths exist: `crates/core/`, `crates/server/`, `crates/wasm/`, `web/`, `migrations/`, `docs/`, `deploy/`, `config.example.toml`, `scripts/`. Specific sub-claims:

- `db_enum!` macro → `crates/core/src/models/mod.rs:7`. **Confirmed.**
- enum↔CHECK test in `crates/core/src/tests.rs` → `:15` reads `concat!(CARGO_MANIFEST_DIR, "/../../migrations/001_schema.sql")` at test time. **Confirmed**, including the "reads the migration at test time" claim in the Tests section.
- Bootstrap in `main.rs`, routes in `api/mod.rs`, config in `config.rs` → all three files exist and hold those roles. **Confirmed.**
- `web/` adapter-static + `ssr = false` → `web/svelte.config.js:1-12` (`adapter-static`, `fallback: 'index.html'`), `web/src/routes/+layout.ts:1`. **Confirmed.**
- `deploy/` "no relay/WebSocket proxy" → `deploy/nginx-komun.conf:6` states it, and the file contains three `proxy_pass` blocks with no `Upgrade`/`ws` handling. **Confirmed.**
- `config.example.toml` "keep in sync with `config.rs` defaults; it must boot" → **Mostly confirmed** with one deliberate divergence: `registration.require_email_verification` defaults to `true` in `config.rs:154-161` but is `false` in `config.example.toml:98`. That divergence is what makes the example bootable (`config.rs:316-321` refuses startup when verification is on without SMTP), so the "it must boot" half holds and the "in sync with defaults" half does not, literally.
- `docs/` row lists ARCHITECTURE, CONVENTIONS, CRYPTO, DATABASE, DEVELOPMENT, DEPLOY → all six present. The directory also contains `rubric.md`, `agent-rubric.md`, `prd.md`, `clippy-report.md`, `iteration-log.md`, which the row does not mention.
- **`web/src/lib/api/**` is "the single hub"** → **Contradicted.** `web/src/lib/stores/auth.ts` makes at least 9 raw `fetch()` calls straight to `/api/auth/*` (`:201, :211, :250, :301, :346, :357, :385, :434, :470, :495`), and `web/src/lib/stores/server.ts:74` hits `/api/node`. Further direct `fetch(` callers outside `lib/api/`: `routes/messages/[id]/+page.svelte`, `routes/search/+page.svelte`, `routes/map/+page.svelte`, `routes/+layout.svelte`, `routes/account/+page.svelte`, `routes/notifications/+page.svelte`, `routes/users/[id]/+page.ts`, `lib/components/RespondModal.svelte`, `lib/components/LinkPreview.svelte`, `lib/stores/location.ts`.

---

## 3. Key architecture facts

| Claim | Artifact | Verdict |
|---|---|---|
| UUIDv7 primary keys | `Cargo.toml:13` `uuid` with feature `v7`; `Uuid::now_v7()` at `db/reviews.rs:95`, `db/posts.rs:135`, `db/endorsements.rs:24`; schema columns are bare `UUID PRIMARY KEY` with no DB default | **Confirmed.** |
| Argon2id verifier, opaque DB sessions, no JWT | `auth/password.rs:20-27` (`Algorithm::Argon2id`, `Version::V0x13`), `:57` encoded hash; `sessions` table | **Confirmed.** |
| Middleware `require_auth`/`require_admin`/`require_superadmin` | `auth/mod.rs:1640`, `:1666`, `:1686` | **Confirmed.** Note a fourth, `require_session` (`:1619`), used by the `/me` routes, is not mentioned. |
| Role loaded from DB on every request | `db/sessions.rs:25` and `:77` — the session lookup selects `u.role` in the same round trip; `auth/mod.rs:1664` documents it | **Confirmed.** |
| API surface: `/api/posts`, `/api/search`, `/api/auth/**`, `/api/users/**`, `/api/me/*`, `/api/conversations/*`, `/api/categories`, `/api/admin/*`, `/api/matches/{id}/reviews` | `api/mod.rs:32-69` plus the per-module routers: `posts.rs:30-36`, `search.rs:16-17`, `auth/mod.rs:103-122`, `users.rs:14`, `conversations.rs:23-27`, `notifications.rs:15-18` (`/me/*`), `categories.rs:36-43`, `admin.rs:20-31`, `reviews.rs:44` | **Confirmed — but incomplete.** Also mounted and unlisted: `/api/health`, `/api/node`, `/api/geocode`, `/api/link-preview`, `/api/directory*` (conditional on `discovery.directory_enabled`), `/api/posts/{id}/report`, `/api/admin/reports*`, `/api/users/{id}/endorsements`. |
| No `/api/alliances`, no `/api/communities` | `api/mod.rs:1-23` — no such module declared, no such route in the whole grep of `.route(`/`.nest(` | **Confirmed.** Note the comment at `api/mod.rs:21-23` says "The file is still on disk" — it is **not**: `crates/server/src/api/alliances.rs` does not exist. The comment is stale; the claim itself holds. |
| `listing`/`want` kinds carry the price fields | `001_schema.sql:150-171` — `price_cents`, `currency`, `price_negotiable`, `item_condition`, `market_listed`, plus `chk_posts_market_fields` restricting them to `kind IN ('listing','want')`, and `chk_posts_kind` listing all five kinds | **Confirmed.** |
| `match_offers` is an append-only trail | `001_schema.sql:240-252` (insert-only shape, no `updated_at`); repo-wide grep for `UPDATE match_offers` / `DELETE FROM match_offers` → **no matches** | **Confirmed.** |
| Review writable only against a `completed` deal | `db/reviews.rs:62-69` `check_reviewable` requires `MatchStatus::Completed`, called at `:86` under a row lock inside the transaction; handler at `api/reviews.rs:98-110` maps refusal to 409 | **Confirmed.** |
| Category taxonomy is a seeded, runtime-editable table (23 rows), not an enum | `001_schema.sql:88-97` (`categories` table) and `:101-124` (seed `INSERT`) — **exactly 23 rows**, cross-checked by `docs/DEVELOPMENT.md:91-93` (aid 2 + both 6 + market 15 = 23). Runtime-editable via `api/categories.rs:39-43` (`POST /admin/categories`, `PATCH /admin/categories/{slug}`) and the `active`/`updated_at` columns. Migration 002 adds no categories. | **Confirmed.** |
| Config from `config.toml` or `KOMUN_CONFIG`, with env overrides | `config.rs:290-305`, `:353-373` (`KOMUN_BIND_ADDRESS`, `KOMUN_PORT`, `DATABASE_URL`, `KOMUN_NODE_NAME`, `BIND_ADDR`) | **Confirmed.** |
| Server runs migrations on startup | `main.rs:78-81` | **Confirmed.** |
| `[market] default_currency` optional and unset by default | `config.rs:171-181` (`Option<String>`, `Default`), `config.example.toml:129` (commented out) | **Confirmed.** |
| Background tasks in `tasks/`: expiry, health, directory registration, bundle cleanup | `tasks/mod.rs:1-30` — exactly those four modules, all four files present | **Confirmed.** (`health` and `registration` only spawn when discovery config enables them.) |
| REPL starts when stdin is a terminal; `help` | `main.rs:150-151` (`IsTerminal::is_terminal(&stdin())`), `repl.rs:26` (`"help" \| "?" => print_help()`) | **Confirmed.** |
| Service worker caches assets and API responses; PWA standalone | `web/src/service-worker.ts:10-55` (asset precache + network-first cache for `/api/node`, `/api/health`, `/api/posts`, `/api/directory`); `web/static/manifest.json:6` `"display": "standalone"` | **Confirmed.** |
| Stack: Axum + sqlx, PostgreSQL 16, SvelteKit 5, AGPL-3.0 | `crates/server/Cargo.toml:9,21`; `docker-compose.yml:3` `postgres:16-alpine`; `web/package.json:13,20`; `Cargo.toml:8` `license = "AGPL-3.0-or-later"` + `LICENSE` | **Confirmed.** |

---

## 4. Findings

**F1 — The server does not serve the frontend.** `AGENTS.md:23-24` gives "the frontend before the server can serve static files" as the reason for build step ordering. `crates/server/src/main.rs:123-129` mounts only `/api`, `/avatars` and `/post-images`; nginx serves `web/build` from `/opt/komun/frontend` (`deploy/nginx-komun.conf:30-34`). The *ordering* is still correct for the reason given at `docs/DEVELOPMENT.md:114-117` (the npm `file:` dependency), but the stated justification is wrong. `docs/DEVELOPMENT.md:111` carries the same wrong comment.

**F2 — "single hub" API-client claim is not true.** `AGENTS.md:61`. At minimum `web/src/lib/stores/auth.ts` (all auth flows) and `web/src/lib/stores/server.ts` bypass `web/src/lib/api/**` with raw `fetch` to `/api/...`, as do ten route/component files.

**F3 — Relay/JWT residue in deployment files, contradicting `AGENTS.md:12` and `:52`.** `docker-compose.yml:30` passes `JWT_SECRET: ${JWT_SECRET}` (a variable `config.rs:353-373` never reads), `:34` publishes port `9001`, `:40,63` mount a `relay_data` volume at `/app/data/relay`; `docker/Dockerfile:23,25` creates `/app/data/relay` and exposes `9001`; `.gitignore:12` still ignores `data/relay/`. Also `web/svelte.config.js:29` keeps `wss://*.piggpin.space` in the CSP `connect-src` although no `WebSocket` or `wss://` usage exists anywhere in `web/src`. None of this makes the code claims false — there is no relay code and no JWT code — but the deployment surface still advertises both.

**F4 — `web/static/manifest.json:4`** describes the app as "**Federated** mutual aid for intercommunal survival", while `AGENTS.md:11-12` states there is no federation. The schema and route table back AGENTS.md; the manifest string is stale.

**F5 — Stale comment in `crates/server/src/api/mod.rs:21-23`** claims `alliances.rs` "is still on disk". It is not. The routing claim in `AGENTS.md` is unaffected.

**F6 — `config.example.toml:98` diverges from the `config.rs:154-161` default** for `require_email_verification` (`false` vs `true`). Deliberate and necessary for "it must boot", but it means the "keep in sync with `config.rs` defaults" instruction is not literally satisfied.

**F7 — Minor incompleteness (not errors).** The API list omits `/api/health`, `/api/node`, `/api/geocode`, `/api/link-preview`, `/api/directory*`, `/api/admin/reports*`; the middleware list omits `require_session`; the `docs/` row omits five files present in `docs/`.

**F8 — Adjacent to the "never commit" list.** `data/avatars/` contains two committed `.webp` uploads and is not gitignored (`.gitignore` ignores `data/relay/` only). `AGENTS.md` does not claim otherwise, so this is not a contract violation — flagging it because it is runtime data in version control.

No file or directory named by `AGENTS.md` is missing.

---

## 5. Could not resolve

- **"0 svelte-check errors/warnings and all vitest suites pass"** (`AGENTS.md:109`) and **"clippy must stay at zero warnings"** (`:105`). These require running `npm run check`, `npx vitest run` and `cargo clippy`, which I am not permitted to do. `docs/clippy-report.md:1-25` asserts a clean `cargo clippy --release -- -D warnings` run dated 2026-09-24 on branch `feature/lab-task2-clippy-gate`, but that is a second document, not an artifact that settles the claim. `web/package.json:5-10` has no `test` script; `npx vitest run` is viable since `vitest ^4.1.10` is a devDependency and `web/vitest.config.ts` exists.
- **"001 is checksum-bookmarked in every provisioned database"** — depends on deployed database state, which the repository does not record. The mechanism (`sqlx::migrate!` at `main.rs:78`) is consistent with it.
- **"The x25519 secret key, password-derived key and recovery code never leave the client"** — supported by the schema, by `crates/wasm/src/lib.rs` and by the `UserProfile` leak test, but proving a universal negative across all client code and all handlers is beyond static reading.
- **"It must boot" for `config.example.toml`** — startup validation is satisfiable by reading `config.rs:308-343`, but actual boot needs a reachable PostgreSQL, which I cannot exercise.
