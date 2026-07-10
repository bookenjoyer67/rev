# Komun

Federated mutual aid discovery platform. Communities post needs, offers, and resources. Encrypted conversations match people who can help.

AGPL-3.0.

## Architecture

```
Browser (SvelteKit SPA)
        │
        ▼
┌─────────────────┐     ┌──────────────┐
│  Axum HTTP API  │────▶│  PostgreSQL   │
│  (port 3000)    │     │               │
│                 │     └──────────────┘
│  ┌───────────┐  │
│  │ piggPin   │  │──── WebSocket relay
│  │ relay     │  │     (port 9001)
│  └───────────┘  │
└─────────────────┘
        │
   ┌────┴────┐
   │  WASM   │  Client-side crypto
   │  crypto │  (ed25519, x25519,
   └─────────┘   ChaCha20, Argon2)
```

## Crates

| Crate | Purpose |
|-------|---------|
| `crates/core` | Shared data models (Community, Post, Member, MatchThread) |
| `crates/server` | Axum HTTP API, JWT auth, DB queries, relay bridge, REPL |
| `crates/wasm` | Client-side crypto compiled to WASM (ed25519, x25519, ChaCha20Poly1305, Argon2, BIP39) |
| `crates/relay` | piggPin WebSocket map relay — real-time location sharing per community |

## Quickstart

### Requirements

- Rust (stable)
- Node.js 18+
- PostgreSQL 16
- wasm-pack (`cargo install wasm-pack`)

### Local dev

```bash
git clone https://git.komun.buzz/Book-Enjoyer/rev.git
cd rev

# Start PostgreSQL
docker compose up db -d

# Copy and edit config
cp config.example.toml config.toml

# Set up the database
createdb komun
# or: docker compose exec db createdb -U komun komun

# Build WASM crypto
wasm-pack build crates/wasm --target web

# Build frontend
cd web && npm install && npm run build && cd ..

# Run
cargo run --bin komun-server
```

Opens on `http://localhost:3000`.

### Docker (full stack)

```bash
cp config.example.toml config.toml
# edit config.toml — set database.url, auth.jwt_secret, etc.
docker compose up --build
```

## Security Model

### Crypto boundaries

- **Client-side (WASM):** ed25519 key generation/signing, x25519 ECDH, ChaCha20Poly1305 encryption, Argon2 key derivation, BIP39 recovery codes
- **Server-side:** JWT HS256, TLS termination
- **Never leaves client:** ed25519 secret key, x25519 secret key, passphrase, recovery code
- **Server stores:** Public keys, encrypted key bundles, recovery_id (Argon2 hash), recovery_code_hash

### Key hierarchy

1. Passphrase → Argon2 → wrap key → decrypts key bundle
2. ed25519 key → signs challenges → JWT token (includes role claim)
3. x25519 key → ECDH → conversation encryption keys (ChaCha20Poly1305)
4. Recovery code (BIP39 12 words) → Argon2 → recovery_code_hash → server verifies

### Auth flow

1. Client generates ed25519 + x25519 keypair in WASM
2. Server issues challenge → client signs with ed25519 → server verifies → JWT issued
3. JWT contains user_id (sub) and role — no DB query needed for authorization
4. Optional: passphrase encrypts key bundle for server-side recovery
5. Optional: BIP39 recovery code as backup identity factor

## Project layout

```
rev/
├── crates/
│   ├── core/src/models/     Data models (Community, Post, Member, etc.)
│   ├── server/src/
│   │   ├── api/             REST endpoints (communities, posts, conversations)
│   │   ├── auth/            JWT, challenge-auth, middleware
│   │   ├── db/              Database queries per entity
│   │   ├── config.rs        Configuration (TOML + env overrides)
│   │   └── tasks/           Background jobs (bundle cleanup, expiry)
│   ├── wasm/src/            Crypto compiled to WASM
│   └── relay/src/           piggPin WebSocket map relay
├── web/                     SvelteKit 5 SPA (static adapter)
├── migrations/              SQLx migrations (numbered, additive)
├── docker/                  Multi-stage Dockerfile
├── config.example.toml      Documented config template
└── scripts/                 Utility scripts
```

## Tests

```bash
cargo test --workspace
```

110 tests across the workspace. Server tests cover auth tokens, challenge flow, config parsing. Core tests cover model serialization/deserialization and type safety.

DB integration tests need a running PostgreSQL instance and `DATABASE_URL` set in the environment.

## Conventions

- UUIDv7 for all primary keys (time-sortable)
- `cargo check` must pass before committing
- Migrations are additive — never edit existing migration files
- Secret keys never logged, never stored server-side in plaintext
- `config.toml` and `.env` are gitignored — use `config.example.toml` as reference
- Svelte 5 runes only: `$state()`, `$derived()`, `$effect()`, `$props()`, `onclick={handler}`
- No `$:`, `export let`, or `on:click`

## License

AGPL-3.0-or-later. See [LICENSE](LICENSE).
