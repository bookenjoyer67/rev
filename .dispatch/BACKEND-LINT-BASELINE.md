# Backend lint baseline — updated by the orchestrator after A2a landed

`cargo clippy -p komun-server --no-deps` reports **exactly 0 warnings on the merged tree** (`feature/agent-b`
after merging `feature/agent-a`) — verified both ways: the row count is 0, and the project's standard gate
`cargo clippy --release -- -D warnings` now finishes clean for the first time. The table below is the historical
arithmetic that got there; do not treat it as current.
A2a drove its own six entries to zero (verified: `count=5`, from the command that emits them).

**Rules.** No card may raise this number. A card that owns one of these files drives its entries to 0.
The user's standard is `cargo clippy --release -- -D warnings` with zero warnings; the **end-of-Phase-A
gate is the point at which all of them are gone**, and it cannot pass while any survives.

| # | location | lint | owner / fate |
|---|----------|------|--------------|
| 1 | `db/endorsements.rs:59` | function `count_for_user` is never used | A3 — dead code, same root as a `cargo build` warning |
| 2 | `api/endorsements.rs:95` | this `map_or` can be simplified | A3 |
| 3 | `db/communities.rs:192` | too many arguments (8/7) | A3.1 deletes the file |
| 4 | `db/alliances/mod.rs:47` | function `get_alliance` is never used | deleted with `db/alliances/**` on B's branch (B3); still present on A's until merge |
| 5 | `federation/mod.rs:10` | function `handshake_with_peer` is never used | deleted with `federation/**` on B's branch (B3); still present on A's until merge |

## Closed by A2a (was rows 1-6 of the original 11)
`auth/mod.rs:98`, `auth/mod.rs:610`, `config.rs:102`, `config.rs:136`, `repl.rs:20`, `repl.rs:125` — all zero
on this branch. Two of them (`config.rs`) were the same `derivable_impls` pattern the orchestrator had already
fixed once on B's branch, so the fix was known-good before A2a started.

## Counts by branch (measured, not inferred)
- `feature/agent-a` after A2a: **5** (the table above).
- `feature/agent-b` after B3: **9** — B never had A1.4/A2a, and B3's deletions removed the two it could.
- **After both merge: expect 3** (rows 1-3). Those three are the remaining work before `--release -- -D warnings`
  can be the gate.

The 4 pre-existing `cargo build` warnings (dead `UserRow`, `count_for_user`) are the same dead code as the lint
rows; they die with the deletions and A3's rewrite, not by adding `#[allow]`.

## How to measure it (cargo caches lint output — do not trust an empty run)

```bash
export PATH=/usr/local/cargo/bin:$PATH; cd /workspace
touch crates/server/src/main.rs          # force the lint to re-emit; a cached run prints nothing
cargo clippy -p komun-server --no-deps --message-format short 2>&1 | grep "^crates/"
```

A run that prints no diagnostics at all is a cache hit, not a clean lint. Any claim about warnings must come
from the command that produces that class of diagnostic — `cargo build` cannot prove a clippy claim.
