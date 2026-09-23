# CARD A2b — Frontend auth rewrite + ed25519 removal

**Owner:** Agent A (Claude Code/opus) · **Branch:** `feature/agent-a` · **Worktree:** `/home/computing/repo-agent-a` (container `agent-repo-agent-a`, cwd `/workspace`)
**Depends on:** A2a (DONE and independently verified by the orchestrator). **Blocks:** A7.3, B4, Phase B frontend.

## STANDING RULES (unchanged, restated because they bind this card)
1. No git mutations. No `cargo add` / `npm install` / network fetch. No docker, no sudo.
2. **You never delete or move files.** Write and edit only. A2.10 removes code; where a whole *file* must go, file `DELETION REQUESTS:` in your report and the orchestrator executes it. (Claude Code's `acceptEdits` denies `rm`/`mv`; that is expected, not a blocker.)
3. Writes confined to `/workspace`. Never touch Session B's DB (`komun_b`) or its worktree.
4. A claim of "no new warning" must come from the command that produces it. `cargo build` never emits clippy lints.
5. Report ends with `FILES WRITTEN: / GATES RUN: / BLOCKED: / DELETIONS REQUESTED:`.

## READ THIS FIRST — A2a already shipped most of this card's backend
Before writing any code, inventory what exists. `crates/server/src/auth/mod.rs` already mounts, verified live by the orchestrator against a real server on this branch: `/auth/signup`, `/auth/signin`, `/auth/salt`, `/auth/verify`, `/auth/resend-verification`, `/auth/password-reset` (+`/confirm`), `/auth/me` (GET/PUT), `/auth/me/avatar`, `/auth/signout`, `/auth/sessions` (GET/DELETE), `/auth/sessions/{id}` (DELETE), `/auth/users/{id}/keys`.

Live auth behaviour the orchestrator confirmed on the real tree (do not re-derive, build on it): signup 200 + 43-char token; wrong verifier 401 `incorrect email or password`, byte-identical for an unknown account; `/me` 200; signout then token reuse 401 `session is invalid or has expired`; salt lookup 200; limiter armed 401×7 → 429 `retry_after_seconds`; Postgres stores `$argon2id$v=19$m=19456,t=2,p=1` and a 32-byte salt.

So the plan's A2.5/A2.6/A2.7/A2.9 are largely **already implemented**. Your job is the gap, not a re-write. Establish the gap by grepping for the route, and say in your report which of these you found already present.

## Task
- **A2b.1 — Backend gap fill (only what the grep shows missing).** Likely: `POST /auth/password/change` (requires the current verifier, re-wraps the x25519 secret, revokes *other* sessions), `POST /auth/recovery/reissue` (fresh 12-word code, re-wraps the recovery copy, invalidates the previous code), and the admin session/role endpoints under `/admin/users/{id}/sessions` + role promotion/demotion (superadmin only, `audit_events` row on change). If a route already exists, do not touch it.
- **A2b.2 — A2.10: remove ed25519, in the order given in Part 1.5.** Collapse `encrypt_key_bundle` to a single-secret wrap and fix the `bytes.slice(0, 32)` split in `recoverFromBundle`.
- **A2b.3 — A2.11: the frontend rewrite** — signup form, login form, verify banner, forgot/reset pages, change-password, sessions screen, recovery-code display shown once at signup with a download option, and automatic in-memory key unlock after login. **No passphrase prompt anywhere.**
- **A2b.4 — hub item 5:** `GET /auth/users/{id}/keys` no longer returns `public_key`; the frontend consumer must absorb that.
- **A2b.5 — wasm/pkg ordering (IMPORTANT).** `crates/wasm/pkg/` is a build artifact and **you cannot rebuild it** (no `wasm32-unknown-unknown`, no `wasm-bindgen`, no network). Land your `crates/wasm/src/lib.rs` edits **first**, then immediately file `/workspace/.dispatch/hub-requests/A2b-wasm.md` saying exactly which exported functions changed, and carry on with the frontend. The orchestrator regenerates `pkg/` and re-runs the web gates; any web test that imports from `pkg/` before that rebuild is expected to fail on the stale artifact, so **do not report such a failure as a defect** — name it as waiting on the rebuild.

## Files you own
`crates/server/src/auth/**`, `crates/server/src/rate_limit.rs`, `crates/server/src/api/admin.rs`, `crates/server/src/api/mod.rs` (auth route mounting only), `crates/wasm/src/lib.rs`, `web/src/lib/crypto.ts`, `web/src/lib/stores/auth.ts`, `web/src/routes/account/**`, `web/src/tests/**`, `crates/*/Cargo.toml`.
Read-only: everything else. `web/src/routes/c/**`, `web/src/lib/components/**` community UI belongs to A6 — leave it broken-but-unchanged.

## Gate (corrected — the plan's version cannot run as written)
```bash
docker exec -w /workspace agent-repo-agent-a bash -c 'export PATH=/usr/local/cargo/bin:$PATH; cd /workspace
grep -rn "ed25519" crates/ web/src --include="*.rs" --include="*.toml" --include="*.ts" ; echo "^^ comments only"
grep -rn "passphrase" web/src | grep -v test ; echo "^^ expect empty"
cargo build --workspace --all-targets        # must be exit 0 — it is now, after the orchestrator applied hub item 2
cargo test --workspace                       # baseline on this branch: core 20 passed, server 44 passed, 0 failed
touch crates/server/src/main.rs; cargo clippy -p komun-server --no-deps --message-format short | grep "^crates/"'
```
- **Do NOT run `wasm-pack build`** — it cannot work here (reason in A2b.5). The plan's line for it is deleted.
- **`cargo clippy --release -- -D warnings` cannot be green yet** and is not your gate. The baseline is **5 rows** (see `BACKEND-LINT-BASELINE.md`): `db/alliances/mod.rs:47`, `db/endorsements.rs:59`, `federation/mod.rs:10`, `api/endorsements.rs:95`, `db/communities.rs:192`. A3 owns them; two die with B's federation/alliance deletions at merge. **Your criterion: still 5, no sixth.** Name any new row and the file it is in.
- **Frontend gate:** `npm run check` on this branch is a **frozen inherited baseline — 14 errors and 37 warnings in 8 files** (12 errors `web/src/tests/AidCard.test.ts`, 2 `web/src/routes/c/[slug]/p/[id]/+page.svelte`, all 37 warnings community UI; every one of them is A6's, not yours). Report `npm run build` (must still succeed) and `npx vitest run` (baseline 1 failed | 42 passed). Your criterion: **no new diagnostic in a file you touched**, and report any movement either direction.
- **Live assertions (run against `komun_a`, on port 3011 so you don't collide with anything):** 5 rapid logins from one IP → 429; a spoofed `X-Forwarded-For` from an untrusted peer ignored; forgot for an unknown email → 200 and sends nothing; a used/expired reset token rejected; reset **with** the recovery code makes a pre-reset message readable and **without** it does not; every session gone after a password reset; change-password keeps the same x25519 secret; reissuing a recovery code invalidates the previous one; the recovery code is never returned by any endpoint after signup.
- **You may not weaken a test to pass it.** If an assertion fails, that is a finding — report it with raw output.

## Exit criteria
Every assertion above observed; `web/build/` produced; no new clippy row; no new frontend diagnostic beyond the frozen baseline; the wasm hub file filed with the exact export changes.

## Orchestrator hand-off notes
- `komun_a` already has its migration bookmarked (`_sqlx_migrations` row 1, checksum verified against `migrations/001_schema.sql` by the orchestrator) and `config.toml` now carries `[registration] require_email_verification = false` with the dead `[auth] jwt_secret`, `[relay]` and `[federation]` sections removed. The server boots: `[email] is not configured` is a warning, not a refusal.
- Carry-overs routed elsewhere, do NOT fix them here: `api/users.rs` serialising dropped columns and `api/directory.rs:27,35` reading `discovery.registration_mode` are both in **A3's** write set and are recorded for that card.
