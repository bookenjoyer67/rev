# CARD D1b — finish D1: the README market depth, and one currency-precedence correction

Owner: Agent B (docs). Worktree: ~/repo-agent-b. Branch: feature/agent-b. Blocks: the Phase B exit.

D1 was cut short by the orchestrator's own time limit AFTER writing nine files — README.md, AGENTS.md,
config.example.toml, deploy/seed.sql and docs/{ARCHITECTURE,CONVENTIONS,DATABASE,DEPLOY,DEVELOPMENT}.md.
Those edits are in your working tree, they are good, and **you must not revert or rewrite them**. This card
closes the two remaining gaps only.

## ENVIRONMENT — do not rediscover any of this (the previous run wasted its session hunting for Postgres)

- `psql` is `/usr/bin/psql`. The database host is **`komun-db-b` on port 5432**, user `komun`, password
  `komun`; your database is `komun_b`.
- **Never** run `docker`, `pg_ctl`, `initdb`, `postgres`, `apt`, and never read `/etc/*` or
  `/var/lib/postgresql/*`. Those paths are outside your sandbox and every attempt is auto-rejected. The
  server is already running and is not yours to start.
- Scratch-database recipe, if you need one:
  ```bash
  psql postgres://komun:komun@komun-db-b:5432/postgres -c 'CREATE DATABASE komun_d1b'
  psql postgres://komun:komun@komun-db-b:5432/komun_d1b -f migrations/001_schema.sql
  psql postgres://komun:komun@komun-db-b:5432/komun_d1b -f deploy/seed.sql
  psql postgres://komun:komun@komun-db-b:5432/komun_d1b -tAc 'select scope, count(*) from categories group by scope'
  psql postgres://komun:komun@komun-db-b:5432/postgres -c 'DROP DATABASE IF EXISTS komun_d1b WITH (FORCE)'
  ```

## Task — two items, nothing else

1. **README.md needs market depth, not one bullet.** Read the marketplace sections you already wrote in
   `docs/ARCHITECTURE.md` and the routes in `crates/server/src/api/` before writing, and add only what is
   accurate: the review rule (writable only against a `completed` deal, one per participant per deal, and
   the rating shown on the profile as an average of one decimal), the category taxonomy being the seeded
   and runtime-editable `categories` table served by `GET /api/categories?scope=…` rather than an enum, and
   the posts-list pagination (`limit` defaults to 100 and caps at 200). Keep the existing bullets and their
   voice; this is an extension of README, not a rewrite.

2. **config.example.toml — correct the `[market]` paragraph.** Its current wording ("the post's own
   currency always wins … Precedence, highest first: the post's currency, then this value, then that 400")
   is incomplete and misleading about offers. What is verified at runtime:
   - a **post** (`listing`/`want`) with no `currency` of its own takes this value; with neither, the post
     carries none, and an offer or accept that names an amount is then refused with a 400 naming the
     missing currency;
   - an **offer** that names its own currency uses it — an offer may name a currency different from the
     post's, and that currency is what the deal records when it is accepted (observed: a USD offer on a CAD
     listing was accepted and `matches.currency` became USD). An `accept` must restate `amount_cents`.
   Rewrite that paragraph to state both, and keep the two sentences that are already right: unset by
   default and deliberately commented out, and a malformed value being a startup failure that names the
   key, the value and the remedy.

## Gate (paste raw output)

```bash
cd /workspace/web && npm run check && npx vitest run   # 0 errors / 0 warnings; 82 tests passing
grep -n -A12 '^\[market\]' /workspace/config.example.toml   # the corrected paragraph
grep -n -i 'pagination\|limit' /workspace/README.md | head   # the cap is stated
```

Files you own: `README.md`, `AGENTS.md`, `config.example.toml`. Nothing under `crates/` or `web/`.

Report with the standing FILES/GATES/BLOCKED/DELETIONS tail.
