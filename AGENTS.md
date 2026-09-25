# AGENTS.md — Komun

Cold-start guide for an agent working in this repo. Keep it accurate: if code and this file
disagree, fix one of them.

## What this is

A **single-server** mutual-aid web app. People post needs/offers/resources and marketplace
listings/wants, search them, negotiate over end-to-end-encrypted conversations, and — for a
completed deal — leave a star review. Rust backend (Axum + sqlx, PostgreSQL 16), SvelteKit 5
SPA frontend, client crypto in WASM, AGPL-3.0. There is no multi-tenant community layer, no
federation, and no relay — those were removed in the reshape. There are no payment rails.

## Critical rules

### Never commit these
- `config.toml` — gitignored, holds the DB URL and optional SMTP credentials
- `.env` / `.env.local` — gitignored
- `crates/wasm/pkg/` — build artifact, gitignored
- `web/build/` — build artifact, gitignored
- `data/avatars/`, `data/post-images/` — runtime uploads, gitignored

### Build order
The wasm package must exist before the frontend is installed. The server does **not** serve the
SPA — nginx does (`deploy/nginx-komun.conf`, "SvelteKit static build … with an SPA fallback"),
and the router mounts only `/api`, `/avatars`, `/post-images` — so step 3 depends on neither of
the first two:

```bash
wasm-pack build crates/wasm --target web     # 1. -> crates/wasm/pkg/
cd web && npm install && npm run build       # 2. package.json needs pkg/ to exist
cargo build --release --bin komun-server     # 3. independent of 1 and 2
```

If you change crypto in `crates/wasm/`, rebuild the wasm package **and** the frontend.

### sqlx uses runtime queries
All queries use `sqlx::query()` / `sqlx::query_as()`, not the compile-time macros. No
`cargo sqlx prepare` step and no offline query cache — Docker builds work as-is.

### Frontend is Svelte 5 runes only
No `$:`, no `export let`, no `on:click`. Use `$state`, `$derived`, `$effect`, `$props`, and
`onclick={handler}`.

### Migrations are frozen at 001
`migrations/001_schema.sql` is checksum-bookmarked in every provisioned database — editing one
byte makes every existing server refuse to boot. Schema changes are additive files
(`002_*.sql`, `003_*.sql`, …). See `docs/DEVELOPMENT.md`.

### Crypto boundaries
- The **x25519 secret key, the password-derived key and the recovery code never leave the client**; the
  server stores public keys and wrapped bundles only. (A password-derived *verifier* is sent — see
  `docs/CRYPTO.md`; the password itself never is.)
- The schema has **no plaintext message column**: `messages` carries `ciphertext` + `nonce` only,
  and the plaintext `matches.message` column was dropped by `003_drop_matches_message.sql`.
- **Never log** keys, bundles, passwords, derived keys, or message plaintext.
- There is no ed25519 key and no JWT; sessions are opaque database rows.

## Code layout

| Path | What | Be careful |
|---|---|---|
| `crates/core/` | Shared models + `db_enum!` macro | Changes affect server and client expectations; the enum↔CHECK test lives in `crates/core/src/tests.rs` |
| `crates/server/` | Axum HTTP server, auth/sessions, DB queries, tasks, REPL | Bootstrap in `main.rs`, routes in `api/mod.rs`, config in `config.rs` |
| `crates/wasm/` | Client crypto → WASM (x25519, XChaCha20Poly1305, Argon2, recovery codes) | Breaking changes here break all encryption; rebuild pkg + frontend |
| `web/` | SvelteKit 5 SPA (static adapter, `ssr = false`) | Runes only. `web/src/lib/api/**` holds the shared API helpers, but most calls live in stores and routes: 37 `fetch()` calls to `/api/` in 14 files, 34 of them outside `lib/api/` (`lib/stores/auth.ts` alone holds 17) |
| `migrations/` | `001_schema.sql` (frozen) + additive migrations | Never edit `001`; add `002+` |
| `docs/` | ARCHITECTURE, CONVENTIONS, CRYPTO, DATABASE, DEVELOPMENT, DEPLOY | Plus the quality-control artifacts (`prd.md`, `rubric.md`, `agent-rubric.md`, `iteration-log.md`, `clippy-report.md`, `contract-audit/`); keep the prose docs in sync with the code |
| `deploy/` | nginx/OpenRC/setup/seed starting points | Docs only; no relay/WebSocket proxy |
| `config.example.toml` | Documented config template | Keep in sync with `config.rs` defaults **except** `require_email_verification = false` here vs `true` in `config.rs` — deliberate, the example must boot without SMTP (a `true` with no `[email]` refuses to start) |
| `scripts/` | Utility scripts | |

