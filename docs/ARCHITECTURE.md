# Architecture

## What Komun is

A single-server mutual-aid web app. The server **is** the community: there is no
multi-tenant `communities` table, no federation, and no relay. Users sign up with email and
password, post needs/offers/resources (and, in Phase B, marketplace listings), and negotiate
over end-to-end encrypted match threads. The only outbound integration is an optional public
directory listing and the OSM/Nominatim map.

## System overview

```
┌──────────────────────────────────────────────────────────┐
│ Browser (SvelteKit 5 SPA, ssr=false)                     │
│  web/src/  ── UI, routes, stores                         │
│  komun-wasm ── x25519 ECDH + XChaCha20Poly1305 + Argon2  │
└───────────────┬──────────────────────────────────────────┘
                │ HTTPS (JSON; ciphertext for messages)
                ▼
┌──────────────────────────────────────────────────────────┐
│ Axum server (crates/server, :3000)                       │
│  api/   REST handlers (flat /api/posts, /api/auth, …)    │
│  auth/  signup, signin, sessions, password, email        │
│  db/    sqlx runtime queries                             │
│  tasks/ expiry, health, directory registration           │
│  repl.rs, security_headers.rs, rate_limit.rs, sessions.rs│
└───────────────┬──────────────────────────────────────────┘
                │
                ▼
        ┌───────────────┐        ┌────────────────────────┐
        │ PostgreSQL 16 │        │ optional: public       │
        │ (one node)    │        │ directory + OSM tiles  │
        └───────────────┘        └────────────────────────┘
```

## Crates

| Crate | Role |
|---|---|
| `crates/core` | Shared models and the `db_enum!` macro (plain data + serde; no DB or HTTP) |
| `crates/server` | Axum HTTP API, auth/sessions, sqlx queries, background tasks, REPL |
| `crates/wasm` | Client-side crypto compiled to WASM (x25519, XChaCha20Poly1305, Argon2, recovery codes) |

`komun-relay` and the `federation/` module were deleted in the reshape; the federation
config section and the alliances API are gone.

## Server module tree (`crates/server/src/`)

```
main.rs              bootstrap: config, pool, migrations, router, tasks, REPL
config.rs            TOML config, env overrides, startup validation
sessions.rs          opaque session tokens (random raw, SHA-256 stored)
rate_limit.rs        per-IP token buckets for auth routes
security_headers.rs  response security headers
repl.rs              interactive admin CLI when stdin is a terminal

api/
  mod.rs             router composition
  health.rs          GET  /api/health
  node.rs            GET  /api/node         (server identity + discovery flags)
  posts.rs           GET/POST /api/posts, GET/PATCH/DELETE /api/posts/{id}, image upload
  conversations.rs   responses, /api/me/conversations, messages, status
  search.rs          GET  /api/search, /api/search/users
  users.rs           GET  /api/users/{id}
  endorsements.rs    GET/POST/DELETE /api/users/{id}/endorse(ments)
  notifications.rs   /api/me/notifications*
  admin.rs           /api/admin/* (superadmin/admin)
  reports.rs         report/hide a post; /api/admin/reports*
  directory.rs       optional /api/directory* (mounted only when enabled)
  geocode.rs         hardened Nominatim proxy (rate-limited, cached)
  link_preview.rs    GET  /api/link-preview
  error.rs           shared StatusError -> JSON error mapping
  geocode/           mod.rs + limiter.rs + cache.rs

auth/
  mod.rs             routes + middleware (require_auth/require_admin/require_superadmin)
  password.rs        Argon2id verifier hashing + policy
  email.rs           SMTP (lettre) for verify / reset mail

db/                  conversations, endorsements, notifications, posts, reports, sessions, users
tasks/               expiry, health, directory registration, bundle cleanup
```

## Frontend route tree (`web/src/routes/`)

```
+page.svelte                 home / feed
+layout.svelte, +layout.ts   shell; ssr=false, prerender=false
account/{login,signup,verify,forgot,reset}/   account lifecycle
aid/+page.svelte             aid post list
aid/new/+page.svelte         create a post (incl. click-to-place coordinates)
p/[id]/+page.svelte          flat post permalink
map/+page.svelte             OSM map of located posts
search/+page.svelte          post/user search
messages/+page.svelte        conversation list
messages/[id]/+page.svelte   conversation thread
notifications/+page.svelte
users/[id]/+page.svelte      profile + endorsements
admin/+page.svelte, admin/users/+page.svelte
```

The old `c/**`, `community/**` and `federation/**` route trees were deleted with the
community model.

## Request lifecycle

1. The SPA calls the API through `web/src/lib/api/**` (the single client hub).
2. Axum matches the route in `api/mod.rs`, applies CORS and the security-headers/trace layers.
3. Protected routes run `require_auth`, which loads the session row (user id **and** role, so a
   demotion takes effect immediately) from `sessions`.
4. Handlers validate input and call `db/*`, which issues parameterized sqlx **runtime**
   queries (`sqlx::query` / `query_as`, not the compile-time macros — so no `sqlx prepare`).
5. The handler returns JSON; errors go through `api/error.rs`.

## Data and trust boundaries

- **Server-visible:** email, password verifier, wrapped key bundles, session hashes, directory
  entries. **Never server-visible:** the plaintext password, the password-derived key, the x25519 secret
  in the clear, and message plaintext (the `messages` table stores ciphertext only).
- **Config** (`config.rs`, `config.example.toml`) covers `[server]`, `[database]`, `[node]`,
  `[discovery]`, `[auth]`, `[security]`, `[posts]`, `[admin]`, `[media]`, `[email]`,
  `[registration]`, `[geocode]`. Environment variables override specific fields. There is no
  `jwt_secret` and no `[relay]`/`[federation]`.
- **Map**: `LocationMap.svelte` (Leaflet) is read-only on `/map` and opt-in *pickable* on
  `aid/new`; the tile URL is a component default (operator-configurable later). No relay, no
  map-community credentials.

See `docs/DATABASE.md`, `docs/CRYPTO.md`, `docs/CONVENTIONS.md`, `docs/DEVELOPMENT.md` and
`docs/DEPLOY.md`.
