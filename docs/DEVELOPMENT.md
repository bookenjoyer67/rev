# Development guide

## Prerequisites

- **Rust** 1.82+ (edition 2024)
- **Node.js** 22+
- **wasm-pack** — `cargo install wasm-pack`
- **PostgreSQL** 16 (or Docker for the DB container)
- **Docker** + docker-compose (optional, for full containerized workflow)

## First-time setup

```bash
# 1. Clone and enter
git clone <repo-url> komun
cd komun

# 2. Create config
cp config.example.toml config.toml
# Edit config.toml — at minimum set a proper jwt_secret

# 3. Start PostgreSQL (choose one)
docker compose up db -d          # containerized
# OR use your local PostgreSQL

# 4. Build WASM crypto library
wasm-pack build crates/wasm --target web

# 5. Build frontend
cd web
npm install
npm run build
cd ..

# 6. Build and run server
cargo build --release --bin komun-server
cargo run --release --bin komun-server
```

The server starts on `http://localhost:3000` (or whatever `bind_address`/`port` you configured). It auto-runs SQLx migrations on startup.

## Build order (critical)

```
wasm-pack build crates/wasm --target web    # 1. WASM crypto library
  ↓ produces crates/wasm/pkg/
cd web && npm install && npm run build     # 2. SvelteKit static export
  ↓ produces web/build/
cargo build --bin komun-server              # 3. Rust backend
```

The server expects `web/build/` to exist at runtime (configured via `static_dir` in config.toml). If you're running API-only (no frontend), remove `static_dir` from config.

## Development workflows

### Backend only (API changes)

```bash
# Run with auto-reload (cargo watch)
cargo watch -x 'run --bin komun-server'

# Check compilation
cargo check

# Run REPL commands (stats, list-users, etc.)
cargo run --bin komun-server
# Type 'help' at the komun> prompt
```

### Frontend only (UI changes)

```bash
cd web
npm run dev        # Vite dev server with HMR
# Opens on http://localhost:5173
# Requires the backend running separately for API calls
```

The Vite config allows `komun.buzz` and `localhost` as hosts and permits filesystem access to `..` (for the WASM package).

### WASM crypto changes

```bash
wasm-pack build crates/wasm --target web
cd web && npm run build && cd ..
cargo run --bin komun-server
```

If you're using Vite dev server, you may only need the `wasm-pack build` step — Vite picks up the `pkg/` changes.

### Config changes

- Edit `config.toml` for runtime changes (restart server)
- If adding new config fields, update both:
  - `config.rs` (struct definition + defaults + env overrides)
  - `config.example.toml` (documented template)

### Database changes

- Add new migration files in `migrations/` with sequential numbering (`003_*.sql`, `004_*.sql`, etc.)
- Never edit existing migrations — they are immutable once applied
- If you add new SQLx queries, run `cargo sqlx prepare` to update the offline query cache for Docker builds

## Docker workflow

```bash
# Build and run everything
docker compose up --build

# Rebuild just the app (after code changes)
docker compose up --build app

# Tear down
docker compose down

# Tear down including volumes (resets DB)
docker compose down -v
```

The Dockerfile has three stages:
1. **builder**: Compiles `komun-server` in release mode with `SQLX_OFFLINE=true`
2. **frontend**: `npm ci` + `npm run build` for SvelteKit static export
3. **runtime**: Slim Debian with the binary + `web/build/` + `migrations/`

## Lint and type check

```bash
# Rust
cargo check                      # type check all crates
cargo clippy                     # lint (if installed)

# Frontend (SvelteKit + TypeScript)
cd web
npm run check                    # svelte-kit sync + svelte-check
```

There are no ESLint, Prettier, or Biome configs. No Tailwind.

## Database connection

Default dev connection string (in `.env` and `config.toml`):

```
postgres://komun:komun@localhost:5432/komun
```

Docker Compose creates this PostgreSQL user/database automatically. The DB container uses `POSTGRES_USER=komun`, `POSTGRES_PASSWORD=komun`, `POSTGRES_DB=komun`.

## Environment variables

| Variable | Purpose | Set in |
|---|---|---|
| `DATABASE_URL` | PostgreSQL connection string | `.env` or docker-compose |
| `BIND_ADDR` | Server listen address (backwards-compat: `host:port`) | `.env` or docker-compose |
| `JWT_SECRET` | JWT signing secret | `.env` |
| `KOMUN_CONFIG` | Path to config file (default: `config.toml`) | optional |
| `KOMUN_BIND_ADDRESS` | Override `[server].bind_address` | optional |
| `KOMUN_PORT` | Override `[server].port` | optional |
| `KOMUN_NODE_NAME` | Override `[node].name` | optional |

## Production deployment notes

- Change `jwt_secret` to a random base64 string (e.g., `openssl rand -base64 32`)
- Set `registration_mode` to `"approval"` to prevent open registration
- Set `allowed_origins` to your specific domain (not `"*"`)
- Configure TLS termination at a reverse proxy (nginx, Caddy) — the Axum server listens on plain HTTP
- Set `[discovery].listed = true` and point `directory_url` to your public URL to appear in the server directory
- The relay WebSocket URL (`external_url`) needs a `wss://` URL if behind a TLS proxy
