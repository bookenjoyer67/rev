# STANDING RULES — pasted at the top of every Phase B dispatch prompt

You are working in /workspace, a git worktree of the Komun repo. Read /workspace/.dispatch/SPEC.md first.

1. Touch ONLY the files listed under "Files you own" in this card. If another file must change, write the
   exact diff (unified, with the real surrounding lines) into /workspace/.dispatch/hub-requests/<ID>.md and
   continue with everything else. Never edit that other file.
2. NEVER delete or move a file: `rm`/`mv` are denied in this sandbox ("Irreversible Local Destruction") and
   every deletion in this project is performed by the orchestrator. List each path you want gone in your
   final report under "DELETIONS REQUESTED:" and keep working; if a deletion truly blocks the card, put it
   in BLOCKED with the path list.
3. NEVER run git commit, push, checkout, restore, stash, rebase, reset, or worktree. The orchestrator owns git.
4. NEVER add a dependency, and never run cargo add / npm install / any fetch: this container has no internet.
   Request it in the hub-request file instead.
5. Write each file the moment it is ready. Do not batch writes at the end.
6. Run the gate commands in the card yourself, and paste their RAW output (unabridged) in your final message.
   If a gate fails, fix it and re-run. If you cannot fix it, say exactly what fails and paste the error text.
7. Never report success you did not observe. A file that exists but does not compile is a failure.
   A warning or lint claim must come from the command that actually produces it: `cargo build` never
   shows clippy lints, so "adds no new warnings" requires `cargo clippy -p <crate> --no-deps`, and that
   command must be forced to re-emit (`touch crates/server/src/main.rs` first) because cargo caches lint
   output: a cached second run prints nothing and reads as "zero warnings".
8. On any provider/billing error, stop and report — do not retry in a loop.
9. Frontend work: Svelte 5 runes only ($state, $derived, $effect, $props; no $:, no export let, no on:click).
10. End your final message with exactly these lines:
   FILES WRITTEN: <paths>
   GATES RUN: <command -> observed result>
   BLOCKED: <none, or the precise blocker>
   DELETIONS REQUESTED: <paths, or none>

## Phase B facts that change how you work (verified on the merged tree, 2026-09-23)

- **No migration is needed for Phase B, and `migrations/001_schema.sql` is FROZEN.** Its sha384 is
  bookmarked in `_sqlx_migrations`, so one changed byte makes every boot fail with a checksum mismatch.
  Everything Phase B needs is already in 001: `match_offers`, `deal_reviews`, `categories`, and the
  market columns on `posts` (`market_listed`, `price_cents`, `currency`, `price_negotiable`,
  `item_condition`, `sold_at`, `buyer_id`). If you believe you need a schema change, stop and file it as
  BLOCKED with the reason — do not write a migration.
- **All routes live under `/api`**, auth included (`/api/auth/signin`, `/api/auth/sessions`). A bare
  `/auth/...` or `/posts/...` path 404s and looks like a routing bug.
- **Point any DB client at the same database as the server.** The server takes `DATABASE_URL` over
  `[database] url`; a `psql` call that reads the config instead can silently query a different database and
  turn a real value into an "empty result" failure. Export the same `DATABASE_URL` for both.
- **The users column is `email_verified_at`** (not `verified_at`), and a user who is not verified can sign
  in but must be refused when posting, responding, messaging or listing.
- **Rebuild before you measure.** A binary built before your change still behaves like the old code; a
  migration you add would not be embedded either. `cargo build --workspace --all-targets` first, then test.
- **Frontend gates are absolute now**: `npm run check` must report 0 errors and 0 warnings (the inherited
  red is gone, not frozen), `npm run build` green, `npx vitest run` with 0 failures. `web/` currently
  passes 52 tests in 5 files; your additions must not reduce that, and new behaviour needs new assertions.
- **The map component is reusable.** `web/src/lib/components/LocationMap.svelte` supports an opt-in pick mode
  (`pickable` default false, `onpick` callback, controlled `pickedLat`/`pickedLon`, module-level
  `clampLatLon`) — use it for listing locations instead of writing another map.
