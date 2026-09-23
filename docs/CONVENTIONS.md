# Conventions

Frozen conventions for the reshaped Komun. Every rule below is taken from
`.dispatch/SPEC.md` Parts 1–2 or verified in the tree, and each names the file or SPEC
section that demonstrates it so a reader can check the claim. (`docs/DEVELOPMENT.md`
covers how to build, run and test.)

Where a rule's implementing file is an Agent A deliverable that has not landed on this
branch yet, the rule is still the contract; the note says so explicitly, and the current
violation is reported rather than silently worked around.

---

## 1. Database enums: one `db_enum!` macro, one source of truth

**Rule.** Every Rust enum that maps to a DB text column is defined **once**, through the
`db_enum!` macro in `crates/core`. The macro emits serde `Serialize`/`Deserialize`,
`as_str()`, and `parse()` from a single variant list, so the three representations cannot
drift apart. Do **not** build an enum's database string with a `serde_json` round-trip
(e.g. `serde_json::to_string(&kind)?.trim_matches('"')`).

**Why.** The pre-reshape code maintained CHECK lists and Rust enums separately with
nothing checking agreement; that produced the three live constraint bugs recorded in
`.dispatch/SPEC.md` §2.2 (P1 `visibility`, P2 `matches.status`, P3 `posts.status`) and
the forbidden round-trip in P4.

**Enforcement.** `crates/core/src/tests.rs` reads `migrations/001_schema.sql` at test
time and asserts that each enum's `as_str()` value set equals the `CHECK (col IN (...))`
list parsed out of the migration, for `PostKind`, `Urgency`, `PostStatus`, `Visibility`,
`UserRole`, and Phase B's `ItemCondition` / `OfferKind`. `Category` is no longer an enum:
it is the seeded `categories` table (SPEC §1.6), so it gets a seed-data test instead.
Authority: `.dispatch/SPEC.md` A1.3 (lines 730–735) and §2.2.

**Demonstrated by.** `migrations/001_schema.sql` (the CHECK lists) + the `db_enum!`
definition and `crates/core/src/tests.rs` — the latter two are A1.3 deliverables, not yet
present on this branch. Current violation on this branch: `crates/server/src/db/posts.rs`
still builds `kind`/`category`/`urgency` with `serde_json` (SPEC P4); see `BLOCKED`.

---

## 2. Migrations: one schema file, additive after it

**Rule.** `migrations/001_schema.sql` **is** the schema (SPEC §1.4). A schema change is a
new numbered file (`002_*.sql`, `003_*.sql`, …) — migrations are additive, and an applied
migration is never edited (`AGENTS.md`: "Never edit existing migrations; add new ones").
A `CHECK (col IN (...))` list added without a matching enum behind it is rejected **by
design**: it fails the §1 enum-pinning test.

**Fresh database.** A new database is created by applying `migrations/001_schema.sql`
once (SQLx also runs migrations automatically at server startup).

**Demonstrated by.** `migrations/001_schema.sql`; `.dispatch/SPEC.md` §1.4 (target schema),
A6 (squash), and Part 8 (Phase B would add `002_*.sql`). On this branch the squash is still
an A1.1 deliverable — `migrations/` still contains `001`–`015`.

---

## 3. Frontend: Svelte 5 runes only, one API hub

**Runes only.** Use `$state`, `$derived`, `$effect`, `$props`. Never `$:`, `export let`,
or `on:click`; event attributes are `onclick={handler}`. Authority: `.dispatch/SPEC.md`
§0.4 rule 9, §6 wave A6, and the tech-stack line ("SvelteKit 5 (runes, static adapter)").
The tree already complies — e.g. `web/src/lib/components/AidCard.svelte` and
`web/src/lib/components/LocationMap.svelte` (B2) use the runes API and `onclick=` only.

**SPA mode.** The app is client-rendered: `web/src/routes/+layout.ts` sets
`ssr = false` and `prerender = false`; `@sveltejs/adapter-static` emits an SPA fallback.

