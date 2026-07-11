# AGENTS.md — Komun

## What this is

Komun is a federated mutual aid discovery platform. It lets communities post needs/offers/resources and match them via encrypted conversations. Rust backend (Axum), SvelteKit SPA frontend, PostgreSQL, AGPL-3.0.

## Critical rules

### Never commit these
- `config.toml` — gitignored, contains secrets
- `.env` / `.env.local` — gitignored
- `crates/wasm/pkg/` — build artifact, gitignored
- `web/build/` — build artifact, gitignored

### Build order matters
The WASM crate must be built before the frontend, and the frontend before the server can serve static files:

```
wasm-pack build crates/wasm --target web
cd web && npm install && npm run build
cargo build --release --bin komun-server
```

If you change crypto in `crates/wasm/`, you must rebuild the WASM pkg and the frontend.

### Docker builds use runtime queries
The Dockerfile sets `ENV SQLX_OFFLINE=true` as a safety measure, but since all queries use `sqlx::query()` / `sqlx::query_as()` (runtime), not `sqlx::query!()` (compile-time), no `sqlx prepare` step is needed. Docker builds work as-is.

### Svelte 5 runes only
No `$:`, no `export let`, no `on:click`. Use `$state()`, `$derived()`, `$effect()`, `$props()`, `onclick={handler}`.

### Crypto boundaries
- Secret keys NEVER leave the client. The server only stores public keys and encrypted key bundles.
- The passphrase NEVER leaves the client. Only the recovery ID (Argon2 hash) goes to the server.
- Do not log keys, bundles, or passphrases anywhere.

## Code layout

| Path | What | Be careful |
|---|---|---|
| `crates/core/` | Shared data models (Community, Member, Post, MatchThread) | Changes here affect both server and client expectations |
| `crates/server/` | Axum HTTP server, REST API, DB queries, auth, REPL | Bootstrap in `main.rs`, routes in `api/mod.rs` |
| `crates/wasm/` | Client-side crypto compiled to WASM (ed25519, x25519, chacha20, argon2) | Breaking changes here break all encryption |
| `crates/relay/` | piggPin WebSocket map relay with optional bridges (MQTT, RNode, Reticulum) | Feature-gated; default build has no bridges |
| `web/` | SvelteKit SPA frontend (static adapter) | SPA mode: `ssr = false`, `prerender = false` |
| `migrations/` | SQLx migrations (numbered, additive) | Never edit existing migrations; add new ones |
| `docker/` | Multi-stage Dockerfile | Builds Rust + frontend separately |
| `config.example.toml` | Documented config template | Keep in sync with `config.rs` defaults |
| `scripts/` | Utility scripts (sync-server-repo.sh) | |

## Quickstart (local dev)

```bash
# Start PostgreSQL
docker compose up db -d

# Build WASM
wasm-pack build crates/wasm --target web

# Build frontend
cd web && npm install && npm run build && cd ..

# Run server (requires config.toml — copy from config.example.toml and edit)
cp config.example.toml config.toml
cargo run --bin komun-server
```

## Quickstart (full Docker)

```bash
docker compose up --build
```

Opens on `http://localhost:3000`.

## Key architecture facts

- UUIDv7 is used for all primary keys (time-sortable)
- JWT auth with HS256, token in `Authorization: Bearer <token>` header
- Auth middleware: `require_auth` (any user), `require_superadmin` (role check)
- API handlers return `Result<Json<T>, (StatusCode, Json<Value>)>`
- Config loaded from `config.toml` with env var overrides (see `config.rs`)
- Background tasks spawned in `tasks/` for expiry, bundle cleanup, health checks
- REPL available when run in a terminal (type `help` for commands)
- Service worker provides offline caching for API and assets
- PWA with standalone display mode, SVG icon

## Docker Compose

Two services: `db` (postgres:16-alpine) and `app` (the Rust binary). Ports 5432 and 3000. Named volume `pgdata` for persistence.

## Tests

There are currently no automated tests. Manual verification is done by running the server and frontend.

## Security Model

### Threat Model
- **Attacker capabilities:** Network observer, compromised relay node, XSS via user-generated content, disk access to server
- **Out of scope:** Physical device compromise, supply chain attacks on dependencies, quantum adversaries

### Crypto Boundaries
- **Client-side (WASM):** ed25519 key generation/signing, x25519 ECDH, ChaCha20Poly1305 encryption, Argon2 key derivation, BIP39 recovery codes
- **Server-side:** JWT HS256, TLS termination
- **Never leaves client:** ed25519 secret key, x25519 secret key, passphrase, recovery code (only Argon2 hash sent to server)
- **Server stores:** Public keys, encrypted key bundles, recovery_id (Argon2 hash), recovery_code_hash (Argon2 hash)

### Key Hierarchy
1. Passphrase → Argon2 → wrap key → decrypts key bundle (ed25519 secret + x25519 secret)
2. ed25519 key → signs challenges → JWT token (includes role claim)
3. x25519 key → ECDH → conversation encryption keys (ChaCha20Poly1305)
4. Recovery code (BIP39 12 words) → Argon2 → recovery_code_hash → server verifies

### Auth Flow
1. Client generates ed25519 + x25519 keypair in WASM
2. Server issues challenge → client signs with ed25519 → server verifies → JWT issued
3. JWT contains user_id (sub) and role — no DB query needed for authorization
4. Optional: passphrase encrypts key bundle for server-side recovery
5. Optional: BIP39 recovery code as backup identity factor
