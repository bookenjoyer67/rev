# CARD B4a — Detached docs draft (CONVENTIONS + DEVELOPMENT)

**Owner:** Agent B (opencode/deepseek-v4-flash) · **Branch:** `feature/agent-b` · **Worktree:**
`/home/computing/repo-agent-b` mounted as `/workspace` in container `agent-repo-agent-b` ·
**Depends on:** nothing — this is the one B card with zero dependency on Phase A. ·
**Note:** B4 will **re-check** these two files rather than rewrite them; that is the point of doing them now.

## Why this card exists and why it is only these two files

B4's own card says writing docs early "guarantees drift", because it documents A's implementation details.
These two files are the exception: they describe **conventions** and **build/run procedure**, which the SPEC
freezes up front and A's cards do not change. Do not write any other doc in this card — not `README.md`, not
`AGENTS.md`, not `ARCHITECTURE.md`, not `CRYPTO.md`, not `DATABASE.md`.

## Files you own in this card

- `docs/CONVENTIONS.md` (new)
- `docs/DEVELOPMENT.md` (new)
- `.dispatch/hub-requests/**`

No code. If you find a convention that the code violates, **do not fix the code** — write it in the doc as
the rule and list the violation in your report under `BLOCKED:`.

## Task

**1. `docs/CONVENTIONS.md`** — every rule must be one you read out of `.dispatch/SPEC.md` (Parts 1 and 2) or
verified in the tree, never one you invent. Cover at least:

- the database-enum policy: one `db_enum!` macro so serde, `as_str()` and `parse()` cannot drift; no
  `serde_json` round-trips to build enum strings; `crates/core/src/tests.rs` reads
  `migrations/001_schema.sql` at test time and pins every enum to its `CHECK (col IN (...))` list;
- the single-migration rule: `migrations/001_schema.sql` is the schema; a new migration is a new file, and
  any `CHECK` list you add without an enum behind it fails the test by design;
- frontend: Svelte 5 runes only (`$state`, `$derived`, `$effect`, `$props`; no `$:`, no `export let`, no
  `on:click`), and the API client is the frontend's single hub file;
- crypto boundaries: no plaintext message body anywhere in the schema; the server never returns key material
  or a recovery code;
- process conventions that are already true in this repo: write in place rather than deferring writes, no new
  dependency without a hub request, and comments explain *why* a thing is the way it is.
  For each rule, name the file that demonstrates it, so a reader can check the claim.

**2. `docs/DEVELOPMENT.md`** — how to build, run and test, in the order that actually works:

- prerequisites (Rust toolchain, `wasm-pack`, Node 22 / npm 10, Postgres 16) and the schema: a fresh DB is
  made by applying `migrations/001_schema.sql` once;
- the build order, flagged for the one trap that bites everyone: `web/package.json` depends on
  `komun-wasm: file:../crates/wasm/pkg`, so **the wasm package must be built before `npm install`**;
- commands: `cargo build --workspace`, `cargo test --workspace`, `cargo clippy --release -- -D warnings`
  (the zero-warning standard), `wasm-pack build crates/wasm --target web`, `npm run check`, `npm run build`,
  `npx vitest run`, and how to run the server;
- a short "measure a lint/tool gate honestly" note, because it has bitten this session twice: a cached
  `cargo clippy` run prints nothing (touch a source file first), `cargo build` never surfaces clippy lints,
  and `svelte-check` is expected to report the frozen inherited baseline until the frontend flatten lands;
- an explicit "not verifiable in a sandbox" section: no network means no tile loading and no live SMTP
  delivery; the wasm build needs network on first use, so `crates/wasm/pkg/` is normally pre-built by whoever
  has egress.

**Execute every command you document at least once**, and mark clearly any command you could not execute here
(`wasm-pack build` cannot run in this container — the image has no `wasm32-unknown-unknown` target and no
`wasm-bindgen`). A doc with an unrun command must say so; that honesty is the same rule as the code cards.

## Gate

```bash
export PATH=/usr/local/bin:/usr/bin:/bin; cd /workspace/web
npm run check     # unchanged: 14 errors / 37 warnings (this card touches no code — if the number moves, revert)
npx vitest run    # unchanged: 1 failed | 42 passed
```

Plus: every command you documented and could run is in your report with its observed output; every doc claim
names the file it was verified against.

## STANDING RULES (apply to this card exactly as written)

1. Touch ONLY the files listed under "Files you own" in this card. If another file must change, write the
   exact diff into `/workspace/.dispatch/hub-requests/<ID>.md` and continue with everything else. Never edit
   that other file.
2. NEVER delete or move a file: `rm`/`mv` are denied in this sandbox and every deletion is performed by the
   orchestrator. List each path you want gone under "DELETIONS REQUESTED:" and keep working.
3. NEVER run git commit, push, checkout, restore, stash, rebase, reset, or worktree. The orchestrator owns git.
4. NEVER add a dependency, and never run cargo add / npm install / any fetch: this container has no internet.
   Request it in the hub-request file instead.
5. Write each file the moment it is ready. Do not batch writes at the end.
6. Run the gate commands in the card yourself, and paste their RAW output (unabridged) in your final message.
   If a gate fails, fix it and re-run. If you cannot fix it, say exactly what fails and paste the error text.
7. Never report success you did not observe. A warning or lint claim must come from the command that actually
   produces it.
8. On any provider/billing error, stop and report — do not retry in a loop.
9. Frontend work: Svelte 5 runes only ($state, $derived, $effect, $props; no $:, no export let, no on:click).
10. End your final message with exactly these lines:
   FILES WRITTEN: <paths>
   GATES RUN: <command -> observed result>
   BLOCKED: <none, or the precise blocker>
   DELETIONS REQUESTED: <paths, or none>

## Environment facts

- No egress. `wasm-pack build` cannot run here; `npm run check` / `npm run build` / `npx vitest run` work
  because `crates/wasm/pkg/` and `web/node_modules/` were pre-built by the orchestrator — say so in
  DEVELOPMENT.md rather than implying a reader can just run `npm install` offline.
- Your DB is `komun_b` on `komun-db-b`; `DATABASE_URL` is not exported. Never touch `komun_a`.
- Use a plain `bash -c`, not `bash -lc`, or `/usr/local/cargo/bin` disappears from `PATH`.
