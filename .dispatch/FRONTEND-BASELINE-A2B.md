# Frontend baseline — `feature/agent-a`, measured AFTER A2b

Measured by the orchestrator on the real tree, in the container, with the toolchain and bindings the
orchestrator supplied:

```bash
docker exec -w /workspace/web agent-repo-agent-a bash -c \
  'export PATH=/usr/local/bin:/usr/bin:/bin; npm run check; npm run build; npx vitest run'
```

## Numbers (A2b closed)

| gate | result | vs frozen baseline |
|------|--------|--------------------|
| `npm run check` | **14 errors, 37 warnings in 8 files** | **unchanged — no new diagnostic** |
| `npm run build` | green, `Using @sveltejs/adapter-static`, `Wrote site to "build"` | unchanged |
| `npx vitest run` | **1 failed \| 46 passed (47)** | baseline was 1 failed \| 42 passed (43) → **+1 passing, same single failure** |

The single failure is `src/tests/AidCard.test.ts > AidCard > shows community name and server` — inherited,
and it belongs to **A6** (the community UI this reshape deletes). It is not a regression from any card.

The 14 errors are 12 in `web/src/tests/AidCard.test.ts` and 2 in
`web/src/routes/c/[slug]/p/[id]/+page.svelte`; all 37 warnings are community UI. Also A6's.

## Environment the orchestrator supplied (the card could not)

`web/node_modules/` and `crates/wasm/pkg/` did not exist in this worktree and this container has no egress,
so **every** frontend gate failed as `sh: 1: svelte-kit: not found`. Both were produced by the orchestrator
over a bridge-networked throwaway of the same image, in this order — the order matters, because
`web/package.json` depends on `file:../crates/wasm/pkg`:

1. `wasm-pack build crates/wasm --target web` → `Done in 16.41s`, `komun_wasm.js` **18,395 bytes**
   (was 21,708 before A2b removed ed25519 — the size drop is evidence the removal reached the bindings).
   Built with `CARGO_TARGET_DIR=/tmp/wasm-target` INSIDE the container, never a path in the worktree:
   a `target-wasm/` in the tree is not in `.gitignore` and turns the next `git add -A` into 611 staged
   artifact files.
2. `npm install` in `web/` → succeeded.

Both outputs are gitignored (`node_modules/`, `crates/wasm/pkg/`), so they never read as scope creep.

## Rules unchanged

A card's frontend criterion is **"no new diagnostic in a file it touched"**, never "green" — the gate is
inherited-red until A6 removes the community UI. Report any movement in either direction with the numbers.
A worker must never claim these baselines without running the commands above, and a failure that names a
missing binary (`svelte-kit: not found`) is an environment fact to report, not a code finding.