**One API hub.** `web/src/lib/api/**` is the frontend's single hub file
(`.dispatch/SPEC.md` §5.1, "the API client — the frontend's hub file"). Every server call
belongs there; components, routes and stores should not call `fetch('/api/...')`
directly. The typed `request<T>` wrapper in `web/src/lib/api/client.ts` is the shape to
extend. Current violation on this branch: many routes/stores still call `fetch()`
directly (e.g. `web/src/lib/stores/auth.ts`, `web/src/routes/account/+page.svelte`); the
A6.2 flatten and the A2b auth rewrite are what collapse them onto the hub; see `BLOCKED`.

**Styling/tokens.** No CSS framework. Design tokens are CSS custom properties in
`web/src/app.css`; components use scoped `<style>` blocks that reference those tokens
(e.g. `web/src/lib/components/LocationMap.svelte`).

---

## 4. Crypto boundaries (hard constraint)

**No plaintext message body in the schema.** Message content is encrypted client-side;
the `messages` table stores `ciphertext BYTEA NOT NULL` plus `nonce`, never `body TEXT`.
Authority: `.dispatch/SPEC.md` §1.3 ("message content is never readable by the server"),
§1.4 (`messages`), A1.2. Expected demonstrator: `migrations/001_schema.sql`'s `messages`
table. Current violation on this branch: `migrations/001_schema.sql` still has
`messages.body TEXT NOT NULL` (line 78); see `BLOCKED`.

**Keys never leave the client; the server returns no key material or recovery code.**
Only public keys and wrapped key bundles are server-visible; the x25519 secret and the
passphrase are not. No endpoint ever returns key material or a recovery code after
signup. Authority: `.dispatch/SPEC.md` §1.5 and the A2 definition of done (line 1149);
`AGENTS.md` "Do not log keys, bundles, or passphrases anywhere." The reshaped column set
is in SPEC §1.4 (`users`: `encryption_public_key`, `encrypted_key_bundle`,
`bundle_salt`, `encrypted_recovery_bundle`, `recovery_bundle_salt`; `public_key` /
`recovery_id` / `recovery_code_hash` are gone).

**Passphrase-free UX.** Users never see, type or manage key material (SPEC decision A12);
there is no passphrase prompt and no proof-of-possession login.

---

## 5. Process conventions

These are the working rules that produced the current tree; they are not optional.

- **Write in place.** The moment a file is ready, write it — do not batch writes to the
  end. Authority: `.dispatch/SPEC.md` §0.4 rule 5.
- **One writer per file; changes to a file you do not own go through a hub request.** A
  file has exactly one owner (`.dispatch/SPEC.md` §5). If a non-owner needs a change, it
  writes the exact unified diff to `.dispatch/hub-requests/<ID>.md` and keeps going; it
  never edits the file. Authority: SPEC §0.2, §5.3.
- **No new dependency without a hub request.** The agent containers have no egress, so
  `cargo add` / `npm install` cannot work. `Cargo.toml`, `crates/*/Cargo.toml`,
  `web/package.json` and `web/package-lock.json` are orchestrator-only. Authority: SPEC
  §0.2 rule 2, §0.4 rule 4, §5.3.
- **Comments explain *why*, not *what*.** Doc- and line-comments state the reason a
  design is the way it is (invariant, constraint, failure mode), rather than restating the
  code. Demonstrated by `crates/server/src/api/geocode/limiter.rs:8-14` and
  `crates/server/src/api/geocode/cache.rs:15-18`, which explain the queueing/eviction
  rationale, not the syntax.
- **Measure gates honestly.** A warning/lint claim must come from the command that
  actually produces it: `cargo build` never shows clippy lints, and a cached `cargo
  clippy` run prints nothing — touch a source file first. `svelte-check` is expected to
  report the frozen inherited baseline until the frontend flatten lands. Authority:
  `.dispatch/SPEC.md` §0.4 rule 7, `.dispatch/BACKEND-LINT-BASELINE.md`,
  `.dispatch/FRONTEND-BASELINE.md`.
- **Never edit an applied migration.** See §2.

---

## Re-check

`docs/CONVENTIONS.md` and `docs/DEVELOPMENT.md` are the two docs written before the
implementation lands; card B4 re-checks them against the shipped code rather than
rewriting them. Anything in this file that names an "A1/A2/A6 deliverable" is the point to
re-verify at B4.
