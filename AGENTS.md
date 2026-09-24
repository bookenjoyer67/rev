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

### Build order
The wasm package must exist before the frontend is installed, and the frontend before the
server can serve static files:

```bash
wasm-pack build crates/wasm --target web     # 1. -> crates/wasm/pkg/
cd web && npm install && npm run build       # 2. package.json needs pkg/ to exist
cargo build --release --bin komun-server     # 3.
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
(`002_*.sql`, …). See `docs/DEVELOPMENT.md`.

### Crypto boundaries
- The **x25519 secret key, the password-derived key and the recovery code never leave the client**; the
  server stores public keys and wrapped bundles only.
- The schema has **no plaintext message column** (`messages.ciphertext` only).
- **Never log** keys, bundles, passwords, derived keys, or message plaintext.
- There is no ed25519 key and no JWT; sessions are opaque database rows.

## Code layout

| Path | What | Be careful |
|---|---|---|
| `crates/core/` | Shared models + `db_enum!` macro | Changes affect server and client expectations; the enum↔CHECK test lives in `crates/core/src/tests.rs` |
| `crates/server/` | Axum HTTP server, auth/sessions, DB queries, tasks, REPL | Bootstrap in `main.rs`, routes in `api/mod.rs`, config in `config.rs` |
| `crates/wasm/` | Client crypto → WASM (x25519, XChaCha20Poly1305, Argon2, recovery codes) | Breaking changes here break all encryption; rebuild pkg + frontend |
| `web/` | SvelteKit 5 SPA (static adapter, `ssr = false`) | Runes only; the API client `web/src/lib/api/**` is the single hub |
| `migrations/` | `001_schema.sql` (frozen) + additive migrations | Never edit `001`; add `002+` |
| `docs/` | ARCHITECTURE, CONVENTIONS, CRYPTO, DATABASE, DEVELOPMENT, DEPLOY | Keep in sync with the code |
| `deploy/` | nginx/OpenRC/setup/seed starting points | Docs only; no relay/WebSocket proxy |
| `config.example.toml` | Documented config template | Keep in sync with `config.rs` defaults; it must boot |
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
  `require_auth` / `require_admin` / `require_superadmin`, and it loads the role from the DB on
  every request.
- API: flat `/api/posts`, `/api/search`, `/api/auth/**`, `/api/users/**`, `/api/me/*`,
  `/api/conversations/*`, `/api/categories`, `/api/admin/*`, `/api/matches/{id}/reviews`. There
  are no `/api/alliances` and no `/api/communities` routes.
- Marketplace: `listing` and `want` post kinds carry the price fields; the negotiation on a
  match thread is the append-only `match_offers` trail; a review is writable only against a
  `completed` deal. The category taxonomy is the seeded, runtime-editable `categories` table
  (23 rows), not an enum. See `docs/ARCHITECTURE.md` and `docs/DATABASE.md`.
- Config is loaded from `config.toml` (or `KOMUN_CONFIG`) with env overrides; the server runs
  migrations on startup. `[market] default_currency` is optional and unset by default.
- Background tasks live in `tasks/` (expiry, health, directory registration, bundle cleanup).
- The REPL starts when stdin is a terminal (type `help`).
- The service worker caches assets and API responses; the app is a PWA with standalone display.

## Tests

```bash
cargo test --workspace     # komun-core + komun-server unit tests
cargo clippy --release -- -D warnings   # must stay at zero warnings
cd web && npm run check && npm run build && npx vitest run
```

The frontend is currently at **0 svelte-check errors/warnings** and all vitest suites pass.
The enum↔CHECK agreement test reads `migrations/001_schema.sql` at test time.

## Security model (short)

Threat model: network observer, compromised client state, XSS via user content, disk access to
the server. Out of scope: device compromise, supply-chain attacks, quantum adversaries.
Server-side crypto is limited to Argon2id verifier hashing and TLS termination at a proxy.

**Honest limitation:** browser-delivered E2E cannot protect against a malicious server serving
modified JavaScript. It protects against database theft, passive disk reads, an operator reading
message content, and admin snooping — not against a hostile operator who ships modified client
code.

More detail: `docs/ARCHITECTURE.md`, `docs/CRYPTO.md`, `docs/DATABASE.md`,
`docs/DEVELOPMENT.md`, `docs/CONVENTIONS.md`, `docs/DEPLOY.md`.
