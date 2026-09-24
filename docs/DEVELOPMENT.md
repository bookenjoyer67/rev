# Development guide

How to build, run and test Komun, in the order that actually works. The conventions the
code must follow are in `docs/CONVENTIONS.md`; the architecture is in
`docs/ARCHITECTURE.md`, the schema in `docs/DATABASE.md`, the crypto in `docs/CRYPTO.md`,
and self-hosting in `docs/DEPLOY.md`.

## Prerequisites

| Tool | Version used here | Notes |
|---|---|---|
| Rust | 1.95.0 (`rustc`/`cargo`) | one Cargo workspace: `komun-core`, `komun-server`, `komun-wasm` |
| `wasm-pack` | present | builds `crates/wasm` to `crates/wasm/pkg/` (browser target) |
| Node.js | 22.x (v22.23.2) | |
| npm | 10.x (10.9.8) | |
| PostgreSQL | 16 | the server runs migrations itself at startup |
| `psql` / `sha384sum` | 16 / coreutils | provisioning a database (below) |

`config.toml` is **gitignored** and therefore absent from a fresh checkout. The server
loads it from `config.toml` in the working directory (or `KOMUN_CONFIG=/path/to/other`).
Copy the template and edit it:

```bash
cp config.example.toml config.toml
# minimum: point [database] url at your Postgres
```

Everything else in the template has a working default: `[registration]` runs without SMTP
(`require_email_verification = false`), `[discovery]` mounts no directory routes, and the
optional `[market]` `default_currency` is unset, which is the intended default — see the
currency precedence below.

The example config must boot as shipped; see the boot check below.

## Provisioning a database (the exact order, and why)

`migrations/001_schema.sql` **is** the schema. It is applied by hand once, and the migrator
takes over from `002` onward. The order matters:

```bash
# 1. Create the database and role.
psql "postgres://postgres@localhost:5432/postgres" <<'SQL'
CREATE USER komun WITH PASSWORD 'change-me';
CREATE DATABASE komun OWNER komun;
SQL

# 2. Create the migrations bookkeeping table (the migrator's own shape).
psql "postgres://komun:change-me@localhost:5432/komun" <<'SQL'
CREATE TABLE _sqlx_migrations (
    version        BIGINT PRIMARY KEY,
    description    TEXT NOT NULL,
    installed_on   TIMESTAMPTZ NOT NULL DEFAULT now(),
    success        BOOLEAN NOT NULL,
    checksum       BYTEA NOT NULL,
    execution_time BIGINT NOT NULL
);
SQL

# 3. Load the schema baseline.
psql "postgres://komun:change-me@localhost:5432/komun" -f migrations/001_schema.sql

# 4. Bookmark it as already applied, with the file's real sha384.
CHECKSUM=$(sha384sum migrations/001_schema.sql | cut -d' ' -f1)
psql "postgres://komun:change-me@localhost:5432/komun" -c \
  "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time) \
   VALUES (1, 'schema', true, decode('$CHECKSUM', 'hex'), 0);"

# 5. Boot the server; the migrator applies 002 and everything after it.
cargo run --bin komun-server
```

**Why this order.** SQLx creates `_sqlx_migrations` inside the same transaction it runs a
migration in, and rolls that transaction back when a statement fails. If you let the
migrator run `001` against an empty database it succeeds and records the row itself — but if
you load `001` by hand *first*, the migrator would then try to create the already-existing
tables, fail, and roll back the bookkeeping table with them. So: create the table, load the
schema, insert the bookmark whose `checksum` equals `sha384sum migrations/001_schema.sql`,
and let the migrator start at `002`. A wrong or missing checksum makes every later boot fail.

### Optional demo seed

`deploy/seed.sql` adds a handful of demo accounts, aid posts and two marketplace posts (one
`listing`, one `want`) so a fresh instance has a feed, map pins and a browseable marketplace.
It is optional and idempotent (`ON CONFLICT (id) DO NOTHING`); it does **not** touch the
taxonomy, which `001_schema.sql` already seeds:

```bash
psql "$DATABASE_URL" -f deploy/seed.sql
# the 23 categories above are the migration's, not this file's:
psql "$DATABASE_URL" -tAc "select scope, count(*) from categories group by scope order by scope"
# aid|2
# both|6
# market|15
```

## Migration rules (easy to get wrong)

- **`migrations/001_schema.sql` is FROZEN.** It is checksum-bookmarked in every existing
  database; changing one byte makes every server refuse to boot with a checksum mismatch.
- **Schema changes are additive files** (`002_*.sql`, `003_*.sql`, …). Never edit an applied
  migration. `crates/core/src/tests.rs` parses `001_schema.sql` and pins the Rust enums to
  its `CHECK` lists, so a new CHECK without a matching enum fails the test by design.

## Build order (critical)

The wasm package must exist before the frontend is installed:

```
1. wasm-pack build crates/wasm --target web      # produces crates/wasm/pkg/
2. cd web && npm install && npm run build        # requires crates/wasm/pkg/ FIRST
3. cargo build --release --bin komun-server      # backend, serves web/build/
```

**The trap:** `web/package.json` depends on `"komun-wasm": "file:../crates/wasm/pkg"`.
`npm install` resolves that local path, so if `crates/wasm/pkg/` does not exist yet the
install fails. Build the wasm package **before** installing, and whenever `crates/wasm`
changes rebuild both the package and the frontend.

If you changed nothing in `crates/wasm`, you do not need steps 1–2 to work on the backend.

## Commands

Run from the repository root unless noted. The agent containers need cargo on `PATH`:

```bash
export PATH=/usr/local/cargo/bin:$PATH
```

### Backend

```bash
cargo build --workspace --all-targets   # compile every crate, tests included
cargo test --workspace                  # unit + integration tests
cargo clippy --release -- -D warnings   # the zero-warning standard
cargo run --bin komun-server            # start the API server on [server].port (default 3000)
```

`cargo run --bin komun-server` needs a reachable Postgres (`[database] url`) and serves the
API under `/api`. When stdin is a terminal it also starts the REPL (`help`).

### WASM + frontend

```bash
wasm-pack build crates/wasm --target web     # emits crates/wasm/pkg/
cd web
npm install                                  # needs crates/wasm/pkg/ to exist (see trap above)
npm run check                                # svelte-kit sync + svelte-check
npm run build                                # adapter-static -> web/build/
npx vitest run                               # component/unit tests
```

## Running the server for a runtime gate

A runtime gate needs an isolated config and a scratch working directory (the server reads
`config.toml` from its cwd, and writes media under it):

```bash
mkdir -p target/runtimecheck && cp config.toml target/runtimecheck/config.toml
# edit target/runtimecheck/config.toml:
#   [server] port = <free port>              (e.g. 3051)
#   [registration] require_email_verification = false   (or configure [email] — see below)
#   [discovery] directory_enabled = true     (only if the gate touches /api/directory*)
# then boot the built binary from that directory:
( cd target/runtimecheck && exec /workspace/target/debug/komun-server >boot.log 2>&1 ) &
curl -s http://127.0.0.1:<port>/api/health      # {"service":"komun","status":"ok",...}
```

`[discovery] directory_enabled = false` means the `/api/directory*` routes are **not mounted
at all**, so they 404 — that is a configuration choice, not a routing bug. `require_email_verification
= true` without `[email] smtp_host` + `from` makes the server refuse to start; that is
enforced by `Config::validate_registration` and is the first thing a fresh clone hits.

### Marketplace runtime check

The marketplace adds no new transport: it is the same flat `/api` surface, with `listing`/`want`
post kinds, offers on a match thread, and reviews against a completed deal. The reference is in
`docs/ARCHITECTURE.md` ("Marketplace" and "HTTP API"); this is what can be checked without a
browser. With a server running (the recipe above) and `BASE` pointing at it:

