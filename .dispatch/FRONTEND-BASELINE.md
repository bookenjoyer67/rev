# Frontend baseline — frozen 2026-09-23 (orchestrator-measured, branch `feature/agent-b`)

**Why this file exists.** `npm run check` and `npx vitest run` are RED on the *untouched* frontend: they
were never green before either agent touched a line. The entire red set is the old community model that
**A6 deletes and rewrites** — it is not debt B introduced, and it is not debt B must pay.

**Therefore every web gate in this project is measured against this frozen list:** a card may not add a
diagnostic and may not add a failing test. Compare the COUNTS, not just the exit code.

## `npm run check` → exit 1, 14 errors + 37 warnings in 8 files

errors (14):

```
 12  web/src/tests/AidCard.test.ts                    (PostLike fixture carries community_id,
                                                      community_name, community_slug, server_* fields)
  2  web/src/routes/c/[slug]/p/[id]/+page.svelte
```

warnings (37):

```
 28  web/src/routes/c/[slug]/+page.svelte
  4  web/src/lib/components/AidCard.svelte
  3  web/src/routes/users/[id]/+page.svelte
  1  web/src/routes/search/+page.svelte
  1  web/src/lib/components/SearchBar.svelte
```

## `npm run build` → GREEN

`@sveltejs/adapter-static`, writes `build/`. Measured green before B2 and must stay green after it.

## `npx vitest run` → 1 failed | 39 passed (40)

The one failure, by name:

```
src/tests/AidCard.test.ts > AidCard > shows community name and server
```

## Rules for every web card

1. Do not increase any count above. If your change makes a number go up, fix your change.
2. Do NOT "fix" the baseline entries unless your card names them: every one of them disappears when A6
   deletes `web/src/routes/c/**` and rewrites `AidCard` / `SearchBar` / the tests fixture. A6's exit
   criterion is 0 errors, which is reachable only there.
3. The toolchain is pre-warmed **by the orchestrator**, not by you: `crates/wasm/pkg/` exists (gitignored,
   built with wasm-pack) and `web/node_modules/` is installed including `leaflet` 1.9.4. Do not run
   `npm install`: there is no egress, and the image has no `wasm32-unknown-unknown` target or
   `wasm-bindgen`, so `wasm-pack build` cannot run inside your container. If you need `pkg/` rebuilt
   because you changed `crates/wasm/src/lib.rs`, say so in `BLOCKED:` — it is an orchestrator step.
4. `web/package.json` and `web/package-lock.json` are orchestrator-only, even when you need a dependency.