## Quickstart (local dev)

```bash
# config
cp config.example.toml config.toml        # edit [database] url

# build (wasm first!)
wasm-pack build crates/wasm --target web
cd web && npm install && npm run build && cd ..

# run
cargo run --bin komun-server              # -> http://localhost:3000
```

## Key architecture facts

- UUIDv7 primary keys (time-sortable).
- Auth: email + password verifier (Argon2id), opaque DB sessions; **no JWT**. Middleware is
  `require_session` / `require_auth` / `require_admin` / `require_superadmin`, and it loads the
  role from the DB on every request.
- API: flat `/api/posts`, `/api/search`, `/api/auth/**`, `/api/users/**`, `/api/me/*`,
  `/api/conversations/*`, `/api/categories`, `/api/admin/*`, `/api/matches/{id}/reviews`. Mounted
  too, and easy to miss: `/api/health`, `/api/node`, `/api/geocode`, `/api/link-preview`,
  `/api/search/users`, `/api/posts/{id}/report` and `/hide`, and `/api/directory*` (only when
  `[discovery] directory_enabled = true` — otherwise those routes are not mounted at all and
  404). There are no `/api/alliances` and no `/api/communities` routes.
- Marketplace: `listing` and `want` post kinds carry the price fields; the negotiation on a
  match thread is the append-only `match_offers` trail (append-only by convention — no trigger or
  revoke enforces it); a review is writable only against a `completed` deal. The category
  taxonomy is the seeded, runtime-editable `categories` table (23 rows), not an enum. See
  `docs/ARCHITECTURE.md` and `docs/DATABASE.md`.
- Config is loaded from `config.toml` (or `KOMUN_CONFIG`) with env overrides; the server runs
  migrations on startup. `[market] default_currency` is optional and unset by default.
- Background tasks live in `tasks/` (expiry, health, directory registration, bundle cleanup; two
  of the four are spawned conditionally).
- The REPL starts when stdin is a terminal (type `help`).
- The service worker caches assets and API responses; the app is a PWA with standalone display.

## Tests

```bash
cargo test --workspace     # komun-core + komun-server unit tests
cargo clippy --release -- -D warnings   # must stay at zero warnings (touch a source file first — a silent second run is a cache hit, not a clean lint)
cd web && npm run check && npm run build && npx vitest run
```

Measured 2026-09-25 in the agent sandbox (`rustc 1.95.0`, `node v22.23.2`), on the commit that
dropped the plaintext column: `cargo test --workspace` → **158 passed, 0 failed, 0 ignored**
(20 in `komun-core`, 138 in `komun-server`; three zero-test suites); `cargo clippy --release -- -D
warnings` → exit 0, no lints (the only line cargo prints is a future-incompat note about the
`sqlx-postgres` dependency); `npm run check` → **0 errors, 0 warnings**; `npm run build` → green;
`npx vitest run` → **82 tests in 7 files, all passing**. The frontend gates need
`crates/wasm/pkg/` to exist first.

The enum↔CHECK agreement test reads `migrations/001_schema.sql` at test time.

## Security model (short)

Threat model: network observer, compromised client state, XSS via user content, disk access to
the server. Out of scope: device compromise, supply-chain attacks, quantum adversaries.
Server-side crypto is Argon2id verifier hashing (a second, independent Argon2id over the verifier
before storage), SHA-256 session-token hashing, a per-process salt pepper, and TLS terminated at a
proxy.

**Honest limitation:** browser-delivered E2E cannot protect against a malicious server serving
modified JavaScript. It protects against database theft, passive disk reads, an operator reading
message content, and admin snooping — not against a hostile operator who ships modified client
code.

More detail: `docs/ARCHITECTURE.md`, `docs/CRYPTO.md`, `docs/DATABASE.md`,
`docs/DEVELOPMENT.md`, `docs/CONVENTIONS.md`, `docs/DEPLOY.md`.