```bash
BASE=http://127.0.0.1:3000

# The taxonomy is a UNION, not an equality: `market` is the 15 market rows + the 6 `both` rows.
curl -s "$BASE/api/categories?scope=market"       # 21 rows
curl -s "$BASE/api/categories?scope=aid"          # 8 rows (2 aid + 6 both)
curl -s "$BASE/api/categories?scope=both"         # 6 rows
curl -s -w '\nHTTP %{http_code}\n' "$BASE/api/categories?scope=commercial"   # 400, names aid, market, both

# A market browse is the same feed with a kind filter.
curl -s "$BASE/api/posts?kind=listing"            # the seeded listing (empty without deploy/seed.sql)

# Offer, status and review writes need a session (Bearer token from POST /api/auth/signin) and a
# market thread; without one the mutating routes answer 401.
curl -s -o /dev/null -w '%{http_code}\n' -X POST \
  "$BASE/api/conversations/00000000-0000-0000-0000-000000000000/offers" \
  -H 'Content-Type: application/json' -d '{"kind":"offer","amount_cents":100}'   # 401
curl -s -o /dev/null -w '%{http_code}\n' -X POST \
  "$BASE/api/matches/00000000-0000-0000-0000-000000000000/reviews" \
  -H 'Content-Type: application/json' -d '{"rating":5}'                          # 401
```

The end-to-end walk that proves the deal lifecycle — two accounts (one verified), post a listing,
respond, `offer` → `counter` → `accept`, `completed`, two reviews, the profile aggregate, and the
refusals (a second review `409`, a review on an incomplete deal `409`) — is the Phase B exit gate
and needs a real client (the encrypted opening message and the key bundles are client-side).

**How a price gets a currency.** A `listing`/`want` keeps the `currency` it was created with; with
none, the server fills in `[market] default_currency` when that key is set, otherwise the post
simply has no currency. At negotiation time a deal needs a unit, so the accepted order is the
offer's own `currency`, else the post's, else `[market] default_currency`, and with none of the
three a `400` names the missing currency. `[market] default_currency` is unset by default and a
malformed value (e.g. `"cad"`) refuses to start, naming the key, the value and the remedy.

## Measure a lint/tool gate honestly

- **`cargo clippy` caches.** A second run with no source change prints nothing at all — that
  is a cache hit, not a clean lint. `touch` a source file (e.g. `touch crates/server/src/main.rs`)
  before measuring.
- **`cargo build` never shows clippy lints.** "No new warnings" must come from
  `cargo clippy -p komun-server --no-deps`, not from the build.
- **`clippy --release -- -D warnings` is the standing gate** and it passes; the way to keep
  it there is to fix the code, never to add `#[allow]` (see `docs/CONVENTIONS.md` §6).
- **Compare numbers, not just exit codes**, and re-measure after each change. The frontend is
  currently at zero: `npm run check` → 0 errors / 0 warnings, `npm run build` green,
  `npx vitest run` → all passing.

## Not verifiable in a sandbox

The agent containers have **no internet**, so several things can only be checked on a machine
with egress. Do not claim them from inside a sandbox:

- **Map tiles.** Leaflet fetches raster tiles over the network; with no egress the map
  initialises and renders its attribution, but no tile loads. Verify tile loading on a
  networked host.
- **Live SMTP.** Email (verification links, password reset) needs a real `[email]`/SMTP
  server; delivery cannot be exercised offline.
- **`wasm-pack build` on first use.** Building the wasm package needs the
  `wasm32-unknown-unknown` target and `wasm-bindgen`, downloaded on first use. In these
  containers that target is absent and cannot be fetched, so `crates/wasm/pkg/` is normally
  **pre-built by whoever has network access** and the frontend is installed on top of it.
- **`npm install`** likewise cannot run offline; `web/node_modules/` is pre-warmed.

## Environments

The server reads `config.toml` by default and honours env overrides including `DATABASE_URL`,
`KOMUN_CONFIG`, `KOMUN_BIND_ADDRESS`, `KOMUN_PORT` and `KOMUN_NODE_NAME` (see
`crates/server/src/config.rs`). There is no JWT secret and no signing key: sessions are opaque
database rows (SPEC A10).
