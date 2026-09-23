# CARD A2a — Old auth deleted; password, signup, sessions

**Owner:** Agent A (Claude Code/opus) · **Branch:** `feature/agent-a` · **Worktree:**
`/home/computing/repo-agent-a` mounted as `/workspace` in container `agent-repo-agent-a` ·
**Depends on:** A1 + A1.5 (DONE, verified by the orchestrator) · **Blocks:** A2b, A3, A6.

## What A1 and A1.5 already did — do not redo any of it

- `migrations/` is now a single `001_schema.sql` (16 tables, seeded categories, the label-indexing FTS
  trigger); the 14 old migrations are deleted. `crates/core/src/tests.rs` parses that file at test time and
  pins every DB-backed enum to its `CHECK` list. 20/20 green.
- `crates/relay/`, `relay_bridge.rs`, `relay_ops.rs`, `models/community.rs`, `models/member.rs` are gone;
  `[relay]` config, `AppState.relay_store` and the spawn block are gone.
- `db/posts.rs`, `db/communities.rs`, `api/communities.rs`, `tests/mod.rs` were repaired mechanically so the
  workspace compiles. **The community handlers and `db/communities.rs` still query tables that no longer
  exist: they compile but cannot work at runtime.** That is intended — A3.1 deletes them. Do not "fix" them.
- The orchestrator removed the `relay_url` field from `api/node.rs` (a file you do not own).
- Green at A1 close: `cargo build --workspace` 0, `cargo build --workspace --all-targets` 0, `komun-core`
  20/20, clippy 11 warnings (frozen in `.dispatch/BACKEND-LINT-BASELINE.md`).

## Files you own in this card

`crates/server/src/auth/**` · `crates/server/src/rate_limit.rs` (new) · `crates/server/src/sessions.rs`
(new) · `crates/server/src/db/sessions.rs` (new) · `crates/server/src/db/users.rs` ·
`crates/server/src/repl.rs` · `crates/server/src/config.rs` · `crates/server/src/main.rs` ·
`crates/wasm/src/lib.rs` (only `compute_recovery_id`) · `crates/server/Cargo.toml` ·
`crates/wasm/Cargo.toml` · `crates/server/src/tests/**` · `.dispatch/hub-requests/**`

