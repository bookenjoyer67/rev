# Development guide

How to build, run and test Komun, in the order that actually works. The conventions the
code must follow are in `docs/CONVENTIONS.md`; the frozen design is `.dispatch/SPEC.md`.

## Prerequisites

| Tool | Version used here | Notes |
|---|---|---|
| Rust | 1.95.0 (`rustc`/`cargo`) | workspace is a single Cargo workspace: `komun-core`, `komun-server`, `komun-wasm`, `komun-relay` (relay is removed by A1.4) |
| `wasm-pack` | present | builds `crates/wasm` to `crates/wasm/pkg/` (browser target) |
| Node.js | 22.x (v22.23.2) | |
| npm | 10.x (10.9.8) | |
| PostgreSQL | 16 | the server runs migrations itself at startup |
| `psql` | 16 client | for creating a fresh database / probes |

`config.toml` is **gitignored** and therefore absent from a fresh checkout. The server
loads it from `config.toml` in the working directory (or `KOMUN_CONFIG=/path/to/other`).
Copy the template and edit it:

```bash
cp config.example.toml config.toml
# minimum: point [database] url at your Postgres
```

The example config is maintained by the docs/deploy card (B4); it must boot as shipped.

## Schema: a fresh database is one file

`migrations/001_schema.sql` **is** the schema (SPEC §1.4). To create a database from
scratch, apply that one file once:

```bash
psql "$DATABASE_URL" -f migrations/001_schema.sql
```

You can also just start the server against an empty database — SQLx applies the migration
at startup. After the squash, schema changes are new numbered migrations
(`migrations/002_*.sql`, …); never edit an applied migration (see `docs/CONVENTIONS.md` §2).

## Build order (critical)

The wasm package must exist before the frontend is installed:

```
1. wasm-pack build crates/wasm --target web      # produces crates/wasm/pkg/
2. cd web && npm install && npm run build        # requires crates/wasm/pkg/ FIRST
3. cargo build --release --bin komun-server      # backend, serves web/build/
```

**The trap:** `web/package.json` depends on `"komun-wasm": "file:../crates/wasm/pkg"`.
`npm install` resolves that local path, so if `crates/wasm/pkg/` does not exist yet the
install fails. Build the wasm package **before** installing, every time `crates/wasm`
changes, rebuild both the package and the frontend.

If you changed nothing in `crates/wasm`, you do not need step 1 or step 2 to work on the
backend.

## Commands

Run from the repository root unless noted. The agent containers need cargo on `PATH`:

```bash
export PATH=/usr/local/cargo/bin:$PATH
```

### Backend

```bash
cargo build --workspace                 # compile every crate
cargo test --workspace                  # unit + integration tests
cargo clippy --release -- -D warnings   # the zero-warning standard
cargo run --bin komun-server            # start the API server on [server].port (default 3000)
```

`cargo run --bin komun-server` needs a reachable Postgres (`[database] url`) and serves
the API under `/api`. When stdin is a terminal it also starts the REPL (`help`).

### WASM + frontend

```bash
wasm-pack build crates/wasm --target web     # emits crates/wasm/pkg/
cd web
npm install                                  # needs crates/wasm/pkg/ to exist (see trap above)
npm run check                                # svelte-kit sync + svelte-check
npm run build                                # adapter-static -> web/build/
npx vitest run                               # component/unit tests
```

## Measure a lint/tool gate honestly

This has bitten the project more than once, so it is a rule, not advice:

- **`cargo clippy` caches.** A second run with no source change prints nothing at all —
  that is a cache hit, not a clean lint. `touch` a source file (e.g.
  `touch crates/server/src/main.rs`) before measuring.
- **`cargo build` never shows clippy lints.** "No new warnings" must come from
  `cargo clippy -p komun-server --no-deps`, not from the build.
- **`svelte-check` reports a frozen inherited baseline** (currently 14 errors / 37
  warnings in 8 files, all in the old community UI that A6 deletes) until the frontend
  flatten lands. A web change is judged by whether it *increases* those counts, not by
  reaching zero. See `.dispatch/FRONTEND-BASELINE.md`; the backend lint baseline is in
  `.dispatch/BACKEND-LINT-BASELINE.md`.
- **Compare counts, not just exit codes.** `npm run check` and `npx vitest run` exit
  non-zero on the frozen baseline.

## Not verifiable in a sandbox

The agent containers have **no internet**, so several things can only be checked on a
machine with egress. Do not claim them from inside a sandbox:

- **Map tiles.** Leaflet fetches raster tiles over the network; with no egress the map
  initialises and renders its attribution but no tile loads. Verify tile loading on a
  networked host.
- **Live SMTP.** Email (verification links, password reset) needs a real `[email]`/SMTP
  server; delivery cannot be exercised offline.
- **`wasm-pack build` on first use.** Building the wasm package needs the
  `wasm32-unknown-unknown` target and `wasm-bindgen`, which must be downloaded on first
  use. In these containers that target is absent and cannot be fetched, so
  `crates/wasm/pkg/` is normally **pre-built by whoever has network access** and the
  frontend is installed on top of it.
- **`npm install`** likewise cannot run offline; `web/node_modules/` is pre-warmed.

## Environments

The server reads `config.toml` by default and honours the same env overrides as before,
including `DATABASE_URL`, `KOMUN_CONFIG`, `KOMUN_BIND_ADDRESS`, `KOMUN_PORT` and
`KOMUN_NODE_NAME` (see `crates/server/src/config.rs`). `[auth] jwt_secret` is gone with
the JWT layer (SPEC A10).
