# Backend lint baseline — frozen 2026-09-23 (orchestrator-measured, identical on both branches)

`cargo clippy -p komun-server --no-deps` reports **exactly 11 warnings** at the close of card A1/A1.5. All 11
predate this project: none was introduced by the reshape, and the one that lives in a file A1.5 touched
(`db/communities.rs:192`) is byte-identical to `HEAD` (verified with `git show`), so it is inherited, not new.

**Rules.** No card may raise this number. A card that owns one of these files must drive its entries to 0.
The user's standard is `cargo clippy --release -- -D warnings` with zero warnings, so the **final Phase A
gate is the point at which all 11 are gone** — that gate cannot pass while any survives.

| # | location | lint | fate |
|---|----------|------|-----|
| 1 | `auth/mod.rs:98` | very complex type used | A2a — rewrites `auth/**` |
| 2 | `auth/mod.rs:610` | struct `UserRow` is never constructed | A2a — rewrites `auth/**` |
| 3 | `config.rs:102` | this `impl` can be derived | A2a — owns `config.rs` (same fix pattern the orchestrator already applied to `GeocodeConfig` on B's branch: use `#[derive(Default)]`) |
| 4 | `config.rs:136` | this `impl` can be derived | A2a |
| 5 | `repl.rs:20` | `str::trim` before `str::split_whitespace` | A2a — `repl.rs` is A's file (Part 5.1) |
| 6 | `repl.rs:125` | literal with an empty format string | A2a |
| 7 | `db/communities.rs:192` | too many arguments (8/7) | vanishes when A3.1 deletes the file |
| 8 | `db/endorsements.rs:59` | function `count_for_user` is never used | A3 |
| 9 | `api/endorsements.rs:95` | this `map_or` can be simplified | A3 |
| 10 | `db/alliances/mod.rs:47` | function `get_alliance` is never used | vanishes when the module is deleted (A5.1/B3) |
| 11 | `federation/mod.rs:10` | function `handshake_with_peer` is never used | vanishes when the module is deleted (A5.1/B3) |

Six of the eleven are A2a's; the other five are owned by cards that delete or rewrite their files. The
expected count at the close of A2a is therefore **≤ 5, all of them in files A2a does not own, named
explicitly in the report**.

## How to measure it (cargo caches lint output — do not trust an empty run)

```bash
export PATH=/usr/local/cargo/bin:$PATH; cd /workspace
touch crates/server/src/main.rs          # force the lint to re-emit; a cached run prints nothing
cargo clippy -p komun-server --no-deps --message-format short 2>&1 | grep -c "^crates/"
cargo clippy -p komun-server --no-deps --message-format short 2>&1 | grep "^crates/"
```

A run that prints no diagnostics at all is a cache hit, not a clean lint. Both branches report the same 11
today; the number is the contract.