Do **not** touch: `web/**` (A2b), `api/**` beyond the `auth` mounting, `db/posts.rs`,
`db/communities.rs`, `api/communities.rs` (A3.1's casualties), `crates/core/**`, `migrations/**`.

## Task

1. **A2.1** Remove `jsonwebtoken`, `Claims`, `create_token`, `verify_token`, `[auth] jwt_secret`,
   `main.rs:48`'s env `set_var`, the env reads at `auth/mod.rs:670` and `:721`, the `CHALLENGES` map,
   `/auth/challenge`, `/auth/verify-challenge`, the `recover` endpoint, `recovery_id` /
   `recovery_code_hash` handling, and `compute_recovery_id`.
2. **A2.2** `auth/password.rs`: derive and store the verifier (server-side Argon2id on top of the client
   verifier), constant-time verify, minimum-length policy, rehash-if-params-change hook.
3. **A2.3** `POST /auth/signup` (email, password verifier, display name, wrapped bundles, encryption public
   key, invite code when the registration mode demands one); an `one_time_tokens` row; the verification
   email; `GET`/`POST /auth/verify`; resend with its own rate limit. Unverified accounts are restricted per
   SPEC Part 1.5 — read it, do not guess the rule.
4. **A2.4** `db/sessions.rs` + `sessions.rs`: create (raw token returned once, only the hash stored),
   verify-by-hash, list, revoke-one, revoke-all-others, and a cleanup task for expired rows. Middleware:
   bearer token → hash → session row → `AuthUser { user_id, role }` read from the DB. `require_admin` (admin
   OR superadmin) for admin routes, superadmin-only for role changes.
5. The `[email]` config block per SPEC Part 1.5 and the `[registration]` mode block (A7) — both in
   `config.rs`, which this card owns.
6. **Drive the six inherited lints in your own files to zero** (`.dispatch/BACKEND-LINT-BASELINE.md` rows 1-6:
   `auth/mod.rs:98`, `auth/mod.rs:610`, `config.rs:102`, `config.rs:136`, `repl.rs:20`, `repl.rs:125`).
   Two are `clippy::derivable_impls` — use `#[derive(Default)]` rather than a hand-written `impl`, which is
   the exact fix the orchestrator applied to `GeocodeConfig` on B's branch. The other five baseline rows are
   in files you do not own in this card; leave them and name them in your report.

## Gate — run it yourself and paste the raw output

```bash
export PATH=/usr/local/cargo/bin:$PATH; cd /workspace
grep -rn "jsonwebtoken\|JWT_SECRET\|recovery_id\|compute_recovery_id" crates/      # expect EMPTY
grep -rn "jwt_secret" crates/server/src/config.rs crates/server/src/main.rs         # expect EMPTY
cargo build --workspace --all-targets        # expect exit 0 (4 dead-code warnings are pre-existing)
cargo test --workspace                       # expect green
touch crates/server/src/main.rs              # cargo caches lint output; force it to re-emit
cargo clippy -p komun-server --no-deps --message-format short 2>&1 | grep "^crates/"
                                             # expect <= 5, all rows 7-11, all in files you do NOT own
cargo clippy --release -- -D warnings        # expected to STILL FAIL on rows 7-11 (A3/A5 own them) —
                                             # report exactly which remain; do not claim zero
psql "postgres://komun:komun@komun-db-a:5432/komun_a" -c "select count(*) from sessions"
```

Auth assertions to demonstrate (test or by hand against `komun_a`, paste the evidence): correct password
accepted; wrong password rejected; a loop test showing no measurable timing difference between the two;
too-short password rejected at signup; signup → restricted until verified; the verify token single-use and
expiring; duplicate email rejected in any case; startup fails loudly when `require_email_verification` is
true and SMTP is unset; a revoked session 401s on the next request; an expired session 401s; a logged-out
token cannot be reused; a demoted admin loses access immediately; `psql` shows `sessions` and
`one_time_tokens` rows being written.

**Honest limitation:** this container has no egress, so you cannot send a real email through SMTP. Prove the
verification mail by asserting on the composed message (recipient, subject, the token in the body) or on the
queued send, and say in your report that live SMTP delivery was not exercised here.

## STANDING RULES (apply to this card exactly as written)

1. Touch ONLY the files listed under "Files you own" in this card. If another file must change, write the
   exact diff (unified, with the real surrounding lines) into `/workspace/.dispatch/hub-requests/<ID>.md` and
   continue with everything else. Never edit that other file.
2. NEVER delete or move a file: `rm`/`mv` are denied in this sandbox and every deletion is performed by the
   orchestrator. List each path you want gone under "DELETIONS REQUESTED:" and keep working.
3. NEVER run git commit, push, checkout, restore, stash, rebase, reset, or worktree. The orchestrator owns git.
4. NEVER add a dependency, and never run cargo add / npm install / any fetch: this container has no internet.
   Request it in the hub-request file instead.
5. Write each file the moment it is ready. Do not batch writes at the end.
6. Run the gate commands in the card yourself, and paste their RAW output (unabridged) in your final message.
   If a gate fails, fix it and re-run. If you cannot fix it, say exactly what fails and paste the error text.
7. Never report success you did not observe. A file that exists but does not compile is a failure.
   A warning or lint claim must come from the command that actually produces it: `cargo build` never shows
   clippy lints, and a cached `cargo clippy` run prints nothing at all.
8. On any provider/billing error, stop and report — do not retry in a loop.
9. Frontend work: Svelte 5 runes only ($state, $derived, $effect, $props; no $:, no export let, no on:click).
10. End your final message with exactly these lines:
   FILES WRITTEN: <paths>
   GATES RUN: <command -> observed result>
   BLOCKED: <none, or the precise blocker>
   DELETIONS REQUESTED: <paths, or none>

## Environment facts

- `DATABASE_URL` is **not** exported. Your DB is the literal
  `postgres://komun:komun@komun-db-a:5432/komun_a`. Never touch `komun_b`.
- Run cargo with a plain `bash -c`, not `bash -lc`: a login shell drops `/usr/local/cargo/bin` from `PATH`.
- **Wasm:** the image has no `wasm32-unknown-unknown` target and no `wasm-bindgen`, so `wasm-pack build`
  **cannot run in this container**. Removing `compute_recovery_id` from `crates/wasm/src/lib.rs` is fine, but
  the built `crates/wasm/pkg/` then goes stale: say so in `BLOCKED:` and the orchestrator rebuilds it.
- No egress: every external host is unreachable, including SMTP.
