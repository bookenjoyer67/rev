# Architecture

## System overview

```
┌─────────────────────────────────────────────────────────┐
│ Browser (SPA)                                           │
│ ┌─────────────┐  ┌──────────────┐  ┌──────────────────┐ │
│ │ SvelteKit UI │──│ ServiceWorker│──│ komun-wasm (WASM)│ │
│ │ (web/src/)   │  │ (cache/offln)│  │ (client crypto)  │ │
│ └──────┬───────┘  └──────────────┘  └──────────────────┘ │
└────────┼────────────────────────────────────────────────┘
         │ HTTPS
         ▼
┌─────────────────────────────────────────────────────────┐
│ Axum Server (crates/server/)                             │
│ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐│
│ │ API      │ │ Auth     │ │ REPL     │ │ Relay Bridge ││
│ │ handlers │ │ (JWT)    │ │ (stdin)  │ │ (spawn_relay)││
│ └────┬─────┘ └──────────┘ └──────────┘ └──────┬───────┘│
│      │                                        │         │
│ ┌────▼─────┐                                   │         │
│ │ DB layer │                                   │         │
│ │ (sqlx)   │                                   │         │
│ └────┬─────┘                                   │         │
└──────┼─────────────────────────────────────────┼─────────┘
       │                                         │
       ▼                                         ▼
┌──────────────┐                    ┌─────────────────────┐
│ PostgreSQL   │                    │ piggPin Relay       │
│ (komun DB)   │                    │ (crates/relay/)     │
│              │                    │ WebSocket :9001     │
│ - users      │                    │                     │
│ - communities│                    │ Optional bridges:   │
│ - posts      │                    │  - MQTT (Meshtastic)│
│ - matches    │                    │  - RNode (serial)   │
│ - messages   │                    │  - Reticulum        │
│ - directory  │                    │  - Peer relay       │
│ - invites    │                    └─────────────────────┘
│ - alliances  │
│ - members    │
│ - notifications│
└──────────────┘
```

## Crate dependency graph

```
komun-core (shared models)
    ↑
    ├── komun-server (axum, sqlx, jwt)
    │       ↓
    │   komun-relay (websocket map relay) [optional]
    │
    └── (no direct dep from wasm)

komun-wasm (standalone, no dep on other crates)
    ↓ built into npm package
web/ (SvelteKit, imports komun-wasm via file:../crates/wasm/pkg)
```

## Server module tree (`crates/server/src/`)

```
main.rs              Entry point, AppState, router assembly, bootstrap
config.rs            TOML config deserialization, env overrides
repl.rs              Interactive admin CLI (stats, user mgmt, community mgmt)
relay_bridge.rs      Spawns piggPin relay as embedded service
relay_ops.rs         Crypto ops for relay community creation (ECIES, DEK wrapping)

api/
  mod.rs             Router composition (merge/nest all sub-routers)
  health.rs          GET /api/health
  node.rs            GET /api/node (server metadata)
  admin.rs           Superadmin-only: stats, user CRUD, community CRUD, directory
  communities.rs     Community CRUD, membership, alliances
  posts.rs           Post CRUD (needs/offers/resources), filtering, responding
  conversations.rs   Match threads, messaging
  notifications.rs   Notification CRUD, unread counts
  directory.rs       Server directory listing/registration

auth/
  mod.rs             JWT create/verify, register, recover, me, user keys, middleware

db/
  mod.rs             Module re-exports
  communities.rs     Community queries
  conversations.rs   Match and message queries
  notifications.rs   Notification queries
  posts.rs           Post queries

federation/          Federation primitives (WIP)

tasks/
  mod.rs             Background task spawner
  bundle_cleanup.rs  Cleanup orphaned key bundles
  expiry.rs          Expire old posts
  health.rs          Periodic health checks
  registration.rs    Registration rate limit tracking
```

## Frontend route tree (`web/src/routes/`)

