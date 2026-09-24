# Parallel Agent Session Tasks

Scope contract for the two-agent session on the Komun repo. Worktree root for every path below:
the main worktree is `/home/computing/rev` (branch `main`); each session works in its own worktree,
mounted in its container as `/workspace`.

Per-wave detail (task steps, gates, sequencing) lives in
`.hermes/plans/2026-09-22_1620_komun-reshape-auth-marketplace.md`, copied into each worktree as
`.dispatch/SPEC.md`. Agents never run git mutations and never delete files: the orchestrator commits,
merges, rebases and performs every deletion (Claude Code's `acceptEdits` mode denies `rm`/`mv`). An agent
that needs a file gone lists it under `DELETIONS REQUESTED:` in its final report.

## Session A

Branch name: `feature/agent-a`

Worktree directory: `/home/computing/repo-agent-a` (container `agent-repo-agent-a`, mounted at `/workspace`)

Task: The spine. Wave A1 (squash migrations to `001_schema.sql`, rebuild core models, pin enum↔CHECK agreement with a test, delete the piggPin relay), Wave A2 (delete the JWT/key-ceremony auth layer; email + password accounts with verification and email reset; opaque DB sessions; per-IP rate limiting; remove ed25519; frontend auth rewrite), Wave A3 (flatten the API: server-scoped posts, no communities, ciphertext conversations), Wave A6 (frontend flatten: delete `routes/c/**` and friends, create `routes/p/[id]`, flat API client). Owns every hub file. Phase B backend (B-q1, B-q2, B-q3) after Phase A's exit criterion passes.

Files or folders the agent may write to:
- `migrations/**`
- `crates/core/src/**`
- `crates/relay/**`, `crates/server/src/relay_bridge.rs`, `crates/server/src/relay_ops.rs` (deletion requested, performed by the orchestrator)
- `crates/server/src/db/posts.rs`, `crates/server/src/api/communities.rs`, `crates/server/src/db/communities.rs`, `crates/server/src/tests/mod.rs` (card A1.5 only — the minimum repair that makes the workspace compile; A3.1 deletes the first three outright)
- `crates/server/src/main.rs`, `crates/server/src/config.rs`, `crates/server/src/api/**`, `crates/server/src/auth/**`, `crates/server/src/db/**`, `crates/server/src/tasks/**`
- `crates/server/src/rate_limit.rs`, `crates/server/src/sessions.rs` (new), `crates/server/src/repl.rs`, `crates/server/src/security_headers.rs`
- `crates/server/src/tests/**`
- `crates/wasm/src/lib.rs`
- `Cargo.toml`, `Cargo.lock`, `crates/wasm/Cargo.toml`, `crates/server/Cargo.toml`, `crates/relay/Cargo.toml`
- `web/src/lib/api/**`, `web/src/lib/stores/auth.ts`, `web/src/lib/crypto.ts`
- `web/src/routes/**` except the files listed under Session B
- `web/src/tests/**`

Files or folders the agent may read but not write to:
- `web/src/lib/components/LocationMap.svelte`, `web/src/routes/map/**` (Session B's new files)
- `crates/server/src/api/node.rs`, `crates/server/src/api/directory.rs`, `crates/server/src/tasks/registration.rs` (Session B's files: the wave cards always assigned them to B, and the ownership map contradicted them until 2026-09-23)
- `docs/**`, `README.md`, `AGENTS.md`
- `deploy/**`, `config.example.toml`
- `sandbox/**`, `Dockerfile`, `setup.md`, `.dispatch/**`
- `web/package.json`, `web/package-lock.json` (dependency manifests are orchestrator-only)

Commands the agent may run:
- `cargo check`, `cargo build --workspace`, `cargo test --workspace`, `cargo test -p komun-core`, `cargo clippy --release -- -D warnings`, `cargo fmt`
- `wasm-pack build crates/wasm --target web`
- `cargo run --bin komun-server`
- `curl` against `http://127.0.0.1:3000` (the running server)
- `psql` against its own DB only (`komun_a` on `komun-db-a`) — `DATABASE_URL` is NOT exported in the sandbox, the literal connection string is in the prompt card; read/write schema probes and fixtures
- `cd web && npm run check && npm run build && npx vitest run`, `node`
- `grep`, `rg`, `find`, `git status`, `git diff`, `git log` (read-only git)
- NOT permitted: any git mutation (commit, push, checkout, restore, stash, rebase, reset, worktree), `rm` / `mv` (deletions are the orchestrator's — request them instead), `cargo add` / `npm install` / any network fetch, `docker`, `sudo`, writes outside `/workspace`, and any write to Session B's DB

Definition of done:
- A1 (card A1.5 included): `cargo test -p komun-core` green, `cargo build --workspace` clean, `cargo build --workspace --all-targets` clean (the test build is what catches `tests/mod.rs`), `grep -ri piggpin crates/` empty (the whole-repo grep belongs to A6/A7 — after A1's deletions its only survivors are B-owned docs and A6-owned web files), `psql -c "\dt"` lists exactly the schema in SPEC Part 1.4, every negative insert rejected (negative price, `market_listed` on a `need`, lowercase currency, `item_condition='mint'`, rating 6, duplicate review, offer kind `'bid'`, `role='root'`, `visibility='federated'`, case-differing duplicate email, unknown category slug, `DELETE` of a category in use), and the enum↔CHECK test provably fails when one CHECK entry is deleted by hand
- A2: `grep -rn "jsonwebtoken\|JWT_SECRET\|recovery_id\|compute_recovery_id" crates/` empty; `grep -rn "ed25519" crates/ web/src` returns comments only; no user-facing `passphrase`; `cargo test --workspace` green; `cargo clippy --release -- -D warnings` zero warnings; `wasm-pack build` and `npm run build` succeed; vitest green; and by hand: signup → verify → login, unverified cannot post, duplicate email rejected in any case, unknown-email and bad-password indistinguishable, 429 after repeated failures with a spoofed `X-Forwarded-For` ignored, reset token single-use and expiring, reset with the recovery code restores a pre-reset message and without it does not, change-password preserves the x25519 secret, revoked/expired sessions 401, demoted admin loses access immediately, no endpoint returns key material or the recovery code after signup
- A3/A6: server boots; `/api/posts`, `/api/search`, a conversation thread and `/auth/me` return no `community`/`slug`; `/api/alliances` 404s; no plaintext message column in the schema; `grep -rn "slug" web/src` empty; `npm run check`, `npm run build` and `npx vitest run` green
- Every change is inside the write list above, hub requests from Session B are applied and the build is re-verified after each, and the final message reports `FILES WRITTEN`, `GATES RUN` with raw output, `BLOCKED`, and `DELETIONS REQUESTED` when anything must be removed
- Phase B cards (B-q1, B-q2, B-q3) are only started after the A7.3 bootstrap end-to-end run passes on a fresh DB

## Session B

Branch name: `feature/agent-b`

Worktree directory: `/home/computing/repo-agent-b` (container `agent-repo-agent-b`, mounted at `/workspace`)

Task: Detached leaves only. Wave A4 (harden the Nominatim geocode proxy with a 1 req/s limiter, a cache and a configurable `User-Agent`; the Leaflet `LocationMap.svelte` component and the `/map` route, with the post-detail map added only after Session A's A6 lands), Wave A5 (delete alliances and the federation module; strip community fields from node info, directory advertisement and node registration), Wave A7.1/A7.2 (rewrite the docs for the new account model, SMTP, flat model and OSM map, including the honest E2E limitation; update the deploy assets and `config.example.toml`). Phase B frontend (B-q4) after Phase A's exit criterion passes.

Files or folders the agent may write to:
- `crates/server/src/api/geocode.rs`, plus a new module for the limiter/cache
- `web/src/lib/components/LocationMap.svelte` (new), `web/src/routes/map/**` (new)
- `web/src/lib/components/MarketCard.svelte`, `web/src/lib/components/OfferPanel.svelte`, `web/src/lib/components/DealReviewModal.svelte` (new)
- `web/src/routes/market/**`, `web/src/lib/api/market.ts`, `web/src/lib/api/categories.ts` (new)
- `docs/ARCHITECTURE.md`, `docs/CONVENTIONS.md`, `docs/CRYPTO.md`, `docs/DATABASE.md`, `docs/DEVELOPMENT.md`
- `README.md`, `AGENTS.md`
- `deploy/nginx-komun.conf`, `deploy/komun.initd`, `deploy/setup.sh`, `deploy/seed.sql`
- `config.example.toml`
- `web/package.json`, `web/package-lock.json` (only via the orchestrator's `npm install leaflet` pre-warm; never by running a network install itself)
- `crates/server/src/api/node.rs`, `crates/server/src/api/directory.rs`, `crates/server/src/tasks/registration.rs` (B3: strip the community fields)
- deletions (requested, performed by the orchestrator): `crates/server/src/api/alliances.rs`, `crates/server/src/db/alliances/**`, `crates/server/src/federation/**`
- `.dispatch/hub-requests/**` (its only permitted way to request changes to files it does not own)

Files or folders the agent may read but not write to:
- `crates/server/src/api/mod.rs`, `crates/server/src/config.rs`, `crates/server/src/main.rs`, `crates/server/src/auth/**`, `crates/server/src/db/**` except the deletion paths above, `crates/server/src/tasks/**` except `registration.rs`
- `crates/core/src/**`, `crates/wasm/**`, `crates/relay/**`
- `migrations/**`, `Cargo.toml`, `Cargo.lock`, `crates/*/Cargo.toml`
- `web/src/lib/api/**` other than `market.ts` / `categories.ts`, `web/src/lib/stores/**`, `web/src/lib/components/**` except the three new components, `web/src/routes/**` except `map/**` and `market/**`
- `web/src/tests/**`
- `sandbox/**`, `Dockerfile`, `setup.md`, `.dispatch/SPEC.md`

Commands the agent may run:
- `cargo check`, `cargo build --workspace`, `cargo test --workspace`, `cargo clippy --release -- -D warnings`, `cargo fmt`
- `cargo run --bin komun-server`
- `curl` against `http://127.0.0.1:3000` (e.g. confirming `/api/alliances` 404s, `/api/node` shape)
- `psql` against its own DB only (`komun_b` on `komun-db-b`) — `DATABASE_URL` is NOT exported in the sandbox; no `DELETE`/`TRUNCATE`/`DROP`
- `cd web && npm run check && npm run build && npx vitest run`, `node`
- `grep`, `rg`, `find`, `git status`, `git diff`, `git log` (read-only git)
- NOT permitted: any git mutation (commit, push, checkout, restore, stash, rebase, reset, worktree), `rm` / `mv` (opencode *can* delete but is under the same rule — deletions are the orchestrator's), `cargo add` / `npm install` / any network fetch, `docker`, `sudo`, writes outside `/workspace`, and every file in the read-only list above (a hub request is the substitute)

Definition of done:
- B1: `cargo build --workspace` clean after its hub request is applied; a test proving the geocode limiter queues the second request instead of issuing it; a test asserting the `User-Agent` contains the configured contact; a bogus contact still boots; `config.example.toml` untouched
- B2: `npm run check`, `npm run build`, `npx vitest run` green; a vitest case asserting the attribution renders; `/map` renders tiles with attribution visible; a bogus `tile_url` degrades to an empty map, never a broken page; the post-detail insertion is either done after A6 (in `web/src/routes/p/[id]/+page.svelte`) or reported as blocked
- B3: `cargo build --workspace` clean after its hub request is applied; `grep -rn "alliance\|federation" crates/server/src` returns only comments; the three deletion paths are gone; `/api/alliances` returns 404 and `/api/node` carries neither `communities_count` nor `federation_enabled`
- B4: docs match the shipped code — every command in them was executed once; no doc mentions `jwt_secret`, `piggpin`, `relay` or a passphrase except as a removal note; the server boots with `config.example.toml` (DB URL filled in) and starts clean
- B-q4 (after Phase A exits): `npm run check`, `npm run build`, `npx vitest run` green, plus a hand walkthrough of listing → offer → accept → complete → review
- Every change is inside the write list above; anything else was requested as a hub request rather than edited; the final message reports `FILES WRITTEN`, `GATES RUN` with raw output, `BLOCKED`, and `DELETIONS REQUESTED` when anything must be removed
