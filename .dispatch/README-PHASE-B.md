# Phase B dispatch index — the marketplace

Phase B continues the same two-agent split as Phase A. Card IDs are `M*` (Agent A, the spine) and
`F*`/`D*` (Agent B, new files and docs) so they never collide with the Phase A `A*`/`B*` cards.

| Card | Owner | Depends on | What it delivers |
|------|-------|-----------|------------------|
| M1 | Agent A | Phase A merged | `[market] default_currency`, the categories API + admin edits, market filters on the posts list |
| M2 | Agent A | M1 | offers on the match thread, the accept that records the agreed price, completion that marks the post sold |
| M3 | Agent A | M2 | star ratings + written reviews, writable only against a completed deal, and the profile aggregate |
| F1 | Agent B | M1 merged into its branch | `/market` browse + `/market/new`, `MarketCard`, `market.ts`, `categories.ts` |
| F2 | Agent B | M2 + M3 merged into its branch | `OfferPanel`, `DealReviewModal`, the deal controls in `/messages/**` |
| D1 | Agent B | M1, M2, M3, F1, F2 | docs + `config.example.toml` for the marketplace; carries Phase B's definition of done |

**Dispatch order:** M1 → merge into B's branch → F1 can start. M2 → M3 → merge both into B's branch → F2.
D1 last. Both agents prepend `.dispatch/STANDING-RULES.md` to the card text when it is dispatched, exactly
as in Phase A.

**Everything M1–M3 need is already in the frozen schema** — `match_offers`, `deal_reviews`, `categories`,
and the market columns on `posts`. Phase B needs NO migration, and `migrations/001_schema.sql` must not be
edited (its sha384 is bookmarked in `_sqlx_migrations`).

Facts carried forward from Phase A that the cards assume:
- Every route lives under `/api`, auth included.
- The frontend gates are absolute: `npm run check` 0 errors / 0 warnings, `npm run build` green,
  `npx vitest run` 0 failures (52 tests in 5 files today).
- Backend gates: `cargo build --workspace --all-targets` clean, `cargo test --workspace` core 20 + server 58
  with 0 failures, `cargo clippy -p komun-server --no-deps` 0 rows (touch `main.rs` first, or cargo's cache
  prints nothing), `cargo clippy --release -- -D warnings` Finished.
- A binary built before a change still behaves like the old code: rebuild before measuring.

**The sandbox was torn down after Phase A** (worktrees, containers and both databases removed; the image
`agent-sandbox:komun` was kept). To resume, recreate the two worktrees and the two agent containers plus
the broker — the exact recipe (mounts, env, network, the claude/opencode invocations) is in `~/rev/setup.md`
§5, which is deliberately untracked, and the database provisioning order is scripted in
`~/.hermes/profiles/dev/cache/scratch/` (`db-provision-b.sh` is the reference implementation: create the
database, create `_sqlx_migrations` FIRST, load `001_schema.sql`, bookmark version 1 with the file's real
`sha384sum`, then boot so the migrator applies 002 itself).