```
+page.svelte             Home — location-based feed with server discovery
+layout.svelte           Persistent shell: nav, hamburger, notifications, onboarding

connect/+page.svelte     Server URL input, known servers, identity recovery
account/+page.svelte     Display name, key display, passphrase, logout
aid/+page.svelte         Community-scoped post listing
aid/new/+page.svelte     Create new post (need/offer/resource)

community/+page.svelte   Community listing + create link
community/create/+page.svelte  Create new community
c/[slug]/+page.svelte    Community detail: posts + map + settings
c/[slug]/map/+page.svelte        piggPin map view
c/[slug]/settings/+page.svelte   Community settings

messages/+page.svelte            Conversation list
messages/[id]/+page.svelte       Single conversation thread
notifications/+page.svelte        Notification feed
federation/+page.svelte           Federation management

admin/+page.svelte                Admin dashboard
admin/communities/+page.svelte    Admin community management
admin/users/+page.svelte          Admin user management
```

## Config system

```
config.example.toml  ──copy──►  config.toml  ──parse──►  Config struct
                                                             │
  .env ──dotenvy──► env vars ──apply_env_overrides───────────┘
```

Config sections: `[server]`, `[database]`, `[node]`, `[discovery]`, `[auth]`, `[federation]`, `[security]`, `[relay]`, `[posts]`, `[admin]`.

Every section has `#[serde(default)]` with sensible `impl Default` in `config.rs`. Env vars override specific fields (documented in `config.example.toml`).

## Request lifecycle

1. **Browser** makes fetch to `/api/...`
2. **Service Worker** intercepts: network-first for API, cache-first for assets (see `service-worker.ts`)
3. **Axum router** (`api/mod.rs`): matches path, applies CORS/tracing middleware
4. **Auth middleware** (`require_auth` or `require_superadmin`): extracts Bearer token, verifies JWT
5. **Handler** (in `api/*.rs`): validates input, calls DB layer
6. **DB layer** (in `db/*.rs`): executes parameterized SQLx queries
7. Response flows back as JSON

## Relay (piggPin) architecture

The relay is an optional embedded WebSocket service on port 9001. It provides real-time map collaboration for communities. Architecture:

```
TCP :9001 ──► relay_bridge::spawn_relay() ──► komun_relay::handler::handle()
                                                    │
                              ┌─────────────────────┼─────────────────────┐
                              ▼                     ▼                     ▼
                          manager.rs            room.rs              handler.rs
                          (room lookup)        (per-room state)     (websocket framing)
                              │                     │
                              ▼                     ▼
                          storage.rs            messages.rs
                          (PersistentStore)     (protocol types)
                              │
                              ▼
                      File system (data/relay/)

Optional bridges (feature-gated):
  mqtt_bridge.rs      ──► Meshtastic via MQTT
  rnode.rs            ──► RNode hardware via serial
  reticulum_bridge.rs ──► Reticulum network stack
  peer_relay.rs       ──► Peer-to-peer relay mesh
```

The `PersistentStore` is shared between the server and relay via `Arc`. The server creates relay communities through `relay_ops::create_relay_community()`.

## Key stores and state management

| Store | Location | Persistence | Purpose |
|---|---|---|---|
| `auth` | `web/src/lib/stores/auth.ts` | localStorage | Per-server auth tokens, keypairs, encrypted bundle |
| `serverState` | `web/src/lib/stores/server.ts` | localStorage | Active server URL, known servers |
| `location` | `web/src/lib/stores/location.ts` | session-only | User location (lat/lon + name) |
| `directories` | `web/src/lib/stores/directories.ts` | hardcoded | Directory server URLs |
| `AppState` | `crates/server/src/main.rs` | in-memory | DB pool, config, relay store |
| `AppState` (relay) | `crates/relay/src/state.rs` | in-memory + disk | Rooms, shares, rate limiter, store |
| `PersistentStore` | `crates/relay/src/storage.rs` | filesystem | Community configs, key material |
