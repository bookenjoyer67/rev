# Komun — Two-Agent Dispatch Plan (Phase A reshape + auth, Phase B marketplace)

> **Supersedes the planning structure of `2026-09-22_1620_Komun reshape auth marketplace`.** Same
> locked decisions, same target schema, same verified findings — rewritten as a dispatch document:
> every wave is now a work order with one owner, an exact file list, a copy-pasteable command, a
> prompt body, and a gate the orchestrator re-runs before anything is committed.
>
> **Status:** dispatch-ready. No wave has been executed. The sandbox facts in Part 3 were measured on
> 2026-09-23 and are marked as such; everything else carries over from the 2026-09-22 planning pass.
>
> Repo: `/home/computing/rev` (main worktree). This file lives in `.hermes/plans/`, which is
> gitignored — it is never committed, and it is copied into each agent worktree as
> `.dispatch/SPEC.md` (Wave 0.4).

**Goal (Phase A, reshape):** collapse Komun from a multi-community, key-ceremony platform into a
normal single-server web app. The server IS the community. Delete the piggPin relay, replace the
live collaborative map with OSM, keep only a public directory listing.

**Goal (Phase A, auth):** accounts work like a normal product — email + password, verified at
signup, reset by emailed link — while conversation encryption keeps working invisibly underneath.
Users never see key material.

**Goal (Phase B):** a peer-to-peer marketplace on the flattened model — listings and wanted ads,
negotiation over the existing encrypted match thread, deal completion, star reviews. No payment rails.

**Tech stack:** Rust 1.95 (Axum 0.8, sqlx runtime queries, Postgres 16), SvelteKit 5 (runes, static
adapter), vitest, Leaflet (new), lettre (new, email), komun-wasm (shrinks to message crypto).

---

## PART 0 — How this document is used

### 0.1 The two agents

| | **Agent A** | **Agent B** |
|---|---|---|
| Tool | Claude Code (headless, `claude -p`) | opencode (`opencode run`) |
| Model | `--model opus` via broker | `sandbox/deepseek-v4-flash` via broker |
| Container | `agent-repo-agent-a` | `agent-repo-agent-b` |
| Worktree | `/home/computing/repo-agent-a` → `/workspace` | `/home/computing/repo-agent-b` → `/workspace` |
| Branch | `feature/agent-a` | `feature/agent-b` |
| Owns | **The spine.** A1 (schema + core models + relay deletion), A2 (auth rework), A3 (API flattening), A6 (frontend flatten), and **every hub file** (Part 5). Phase B backend. | **Detached leaves only:** A4 (OSM map), A5 (federation deletion), A7.1/A7.2 (docs + deploy assets), Phase B frontend. Every file it writes is new, and every other change is a deletion. |
| Why | Highest reasoning load, security-critical, touches every shared file. | Bounded, mechanical, verify-by-command work; a cheap model with an exact brief is the right tool. |

Both containers hold **no credential**: a dummy token plus `http://rev-broker:4000`. Do not weaken this.

### 0.2 The three rules that keep two writers from colliding

1. **One writer per file.** A file has exactly one owner (Part 5). If an agent believes a file it
   does not own must change, it writes the exact diff into `.dispatch/hub-requests/<ID>.md` and keeps
   going — it never edits the file.
2. **Hub edits go through the orchestrator.** `api/mod.rs`, `config.rs`, `Cargo.toml`, `main.rs`,
   `crates/core/src/**`, `migrations/**` are hub files. Only Agent A edits them, and only when the
   orchestrator's card says so. Requests from B are applied by the orchestrator, to both branches, so
   the two branches stay convergent.
3. **Nothing merges until its gate passes.** The gate is re-run **by the orchestrator**, on the host,
   not taken from the agent's report. A green claim is not a green build.

### 0.3 How a wave is dispatched (the mechanical loop)

```bash
# 1. write the prompt file into the worktree the agent sees
$EDITOR ~/repo-agent-<x>/.dispatch/<ID>.md      # card body + STANDING RULES (0.4)

# 2. dispatch (Agent A)
docker exec -w /workspace agent-repo-agent-a bash -lc \
  'export HOME=/root; cd /workspace; claude --model opus --permission-mode acceptEdits -p "$(cat .dispatch/<ID>.md)"'

# 3. dispatch (Agent B)
docker exec -w /workspace agent-repo-agent-b bash -lc \
  'export HOME=/root; cd /workspace; opencode run -m sandbox/deepseek-v4-flash "$(cat .dispatch/<ID>.md)"'

# 4. watch the spend side, one line per model call
docker logs -f rev-broker

# 5. when the agent reports, review the diff and re-run its gate yourself
git -C ~/repo-agent-<x> status --short && git -C ~/repo-agent-<x> diff
```

Verified 2026-09-23: `claude -p "... --permission-mode acceptEdits"` writes files headlessly inside
`agent-repo-agent-a` (probe wrote `/tmp/claude-probe.txt`, contents `HELLO`), and
`opencode run -m sandbox/deepseek-v4-flash` wrote `agent-summary.md` (see §5 of `setup.md`).

### 0.4 STANDING RULES — pasted at the top of every dispatch prompt

```
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
   shows clippy lints, so "adds no new warnings" requires `cargo clippy -p <crate> --no-deps` — a
   build-only check has already hidden a `clippy::derivable_impls` warning in this project once.
8. On any provider/billing error, stop and report — do not retry in a loop.
9. Frontend work: Svelte 5 runes only ($state, $derived, $effect, $props; no $:, no export let, no on:click).
10. End your final message with exactly these lines:
   FILES WRITTEN: <paths>
   GATES RUN: <command -> observed result>
   BLOCKED: <none, or the precise blocker>
   DELETIONS REQUESTED: <paths, or none>
```

### 0.5 What counts as done

A card is done when **all** of these hold:

- every file in "Files you own" exists in the state the card describes;
- the card's gate commands were re-run by the orchestrator on the worktree and passed;
- the diff contains nothing outside the card's file list (scope creep is a finding, not a bonus);
- hub requests (if any) are applied and the worktree still builds after they are applied;
- `BLOCKED: none` — or the blocker is recorded and the card stays open.

---

## PART 1 — Frozen shared spec (both agents read this; it does not change mid-flight)

### 1.1 Locked decisions — Phase A reshape

| # | Decision | Choice |
|---|----------|--------|
| A1 | Communities | **Dropped.** The server is the single point of connection. Server identity comes from the existing `[node]` config. |
| A2 | Relay (piggPin) | **Deleted entirely** — 6,042 LOC across ~20 files, plus `relay_bridge.rs`, `relay_ops.rs`, `AppState.relay_store`, the `[relay]` config section, the `komun-relay` dependency and the workspace member. |
| A3 | Map | **OSM**: Leaflet + operator-configurable tile URL (default OSM public tiles + attribution). The existing Nominatim geocode proxy is hardened, not replaced. |
| A4 | Federation | **Public directory listing only.** Alliances and the federation module are deleted. |
| A5 | Existing data | **Fresh cut.** No backfill; schema squashed, DB recreated. |
| A6 | Migrations | **Squashed** into one fresh `migrations/001_schema.sql` (also kills the pre-existing constraint bugs by construction). |
| A7 | Registration | `[registration] mode = open \| invite \| closed`, **default `open`**, server-level invite codes for `invite` mode. |
| A8 | Roles | Server-level on `users.role`: **user / admin / superadmin**. |
| A9 | Visibility | Collapsed to **public / private**. |

### 1.2 Locked decisions — Phase A auth

| # | Decision | Choice |
|---|----------|--------|
| A10 | Sessions | **Opaque DB sessions.** 256-bit random token stored only as a hash, with device label, last-used, expiry, per-device revocation. **JWT disappears** — no `jsonwebtoken`, no shared `jwt_secret`; the operator holds no token-minting key. |
| A11 | Accounts | **Email + password, verified at signup, reset by emailed link.** SMTP via config (`[email]`, e.g. Zoho on 587 STARTTLS). This is the driver: a normal account, not a key ceremony. |
| A12 | Keys | **Invisible and automatic.** The x25519 secret key is created at signup and stored wrapped twice — once by a password-derived key, once by a recovery-code-derived key. Users never see, type or manage key material; no passphrase prompt, no proof-of-possession login. |
| A13 | Old identity layer | **Deleted.** ed25519 gone from the codebase entirely (client and server), along with signed-challenge login, the server-side recovery-id oracle, the fixed-salt KDFs and all `[recovery]` config. |

### 1.3 Locked decisions — Phase B marketplace

| # | Decision | Choice |
|---|----------|--------|
| B1 | Money | **Declarative price**; settlement in person. Structured offer/counter/accept ships anyway (what reviews need). No Stripe, no escrow, no payment-state columns. |
| B2 | Data model | Facet on `posts` + `listing` / `want` kinds. |
| B3 | Negotiation | Reuse the existing match thread; offer amounts in `match_offers` keyed to the same thread. |
| B4 | Trust | Star ratings + written reviews, writable only against a completed deal. |
| B5 | Expiry | Marketplace posts expire on the **same clock** as aid posts. |
| B6 | Wanted ads | **In scope** — `PostKind::Want`. |
| B7 | Currency | Per-listing ISO-4217 + optional server-level `[market] default_currency`, unset by default. |
| B8 | Categories | **Seeded `categories` table**, one flat shared list with a `scope` column, admin-editable at runtime: 15 market-only + 6 both + 2 aid-only = 23 entries (Part 1.6). |

**Hard constraint throughout:** message content is never readable by the server. Session tokens,
email addresses, password verifiers and wrapped key bundles are server-visible; plaintext messages
and unwrapped keys are not.

### 1.4 Target schema (fresh baseline) — `migrations/001_schema.sql` replaces 001-015

```
users              id, email TEXT UNIQUE (stored lowercased), email_verified_at,
                   password_hash TEXT,                  -- Argon2id(verifier)
                   auth_salt BYTEA,                     -- salt for the client-side verifier
                   display_name, role CHECK ('user','admin','superadmin'),
                   bio, avatar_path, profile_json JSONB, last_seen, created_at,
                   encryption_public_key BYTEA,         -- x25519, for message E2E
                   encrypted_key_bundle BYTEA,          -- x25519 secret, wrapped by the password key
                   bundle_salt BYTEA,
                   encrypted_recovery_bundle BYTEA,     -- x25519 secret, wrapped by the recovery-code key
                   recovery_bundle_salt BYTEA
                   (public_key / recovery_id / recovery_code_hash are GONE)
sessions           id, user_id -> users ON DELETE CASCADE,
                   token_hash BYTEA UNIQUE NOT NULL,    -- SHA-256 of the 256-bit token
                   device_label, user_agent, ip,
                   created_at, last_used_at, expires_at, revoked_at
one_time_tokens    id, user_id -> users ON DELETE CASCADE,
                   kind CHECK ('email_verify','password_reset'),
                   token_hash BYTEA UNIQUE NOT NULL, expires_at, used_at, created_at
audit_events       id, actor_id -> users, action, subject_id, detail JSONB, created_at
invites            code PK, created_by, uses_remaining, expires_at, created_at
categories         slug TEXT PK, label TEXT NOT NULL, scope CHECK ('aid','market','both'),
                   sort_order INT NOT NULL DEFAULT 0, active BOOL NOT NULL DEFAULT true,
                   created_at, updated_at        -- seeded by this migration
posts              id, author_id -> users, kind CHECK, category -> categories(slug) ON DELETE RESTRICT,
                   title, body,
                   location_name/lat/lon, urgency, quantity,
                   status CHECK ('active','matched','fulfilled','expired','withdrawn',
                                 'hidden','flagged'),
                   visibility CHECK ('public','private'),
                   expires_at, tags TEXT[], contact_method, images TEXT[],
                   verified_by -> users, verified_at,
                   market_listed BOOL, price_cents BIGINT, currency TEXT,
                   price_negotiable BOOL, item_condition TEXT, sold_at, buyer_id -> users,
                   search_vector tsvector + trigger + GIN index,
                   created_at, updated_at          (community_id gone)
matches            id, post_id -> posts, responder_id -> users, responder_post_id -> posts,
                   message, status CHECK ('proposed','accepted','completed','withdrawn'),
                   agreed_price_cents, currency, created_at, resolved_at
messages           id, match_id -> matches, sender_id -> users,
                   ciphertext BYTEA NOT NULL, nonce BYTEA, created_at
                   (body TEXT is gone: message content is encrypted, never plaintext)
match_offers       id, match_id -> matches, actor_id -> users,
                   kind CHECK ('offer','counter','accept','decline'),
                   amount_cents, currency, note, created_at
deal_reviews       id, match_id -> matches, reviewer_id, reviewee_id,
                   rating SMALLINT CHECK (1..5), body, created_at, UNIQUE (match_id, reviewer_id)
endorsements       id, endorser_id, endorsee_id, note, created_at, UNIQUE (pair)
notifications      id, user_id, kind, title, body, link, read, created_at
reports            id, reporter_id, post_id, reason, status, admin_notes,
                   resolved_by, resolved_at, created_at
avatar_uploads     id, user_id, uploaded_at
directory_entries  url PK, name, description, location_name/lat/lon, version,
                   last_seen, registered_at
```

Deleted: `communities`, `members`, `alliances`, `users.public_key`, `users.recovery_id`, the map
columns, `invites.community_id`, `posts.community_id`, `directory_entries.communities_count` /
`community_locations`, `[auth] jwt_secret`, the whole `[recovery]` concept, and the `Category` enum
(replaced by the `categories` table). Carry over every index and the `posts_search_update()` trigger
from `014_post_fts.sql`.

### 1.5 Auth design (the parts a schema cannot express)

**Password handling — the server never receives the password.** The client derives two different
values from the password with two different salts: `verifier = Argon2id(password, auth_salt)` is sent
and stored (server-side Argon2id on top), while `wrap_key = Argon2id(password, bundle_salt)` is
derived locally and never transmitted. The server holds a verifier it cannot invert and nothing that
can unwrap the x25519 secret. This is what lets a normal email + password form coexist with the
server being unable to read messages.

**Signup.** Email + password (+ display name). The browser generates an x25519 keypair and uploads the
secret key wrapped twice — once with `wrap_key`, once with a key derived from a freshly generated
12-word recovery code — plus the public key. There is no separate master key: the wrapped secret IS
the key material, so re-wrapping on a password change or code reissue preserves history with nothing
extra to keep in sync. The server stores the account unverified and emails a verification link.

**Verification.** `one_time_tokens` row of kind `email_verify`, single-use, 24-hour expiry. Until
verified the user can sign in but cannot post, respond, message or create listings (confirmed).

**Login.** Email + password → the client sends the verifier; the server verifies it against
`password_hash`, creates a session, returns the wrapped bundle. The client unwraps the x25519 secret
in memory with `wrap_key` and holds it for the session. No passphrase prompt, no key UI.

**Password reset by email.** `POST /auth/password/forgot {email}` always returns 200 (no account
enumeration) and sends a single-use 30-minute link. `POST /auth/password/reset {token,
new_password, [recovery_code]}`: the reset token authorises fetching the wrapped bundles; if the
recovery code is supplied the client unwraps the x25519 secret and re-wraps it under the new
password, so message history survives. **Without the recovery code, old messages are unreadable** —
the account works, history does not come back. That is the accepted tradeoff (A12).

**Change password (signed in).** Requires re-entering the current password, which lets the client
unwrap and re-wrap; other sessions are revoked, and the wrapped secret is re-wrapped rather than
replaced, so other devices keep working with no coordination.

**Sessions.** `POST /auth/login` returns a 256-bit random token; the server stores only
`SHA-256(token)`, so a database leak yields no usable tokens. The middleware loads `user_id` AND
`role` from the DB on every request — no stale role claim, and a demotion takes effect on the next
request. Sliding `last_used_at`, throttled to one write per minute.

**Rate limiting.** A small in-process token bucket keyed by IP + route class over signup, login,
forgot-password and reset. `X-Forwarded-For` is honoured **only** from configured trusted proxies,
otherwise a spoofed header defeats the limiter. Forgot-password is additionally limited per email
address to prevent mail bombing.

**Email.** `[email]` config: `smtp_host`, `smtp_port`, `username`, `password`, `from`, `starttls`, and
a `public_url` link base. Startup fails loudly when `[registration] require_email_verification` is
true and SMTP is unconfigured.

**Honest limitation to document, not hide.** Browser-delivered E2E cannot protect against a malicious
server serving modified JavaScript. This design protects against database theft, passive disk reads,
an operator reading message content, and admin snooping. It does not protect against a hostile
operator who ships modified client code. The docs must say exactly that.

**What stays in `crates/wasm`:** x25519 ECDH, ChaCha20Poly1305, Argon2 (verifier + wrap key) and
BIP39 recovery-code generation. **What goes:** ed25519 entirely — `KeyPair`, `generate_keypair`,
`sign`, `verify`, the imports at `crates/wasm/src/lib.rs:6`, the code at 13-67, `ed25519-dalek` in
`crates/wasm/Cargo.toml:15` and `crates/server/Cargo.toml:27`, the server's verifying-key code, the
three ed25519 tests in `crates/server/src/tests/mod.rs:143-180`, `signRegisterChallenge` and the
ed25519 fields in `web/src/lib/crypto.ts`, the ed25519 fields in `web/src/lib/stores/auth.ts`,
`compute_recovery_id` (`crates/wasm/src/lib.rs:311`), and the signing cases in
`web/src/tests/crypto.test.ts`.

One format consequence: `encrypt_key_bundle` (`crates/wasm/src/lib.rs:257-266`) concatenates an
ed25519 secret with an x25519 secret, and `recoverFromBundle` reverses that with a `bytes.slice(0, 32)`
split (`crypto.ts:112`). With ed25519 gone the bundle becomes a single-secret wrap, so both sides
change together. Safe only because the data cut is fresh (A5).

### 1.6 Categories (seeded table, one shared list)

`category` today is `TEXT NOT NULL` with **no CHECK constraint** (`migrations/001_schema.sql:42`),
held together only by the 8-value Rust enum — which is why nothing caught it drifting. It becomes a
`categories` table so the taxonomy is seed data: an admin adds, renames, reorders or retires a
category without a release. The enum disappears with it.

23 rows, flat, one list serving both halves; `scope` decides which form shows what:

| scope | slug | label |
|---|---|---|
| market | electronics | Electronics & Computers |
| market | furniture | Furniture & Home |
| market | appliances | Appliances |
| market | tools | Tools & Equipment |
| market | clothing | Clothing & Accessories |
| market | bikes-vehicles | Bikes & Vehicles |
| market | books-media | Books & Media |
| market | garden-outdoors | Garden & Outdoors |
| market | sports | Sports & Fitness |
| market | toys-games | Toys & Games |
| market | baby-kids | Baby & Kids |
| market | building-materials | Building Materials |
| market | art-craft | Art & Craft |
| market | household | Household Goods |
| market | free | Free / Give Away |
| both | services | Services & Labor |
| both | food | Food |
| both | health | Health |
| both | education | Education |
| both | legal | Legal |
| both | other | Other |
| aid | shelter | Shelter |
| aid | transport | Transport |

`services` absorbs the old aid `labor`. "Wanted" is deliberately **not** a category — it is
`PostKind::Want`. The scope assignments are seed data: changing one is a single `UPDATE`.

`posts.category` becomes a slug FK (`ON DELETE RESTRICT`), keeping `idx_posts_category`, so a category
in use cannot be deleted — admins set `active = false` to retire one. `GET /api/categories?scope=`
returns the active list; post list/detail queries LEFT JOIN to also return `category_label`. The FTS
trigger indexes the **label**, not the slug, so search matches the word a human types; that makes a
rename a two-part operation (B-q3's rename path re-runs the FTS update for affected posts).

---

## PART 2 — Ground truth: what is true today, and what must not come back

### 2.1 Verified baseline (2026-09-22)

Toolchain: rustc 1.95.0, cargo 1.95.0, node v26.8.2, npm 12.0.2, wasm-pack present, Postgres 16
running locally on `:5432`. (The docker daemon was down during that pass; it is up now.)

`cargo check -p komun-core` passes. `cargo test -p komun-core` **fails to compile**:
`crates/core/src/tests.rs:135` is missing `image_path` after migration 015. The README's "110 tests
passed" badge is not true of this repo. A1.2 fixes this by deleting the community tests.

Blast radius: **684 Rust references** to communities, **280 web references across 25 files**.

Tables referenced by code vs defined by migrations match today: `users, posts, matches, messages,
invites, alliances, members, communities, directory_entries, notifications, reports, endorsements,
avatar_uploads`. No drift.

**The archived komun skill describes code that is NOT in this clone.** It documents web push
notifications (`push_subscriptions`, `vapid_keys.json`, `crates/server/src/push.rs`). None of it
exists here — every `push` hit is inside `crates/relay`. Treat the skill as intent, not as a
description of this codebase.

### 2.2 Pre-existing bugs that die with the squash (recorded so they are not reintroduced)

| # | Problem | Evidence |
|---|---|---|
| P1 | `chk_posts_visibility` allows `('local','federated','private')` but code writes `'public'` | `db/posts.rs:77`, `db/communities.rs:34`, `api/communities.rs:82` |
| P2 | `chk_matches_status` excludes `'completed'` but code writes it | `db/conversations.rs:238,251` |
| P3 | `chk_posts_status` lacks `'matched'`/`'hidden'`/`'flagged'` which the enum has and code writes | `core/models/post.rs:37-45`, `reports.rs:95` |
| P4 | `db/posts.rs:71-75` builds enum strings via a `serde_json` round-trip (convention forbids it) | same file |
| P5 | **`config.example.toml` cannot boot the server as shipped.** Its placeholder `jwt_secret` is 36 chars; `main.rs:50` requires **≥40** and `config.rs:324` requires ≥32 — three numbers that disagree, and the example fails the strictest one. A verbatim copy dies with `Error: JWT_SECRET is too short (< 40 chars)`. Measured during B1's boot gate. | `config.example.toml:39`, `crates/server/src/main.rs:50`, `crates/server/src/config.rs:324` |

P1-P3 exist because CHECK lists and Rust enums were maintained separately with nothing checking
agreement. A1.3 pins them together with a test.

### 2.3 Why the old auth is being deleted (the record, not a to-do list)

**F1 — Fixed-salt recovery id was a cross-deployment dictionary oracle.** `compute_recovery_id`
(`crates/wasm/src/lib.rs:311`) is `Argon2id(passphrase, salt = b"komun-recovery-v1")[..16]` — a
hardcoded salt compiled into the WASM, identical on every deployment. `POST /auth/recover` returned
`encrypted_key_bundle`, `bundle_salt`, name and public keys with no rate limit and, when
`recovery_code_hash` was NULL, no check at all (`auth/mod.rs:314-321`). It also made `recovery_id`
UNIQUE, so two users picking the same passphrase could not both register.

**F2 — No login endpoint; `register` doubled as login, and the challenge store leaked.** The client
never called `/auth/challenge` or `/auth/verify-challenge` (`crypto.ts:89` generated its own
challenge); the server's `CHALLENGES` map therefore only grew, fed by an unauthenticated route
(`auth/mod.rs:98-99, 349-350`).

**F3 — A stolen token was permanent account takeover.** `PUT /auth/me/update_profile` accepted new
`encrypted_key_bundle`, `bundle_salt`, `recovery_id` and `recovery_code_hash` from any bearer token
(`auth/mod.rs:451-462, 473-484`), the token lived 30 days, and nothing could revoke it — the client's
`logout()` (`web/src/lib/stores/auth.ts:362`) only cleared local state.

**F4 — HS256 shared secret, and the shipped default was public.** `config.example.toml` shipped a
36-character placeholder `jwt_secret`, which passed the `len() < 32` check at `config.rs:273`.

**F5 — Registration rate limiting was global, not per-IP** (`auth/mod.rs:156-171` counted every user
created in an hour, so one spammer locked out real neighbours). Fixed by the per-IP limiter (A2.6).

**F6/F7 — Convention drift and a wrong response shape:** env-var `JWT_SECRET` reads
(`auth/mod.rs:670,721`), and `GET /auth/me` returning `token: ""` (`auth/mod.rs:422`).

The replacement in Part 1 avoids all of these by construction.

---

## PART 3 — Sandbox contract (measured 2026-09-23, not assumed)

### 3.1 Topology as it stands

| Container | Role | Network | Key mounts |
|---|---|---|---|
| `rev-broker` | holds the real keys, injects them upstream | `agent-net` + `bridge` | `~/.claude` (rw), `~/.config/komun-sandbox` (rw) |
| `agent-repo-agent-a` | Agent A | `agent-net` only | `~/repo-agent-a` → `/workspace`, `repo-agent-a-cargo-target`, `komun-cargo-registry`, broker's opencode config (ro) |
| `agent-repo-agent-b` | Agent B | `agent-net` only | `~/repo-agent-b` → `/workspace`, `repo-agent-b-cargo-target`, `komun-cargo-registry`, broker's opencode config (ro) |
| `agent-rev` | spare container for the main worktree | `agent-net` only | `~/rev` → `/workspace` |

Both agent containers also mount `/home/computing/rev/.git` at the same absolute path — the worktrees
share one object store, so each agent can see the other's branches and commits. That is a
coordination channel and a collision surface; see Part 5.6.

### 3.2 What is reachable (measured from inside `agent-repo-agent-a`)

```
broker http://rev-broker:4000/health   -> 200
https://api.anthropic.com/v1/messages  -> 000   (no route, DNS does not resolve)
http://172.17.0.1:5432/                -> 000   (host Postgres NOT reachable)
getent hosts host.docker.internal      -> no such host
```

`rev-broker` runs `ThreadingHTTPServer`, so Agent A and Agent B can call the provider concurrently
without serialising. Broker routes: `/v1/messages*` and `/api/oauth/claude_cli/roles` → Anthropic;
`/v1/chat/completions`, `/v1/models`, `/v1/embeddings` → DeepSeek. Everything else 404s. `/health`
never returns secret material; `POST /refresh` forces the OAuth rotation. **There is no model
allowlist and no spend cap** — the broker is an oracle the agents can spend through. Cost control is
Part 9.

### 3.3 The three gaps this document has to work around

1. **git inside the containers fails today.** Both worktrees are owned by uid 1000 while the
   containers run as root, so every git command dies with
   `fatal: detected dubious ownership in repository at '/workspace'`. Measured, reproduced in both
   agent containers. Ugliest side effect: an agent cannot self-review a diff. Fix in Wave 0.1. (The
   containers run as root because `run-agent.sh` does not pass `--user`; mounting a workspace rw as
   root also means files created inside land root-owned on the host — the orchestrator `chown`s them
   back, or the entrypoint is changed later.)
2. **The host database is out of reach.** `127.0.0.1:5432` is unreachable from `agent-net`, so every
   DB-touching gate (schema probes, integration tests, a running server) needs a Postgres **container
   on `agent-net`** — proven: `psql postgres://komun:***@<container>:5432/<db>` from inside
   `agent-repo-agent-a` returned `PostgreSQL 16.15`. One per worktree, distinct DB names (Wave 0.2).
3. **No egress means no new dependencies.** `lettre` (A2) and `leaflet` (A4) are both absent from
   `Cargo.lock` and `web/package.json`, so neither can be added from inside a sandbox. The pre-warm
   path is Wave 0.5, and its deliberate opt-out (`docker network connect bridge <container>`, add the
   dep, disconnect) is the only sanctioned way to break the isolation, only for that step.

### 3.4 Sandbox rules the agents run under (unchanged, do not weaken)

- Agent containers hold **no credential**: `ANTHROPIC_AUTH_TOKEN=sandbox-dummy-token` and a broker URL.
- Agent containers have **no internet**: internal network only.
- The workspace mount is **read-write**, so a hallucinated `rm` can destroy work — mitigated by one
  worktree per agent and by never committing without review.
- `config.toml` is gitignored and therefore **absent from a fresh worktree** (verified for both).
  Each worktree needs its own copy (Wave 0.3).
- The agents' root-owned writes on the bind mount need `chown computing:computing` on the host.
- **`DATABASE_URL` is not exported inside the sandbox** (verified: the container env carries only
  `ANTHROPIC_BASE_URL` / the dummy token). Gates and prompts must use the literal connection string
  from 0.2 (`postgres://komun:komun@komun-db-<x>:5432/komun_<x>`); the server itself reads
  `[database].url` out of the worktree's `config.toml`.
- **A login shell loses cargo.** The image sets `PATH=/usr/local/cargo/bin:...` and
  `CARGO_TARGET_DIR=/workspace/target` as container ENV, but `bash -lc` re-reads `/etc/profile` and
  drops `/usr/local/cargo/bin` (measured: `which cargo` empty under `-lc`, resolved under a plain
  shell). So: enter with `docker exec -it <container> bash` (no `-l`), or prefix
  `export PATH=/usr/local/cargo/bin:$PATH`. Every gate command in this document does the export;
  keep it in any new one.

---

## PART 4 — WAVE 0: pre-flight (orchestrator only; no agent is dispatched until 0.6 passes)

### 0.1 — Fix git inside both containers

```bash
for c in agent-repo-agent-a agent-repo-agent-b; do
  docker exec "$c" bash -lc 'git config --global --add safe.directory "*"'
  docker exec "$c" bash -lc 'cd /workspace && git status --short | head; git log --oneline -1'
done
```

**Verified:** after this, `git branch -a` inside `agent-repo-agent-a` lists `feature/agent-a`,
`feature/agent-b` and `main` — the shared object store is visible. The setting is container-local
(`/root/.gitconfig`) and dies with the container, so it must be re-applied after every
`run-agent.sh` — candidate addition to the script's tail.

### 0.2 — One Postgres per worktree, on `agent-net`

```bash
docker pull postgres:16-alpine          # now present locally
for x in a b; do
  docker run -d --name "komun-db-$x" --network agent-net \
    -e POSTGRES_USER=komun -e POSTGRES_PASSWORD=komun -e POSTGRES_DB="komun_$x" \
    postgres:16-alpine
done
docker exec agent-repo-agent-a bash -lc \
  'psql "postgres://komun:komun@komun-db-a:5432/komun_a" -c "select 1"'
docker exec agent-repo-agent-b bash -lc \
  'psql "postgres://komun:komun@komun-db-b:5432/komun_b" -c "select 1"'
```

Two databases, not one: parallel agents sharing a test DB collide on unique constraints and on any
server-side limit computed over the whole DB (Komun has `max_registrations_per_hour`). Each worktree
gets its own; **never truncate the other one's DB**.

### 0.3 — A `config.toml` per worktree

```bash
for x in a b; do
  cp ~/rev/config.example.toml ~/repo-agent-$x/config.toml
done
# then, in each file, set [database].url to that worktree's DB:
#   ~/repo-agent-a/config.toml -> postgres://komun:komun@komun-db-a:5432/komun_a
#   ~/repo-agent-b/config.toml -> postgres://komun:komun@komun-db-b:5432/komun_b
```

`config.toml` is gitignored, so this never reaches a commit. The orchestrator refreshes both copies
after any wave that changes `config.rs` (A2 adds `[email]`, A4 adds `[map]`/`[geocode]`, A5/A6 remove
`[relay]`/`[federation]`, A7 removes `[auth] jwt_secret`).

### 0.4 — The spec both agents read

```bash
for x in a b; do
  mkdir -p ~/repo-agent-$x/.dispatch/hub-requests
  cp ~/rev/.hermes/plans/2026-09-22_1620_komun-reshape-auth-marketplace.md \
     ~/repo-agent-$x/.dispatch/SPEC.md
done
```

`.dispatch/` stays untracked: the orchestrator always stages files by explicit path, never
`git add -A`. Re-copy `SPEC.md` after every edit to this document, or the agents work from a stale spec.

### 0.5 — Pre-warm the two dependencies the plan adds (no egress inside the sandbox)

```bash
# npm + wasm. Two corrections from the first attempt (2026-09-23):
#   (a) run in web/, not the worktree root — the app's package.json lives in web/; `-w /w` would have
#       created a stray root package.json and installed nothing the app uses;
#   (b) build the wasm package FIRST: web/package.json depends on `komun-wasm: file:../crates/wasm/pkg`,
#       so npm install fails outright while pkg/ is missing. The image has no wasm32 target and no
#       wasm-bindgen, so this is the ONLY place the wasm build can run (bridge network, throwaway).
docker run --rm --network bridge --entrypoint bash \
  -v ~/repo-agent-b:/w -v komun-cargo-registry:/usr/local/cargo/registry -w /w agent-sandbox:komun \
  -lc 'export PATH=/usr/local/cargo/bin:$PATH
        rustup target add wasm32-unknown-unknown
        wasm-pack build crates/wasm --target web
        cd /w/web && npm install --no-fund --no-audit && npm install leaflet --no-fund --no-audit
        node -e "console.log(require(\"leaflet/package.json\").version)"'
# -> wasm-pack "Done in 16.41s", 154 packages + 3 (leaflet 1.9.4), pkg/ = komun_wasm.js/.d.ts/.wasm
#    (verified in Agent B's worktree 2026-09-23, then chown -R computing back)
# Both artifacts are gitignored (pkg/ .gitignore:5, node_modules/ .gitignore:2) so B's scope check stays
# clean. The rustup target lives in the THROWAWAY container: agents still cannot run wasm-pack. Rebuild
# pkg/ with this command after any change to crates/wasm/src/lib.rs (A2a/A2b both change it).

# cargo: lettre, fetched into the SHARED registry volume both containers mount
docker run --rm --network bridge --entrypoint bash \
  -v komun-cargo-registry:/usr/local/cargo/registry \
  -v ~/.hermes/profiles/dev/cache/scratch/depwarm/crate:/w -w /w agent-sandbox:komun \
  -lc 'export PATH=/usr/local/cargo/bin:$PATH; cargo add lettre --features smtp-transport; cargo fetch'
# -> lettre 0.11.23 in /usr/local/cargo/registry/src/index.crates.io-*/lettre-0.11.23  (verified)
# RESOLVED 2026-09-23: the exact feature set A2a needs — lettre 0.11 with {smtp-transport, tokio1-rustls-tls,
# builder, hostname}, plus argon2 0.5 and sha2 0.10 — resolves and builds FULLY OFFLINE from this cache.
# Cargo.lock gains all three and `CARGO_NET_OFFLINE=true cargo build -p komun-server` → Finished, exit 0.
# argon2/sha2 were already in the lock (komun-wasm depends on them); lettre was the only new edge, and the
# expanded feature set needed no extra fetch beyond what this pre-warm already pulled.
```

Run these **before** the card that needs them, with `--network bridge` on the throwaway container —
never on the agent container. If a later wave needs a version or feature these commands did not
fetch, repeat the same shape and re-run `cargo fetch`. The sanctioned opt-out if something must be
resolved interactively inside an agent container:

```bash
docker network connect bridge agent-repo-agent-a     # temporarily
#   ... install / fetch ...
docker network disconnect bridge agent-repo-agent-a  # immediately after, then verify again:
docker exec agent-repo-agent-a bash -lc 'curl -s -m 5 -o /dev/null -w "%{http_code}\n" https://crates.io'  # -> 000
```

### 0.6 — Prove the boundary before dispatching anything

```bash
docker exec rev-broker python3 -c "import os;print(os.path.exists('/secrets/claude/.credentials.json'), os.path.exists('/state/deepseek.key'))"
docker exec agent-repo-agent-a bash -lc 'curl -s http://rev-broker:4000/health'
docker exec agent-repo-agent-a bash -lc 'curl -s -m 5 -o /dev/null -w "%{http_code}\n" https://api.anthropic.com'
docker exec agent-repo-agent-a bash -lc 'env | grep -Ei "token|key"'          # dummy only
docker exec agent-repo-agent-a bash -lc 'find / -xdev -name "*.credentials.json" -o -name "deepseek.key"'  # empty
docker exec -w /workspace agent-repo-agent-a bash -lc 'export HOME=/root; cd /workspace; claude --model opus -p "Reply with exactly: A_OK"'
docker exec -w /workspace agent-repo-agent-b bash -lc 'export HOME=/root; cd /workspace; opencode run -m sandbox/deepseek-v4-flash "Reply with exactly: B_OK"'
```

Expected: `True True` / `{"ok": true, ...}` / `000` / dummy only / no output / `A_OK` / `B_OK`.
`docker logs rev-broker` shows one `200` line per call. Wave 0 is done when all seven are right.

---

## PART 5 — Ownership map, hub protocol, and what the agents may never do

### 5.1 Agent A owns (spine + hub files)

```
migrations/**                          (001-015 deleted, 001_schema.sql created)
crates/core/src/**                     (models, tests, mod)
crates/server/src/main.rs              (relay spawn block, env set_var)
crates/server/src/config.rs            ([email], [map], [registration], [market]; [relay]/[federation] out)
crates/server/src/api/mod.rs           (route mounting: flattening + deletions)
crates/server/src/auth/**
crates/server/src/rate_limit.rs        (new)
crates/server/src/sessions.rs          (new; middleware)
crates/server/src/db/**                (posts, users, conversations, sessions; communities/alliances deleted)
crates/server/src/api/**               (posts, search, conversations, geocode, admin, users, reports,
                                        endorsements, notifications, health, link_preview)
                                        NOT api/node.rs or api/directory.rs — those are B's (see 5.2). A1
                                        removing `[relay]` broke api/node.rs:68, which read config.relay;
                                        that one break was repaired by the orchestrator on both branches.
crates/server/src/tasks/**             (expiry, bundle_cleanup, health)
                                        NOT tasks/registration.rs — B's (see 5.2)
crates/server/src/repl.rs, security_headers.rs
crates/server/src/tests/**             (incl. the new tests/market.rs)
crates/wasm/src/lib.rs                 (ed25519 out, single-secret bundle)
crates/wasm/Cargo.toml, crates/server/Cargo.toml, Cargo.toml (workspace)
web/src/lib/api/**                     (the API client — the frontend's hub file)
web/src/lib/stores/auth.ts, web/src/lib/crypto.ts
web/src/routes/+layout.svelte, +page.svelte, aid/**, search/**, admin/**, users/[id]/**,
web/src/routes/p/[id]/**               (created by A6.1)
web/src/tests/**
```

### 5.2 Agent B owns (new files and deletions only)

```
web/src/lib/components/LocationMap.svelte        (new)
web/src/routes/map/+page.svelte                  (new)
web/src/lib/components/MarketCard.svelte         (new, Phase B)
web/src/lib/components/OfferPanel.svelte         (new, Phase B)
web/src/lib/components/DealReviewModal.svelte    (new, Phase B)
web/src/routes/market/**, web/src/routes/market/new/**   (new, Phase B)
web/src/lib/api/market.ts, web/src/lib/api/categories.ts (new, Phase B)
docs/ARCHITECTURE.md, docs/CONVENTIONS.md, docs/CRYPTO.md, docs/DATABASE.md, docs/DEVELOPMENT.md
README.md, AGENTS.md
deploy/nginx-komun.conf, deploy/komun.initd, deploy/setup.sh, deploy/seed.sql
config.example.toml
crates/server/src/api/node.rs                    (A5.2/B3: drop communities_count + federation_enabled;
                                                  `relay_url` was already removed by the orchestrator)
crates/server/src/api/directory.rs               (A4/B3)
crates/server/src/tasks/registration.rs          (A5.1/B3)
deletions: crates/server/src/api/alliances.rs, crates/server/src/db/alliances/**,
           crates/server/src/federation/**
```

B never edits: `api/mod.rs`, `config.rs`, any `Cargo.toml`, `main.rs`, `crates/core/**`,
`migrations/**`, `web/src/lib/api/**` (existing modules), `web/src/lib/stores/**`,
`crates/relay/**` (A1.4 deletes it).

### 5.3 Hub protocol (how B gets a hub file changed)

1. B writes `.dispatch/hub-requests/<ID>.md` containing: the file path, the exact unified diff, the
   reason, and the gate command that will prove it. It does **not** edit the file.
2. The orchestrator reads the request, applies it to **both** worktrees (so the branches stay
   convergent — this matters at merge time), and runs that worktree's build gate.
3. If applying it breaks the other agent's in-flight work, the orchestrator applies it to the
   requesting branch only and records it in Part 7's merge notes.

Three hub requests are already expected, and are pre-approved as *shapes*: `config.rs` `[map]` block
(A4.1), `config.rs` `[geocode] contact` (A4.2), `api/mod.rs` route-mount deletions (A5.1/A5.2).

### 5.4 Git rules

- **Agents never run git mutations.** The orchestrator commits, merges and rebases, on the host.
- Commit messages: `feat(A2): ...` / `fix(A1.3): ...`, one card per commit, only after its gate passes.
- **No push, ever, without explicit per-action approval from the user** — and never a push of code
  whose gate has not passed locally.
- Chown after every agent wave: `sudo chown -R computing:computing ~/repo-agent-<x>` for files the
  root container created.

### 5.5 Collision surfaces that are already known

- `web/src/routes/c/[slug]/p/[id]/+page.svelte` is deleted by A6.1 while A4.3 wants to add a map to
  the post detail page. **Order fix:** A4.3 adds the standalone `/map` route + `LocationMap.svelte`
  first; the post-detail insertion waits until after A6 lands, and then targets
  `web/src/routes/p/[id]/+page.svelte`. Recorded in card B2 and card A6.
- `api/mod.rs` is rewritten by A3.2 and has mounts deleted by A5.1. Both arrive at merge time; the
  resolution rule is in Part 7.2.
- `config.rs` is touched by A2 (`[email]`), A4 (`[map]`, `[geocode]`), A5/A6 (removals).
- `web/src/lib/components/index.ts` (if it exists) is a shared barrel file — B adds files there only
  with the orchestrator's say-so in the card.

### 5.6 The shared object store

Both worktrees see the same `.git`, so Agent B can read Agent A's branch
(`git show feature/agent-a:path`) without a remote. That is useful for reading a frozen interface,
but it is **not** a licence to build against code that has not landed on B's branch: B's card lists
what exists on B's branch, and nothing else. No agent rebases its own branch.

---

## PART 6 — DISPATCH CARDS

Card fields: **Owner · branch · depends on · goal · files you own · do not touch · prompt · gate ·
exit criteria.** Prompt = the card's Goal/Files text plus the STANDING RULES block (Part 0.4),
written to `.dispatch/<ID>.md`.

### Wave A1 — Fresh baseline + core models  *(Agent A)*

- **Depends on:** Wave 0. **Blocks:** everything.
- **Files you own:** `migrations/**`, `crates/core/src/**`, `crates/relay/**` (delete),
  `crates/server/src/relay_bridge.rs` + `relay_ops.rs` (delete), `crates/server/src/main.rs`,
  `Cargo.toml`, `crates/server/Cargo.toml`, `crates/relay/Cargo.toml`.
- **Do not touch:** `crates/server/src/auth/**`, `api/**`, `db/**` (A2/A3), anything under `web/`.
- **Task:**
  1. **A1.1** Delete `migrations/001-015`; create `migrations/001_schema.sql` with Part 1.4 in full,
     carrying over every index and the `posts_search_update()` trigger from `014_post_fts.sql`.
  2. **A1.2** Delete `community.rs` / `member.rs`; `user.rs` becomes email/password oriented with a
     typed `role` enum; `post.rs` loses `community_id` and collapses `Visibility`;
     `match_thread.rs`'s `Message` carries `ciphertext`/`nonce` instead of `body`; `tests.rs` drops
     the community tests (this also removes the `image_path` compile breakage).
  3. **A1.3** In `crates/core/src/tests.rs`, for every enum that maps to a DB text column
     (`PostKind`, `Urgency`, `PostStatus`, `Visibility`, `UserRole`, plus Phase B's `ItemCondition`
     and `OfferKind`) assert that `as_str()`'s value set equals the `CHECK (col IN (...))` list parsed
     out of `migrations/001_schema.sql`. `Category` is no longer an enum, so it gets its own test
     instead: every seeded row's `scope` is one of the CHECK values, slugs are unique and
     lowercase-kebab, and no label is empty.
  4. **A1.4** Delete `crates/relay/`, `relay_bridge.rs`, `relay_ops.rs`, `data/relay/`, the `[relay]`
     config section, the `komun-relay` dependency and workspace member, and `main.rs`'s spawn block
     (lines 70-91). **The orchestrator performs this deletion, not you** (STANDING RULE 2): list the paths
     under DELETIONS REQUESTED and carry on.
  5. **A1.5 — minimum repair so the workspace compiles again.** A1.2 removes the community model, which
     breaks three files that are not yours: `crates/server/src/db/posts.rs` (14 errors),
     `crates/server/src/api/communities.rs` (7), `crates/server/src/db/communities.rs` (4). Repair them in
     the smallest way that compiles, keeping their present behaviour. Do NOT redesign them and do not move
     their logic: A3.1 deletes `api/communities.rs` + `db/communities.rs` outright and writes the real
     `api/posts.rs`, so any elegance spent here is thrown away. This step exists only so A2a's
     `cargo test --workspace` / clippy gates have a compiling workspace to run against.
- **Prompt:** the five steps above verbatim + STANDING RULES.
- **Gate (orchestrator re-runs, in `agent-repo-agent-a`):**
  ```bash
  docker exec -w /workspace agent-repo-agent-a bash -lc 'export PATH=/usr/local/cargo/bin:$PATH; cd /workspace
  cargo test -p komun-core
  cargo build --workspace
  grep -ri piggpin crates/ --exclude-dir=target | wc -l                     # expect 0 (whole-repo grep: A6/A7)
  grep -c "communities\|members" migrations/001_schema.sql                  # expect 0
  psql "postgres://komun:komun@komun-db-a:5432/komun_a" -c "\dt"'
  ```
  Then the negative inserts: negative price, `market_listed = true` on kind `need`, lowercase
  currency, `item_condition = 'mint'`, rating 6, duplicate review, offer kind `'bid'`, `role = 'root'`,
  `visibility = 'federated'`, duplicate email differing only by case, an unknown category slug (FK
  rejected), and deleting a category that has posts (`RESTRICT` rejected).
- **Exit criteria:** `cargo test -p komun-core` green; `cargo build --workspace` clean **after A1.5**
  (A1.1-A1.4 alone cannot build the workspace — deleting the community model breaks A3's files); `\dt`
  lists exactly the tables in Part 1.4; every negative insert rejected; the A1.3 test demonstrably fails
  when one CHECK entry is deleted by hand (do that once, then restore it).
- **Explicitly NOT A1's gate:** the whole-repo `piggpin` grep. After A1's deletions its only survivors are
  B-owned (`README.md`, `AGENTS.md`, `docs/**`, `config.example.toml` — A7.1/A7.2) and A6-owned
  (`web/src/routes/**`, `web/svelte.config.js`), so it is A6/A7's exit criterion.

### Wave A2a — Old auth deleted; password, signup, sessions  *(Agent A)*

- **Depends on:** A1. **Blocks:** A2b, A3, A6.
- **Files you own:** `crates/server/src/auth/**`, `crates/server/src/rate_limit.rs` (new),
  `crates/server/src/sessions.rs` (new), `crates/server/src/db/sessions.rs` (new),
  `crates/server/src/db/users.rs`, `crates/server/src/repl.rs`, `crates/server/src/config.rs`,
  `crates/server/src/main.rs`, `crates/wasm/src/lib.rs` (only `compute_recovery_id` in this card),
  `crates/server/Cargo.toml`, `crates/wasm/Cargo.toml`, `crates/server/src/tests/**`.
- **Do not touch:** `web/**` (A2b), `api/**` beyond `auth` mounting (A3).
- **Task:**
  1. **A2.1** Remove `jsonwebtoken`, `Claims`, `create_token`, `verify_token`, `[auth] jwt_secret`,
     `main.rs:48`'s env `set_var`, the env reads at `auth/mod.rs:670,721`, the `CHALLENGES` map,
     `/auth/challenge`, `/auth/verify-challenge`, the `recover` endpoint,
     `recovery_id`/`recovery_code_hash` handling, and `compute_recovery_id`.
  2. **A2.2** `auth/password.rs`: derive-and-store the verifier (server-side Argon2id on top of the
     client verifier), constant-time verify, minimum-length policy, rehash-if-params-change hook.
  3. **A2.3** `POST /auth/signup` (email, password verifier, display name, wrapped bundles,
     encryption public key, invite code when the mode demands one); `one_time_tokens` row;
     verification email; `GET/POST /auth/verify`; resend with its own rate limit. Unverified accounts
     are restricted per Part 1.5.
  4. **A2.4** `db/sessions.rs` + `sessions.rs`: create (raw token returned once), verify-by-hash, list,
     revoke-one, revoke-all-others, cleanup task for expired rows. Middleware: bearer token → hash →
     session row → `AuthUser { user_id, role }` from the DB. `require_admin` (admin OR superadmin) for
     admin routes, superadmin-only for role changes.
  5. The `[email]` config block per Part 1.5, and the `[registration]` mode block (A7) — both in
     `config.rs`, which this card owns.
  6. **Drive the six inherited lints this card owns to zero** (`.dispatch/BACKEND-LINT-BASELINE.md` rows
     1-6: `auth/mod.rs:98`, `auth/mod.rs:610`, `config.rs:102`, `config.rs:136`, `repl.rs:20`,
     `repl.rs:125`). Two are `derivable_impls` — `#[derive(Default)]`, the same fix already applied to
     `GeocodeConfig` on B's branch. Rows 7-11 are in files this card does not own: leave them, name them.
- **Gate:**
  ```bash
  docker exec -w /workspace agent-repo-agent-a bash -lc 'export PATH=/usr/local/cargo/bin:$PATH; cd /workspace
  grep -rn "jsonwebtoken\|JWT_SECRET\|recovery_id\|compute_recovery_id" crates/ ; echo "^^ expect empty"
  cargo clippy --release -- -D warnings
  cargo test --workspace'
  ```
  Plus, in tests or by hand against `komun-db-a`: correct password accepted; wrong rejected; a loop
  test showing no measurable timing difference; a too-short password rejected at signup; signup →
  restricted until verified; the verify token single-use and expiring; duplicate email rejected in any
  case; startup failing loudly when `require_email_verification` is true and SMTP is unset; a revoked
  session 401s on the next request; a demoted admin loses access immediately; an expired session 401s;
  a logged-out token cannot be reused.
- **Exit criteria:** the grep is empty; `cargo clippy -p komun-server --no-deps` is **≤ 5 with every
  survivor a named row 7-11 of the lint baseline, in files this card does not own** (zero-warning is not
  reachable here — A3 and A5 own rows 7-11; the `--release -- -D warnings` zero-warning gate belongs to the
  end of Phase A); every auth assertion above passes; and `psql` shows `sessions` / `one_time_tokens` rows
  being written.
- **Honest limitation:** this sandbox has no egress, so live SMTP delivery cannot be exercised here. Prove
  the verification mail by asserting on the composed message (recipient, subject, token in the body) or on
  the queued send, and state in the report that real delivery was not tested.

### Wave A2b — Login, limits, reset, sessions UI, ed25519 removal  *(Agent A)*

- **Depends on:** A2a. **Blocks:** A7.3, B4, Phase B frontend.
- **Files you own:** `crates/server/src/auth/**`, `crates/server/src/rate_limit.rs`,
  `crates/server/src/api/admin.rs`, `crates/server/src/api/mod.rs` (auth routes only),
  `crates/wasm/src/lib.rs`, `web/src/lib/crypto.ts`, `web/src/lib/stores/auth.ts`,
  `web/src/routes/account/**`, `web/src/tests/**`, `crates/*/Cargo.toml`.
- **Task:** **A2.5** `POST /auth/login` → session + wrapped bundles, with indistinguishable responses
  for unknown email and wrong password. **A2.6** Token bucket keyed by IP and route class over signup,
  login, forgot, reset, verify-resend; global hourly registration cap as a backstop; `X-Forwarded-For`
  honoured only from `[server] trusted_proxies`. **A2.7** `POST /auth/password/forgot` (always 200,
  single-use 30-minute token, per-email limit) and `POST /auth/password/reset` (new verifier, optional
  recovery code, all sessions revoked, one audit row). **A2.8** Change password (requires the current
  password, re-wraps the x25519 secret, revokes other sessions) and `POST /auth/recovery/reissue`
  (fresh 12-word code, re-wraps the recovery copy; the code is shown **once** at signup, with a
  download option). **A2.9** `GET /auth/sessions`, `DELETE /auth/sessions/{id}`, `DELETE /auth/sessions`
  (others), admin equivalents under `/admin/users/{id}/sessions`, role promotion/demotion (superadmin
  only), `audit_events` on role change, admin revocation and password reset. **A2.10** Remove ed25519
  in the order given in Part 1.5 and collapse `encrypt_key_bundle` to a single-secret wrap, fixing the
  `bytes.slice(0, 32)` split in `recoverFromBundle`. **A2.11** Frontend auth rewrite: signup form,
  login form, verify banner, forgot/reset pages, change-password, sessions screen, recovery-code
  display at signup, automatic in-memory key unlock after login (no passphrase prompt anywhere).
- **Gate:**
  ```bash
  docker exec -w /workspace agent-repo-agent-a bash -lc 'export PATH=/usr/local/cargo/bin:$PATH; cd /workspace
  grep -rn "ed25519" crates/ web/src --include="*.rs" --include="*.toml" --include="*.ts" ; echo "^^ comments only"
  grep -rn "passphrase" web/src | grep -v test ; echo "^^ expect empty"
  wasm-pack build crates/wasm --target web
  cargo test --workspace && cargo clippy --release -- -D warnings
  cd web && npm run build && npx vitest run'
  ```
  Plus: 5 rapid logins from one IP → 429; a spoofed `X-Forwarded-For` from an untrusted peer ignored;
  forgot for an unknown email returns 200 and sends nothing; a used/expired reset token rejected; reset
  **with** the recovery code makes a pre-reset message readable and **without** it does not; every
  session gone after a reset; change-password keeps the same x25519 secret; reissuing a code
  invalidates the previous one; the recovery code is never returned by any endpoint after signup.
- **Exit criteria:** all of the above observed; `web/build/` produced; vitest green.

### Wave A3 — API flattening  *(Agent A)*

- **Depends on:** A2a (auth middleware shape); may run in parallel with A2b if `auth/**` is untouched.
- **Files you own:** `crates/server/src/api/**` (except `alliances.rs`, which B deletes),
  `crates/server/src/db/**`, `crates/server/src/api/mod.rs`.
- **Task:** **A3.1** Delete `api/communities.rs` and `db/communities.rs`; create `api/posts.rs` at
  `/api/posts` (`GET /`, `GET /{id}`, `POST /`, `PATCH /{id}`, `DELETE /{id}`, `POST /{id}/images`);
  drop `community_id` from `db/posts.rs` and replace the `serde_json` round-trip with `as_str()` (P4).
  **A3.2** `api/search.rs` drops the `JOIN communities` and the `community` param; `db/users.rs:33`'s
  `community_count` goes; `api/mod.rs` re-mounted — and in the same edit the `alliances` module and
  its route mounts come out (A5.1's hub half; see Part 7.2). **A3.3** `db/conversations.rs` and
  `api/conversations.rs` store and return `ciphertext`/`nonce` instead of `body`.
- **Gate:** `cargo build --workspace`, `cargo clippy --release -- -D warnings`, then boot the server
  and curl the feed, one post, a search, a conversation thread and `/auth/me` — confirming no
  `community`/`slug` in any body, no plaintext message column in the schema, and `/api/alliances` 404s.
- **Exit criteria:** the curls above, plus `grep -rn "communit" crates/server/src` returning only
  comments.

### Wave A6 — Frontend flatten  *(Agent A)*

- **Depends on:** A3. **Blocks:** B2's post-detail map, B4's docs.
- **Files you own:** `web/src/routes/**` (except the files B2 creates), `web/src/lib/api/**`,
  `web/src/lib/components/**` (existing ones only), `web/src/tests/**`.
- **Task:** **A6.1** Delete `routes/c/**`, `routes/community/**`, `routes/federation/**`,
  `routes/admin/communities/**`; create `routes/p/[id]/+page.svelte`; strip slug parameters from
  `+layout.svelte`, `+page.svelte`, `aid/**`, `search`, `admin`, `users/[id]`, `account`, `connect`.
  **A6.2** API client: the `communities` group → a flat `posts` group; drop community/admin-community
  and alliance calls. **A6.3** `AidCard.test.ts` loses community props.
- **Gate:** `grep -rn "slug" web/src` (empty), `npm run check` **green** — this card is where the frozen
  baseline reaches 0: it deletes `routes/c/**` and rewrites the `AidCard`/`SearchBar`/test-fixture code that
  produces all 14 errors and the single failing test — then `npm run build` and `npx vitest run` with zero
  failures.
- **Exit criteria:** the SPA builds, every route loads against a running server, and
  `.dispatch/FRONTEND-BASELINE.md` is retired: 0 errors, 0 failures. Nothing of A6 is done while `npm run
  check` still reports 12 errors in `AidCard.test.ts`.

### Wave B1 — Harden the geocode proxy  *(Agent B)*

- **Depends on:** nothing (independent of the schema). Starts immediately after Wave 0.
- **Files you own:** `crates/server/src/api/geocode.rs`, and any **new** module for the limiter/cache.
- **Task:** **A4.2** A 1 req/s limiter (Nominatim's hard limit), a result cache, and a `User-Agent`
  built from an operator-supplied `[geocode] contact` instead of the hardcoded `"Komun/0.1"`. Write
  the `config.rs` additions as hub request `.dispatch/hub-requests/B1.md` with the exact diff.
- **Gate:** `cargo build --workspace` clean **after** the hub request is applied; a test proving the
  limiter queues the second request rather than issuing it; a test that the `User-Agent` contains the
  configured contact; a bogus contact in config still boots.
- **Exit criteria:** build clean, limiter test passing, hub request applied, `config.example.toml`
  NOT touched (B4 owns it — one writer).

### Wave B2 — OSM map: Leaflet component + `/map` route  *(Agent B)*

- **Depends on:** Wave 0.5 — **DONE 2026-09-23**: `crates/wasm/pkg/` built, `web/node_modules/` installed
  with `leaflet` 1.9.4, both by the orchestrator. **Post-detail insertion waits for A6.**
- **Files you own:** `web/src/lib/components/LocationMap.svelte` (new),
  `web/src/routes/map/+page.svelte` (new).
- **Task:** **A4.3** `LocationMap.svelte` (props: lat, lon, zoom; tile URL + attribution from
  `/api/node`; attribution always rendered); a `/map` route showing public posts; a small map on the
  post detail page — **that last part only after A6 has merged**, and then in
  `web/src/routes/p/[id]/+page.svelte`. If A6 has not merged, skip it, say so in `BLOCKED:`, and leave
  the rest complete. The `[map]` config keys come from the B1 hub request. A bogus `tile_url` must
  degrade to an empty map, never a broken page.
- **Gate — measured against `.dispatch/FRONTEND-BASELINE.md`, NOT against green:** `cd web && npm run check`
  must stay at exactly 14 errors / 37 warnings (it cannot be green here: the whole baseline is the community
  UI that A6 deletes); `npm run build` green; `npx vitest run` 39 passed / 1 failed with the one known
  failure (`AidCard > shows community name and server`); plus a new vitest case asserting the attribution
  renders; plus a manual check that `/map` renders tiles with attribution visible.
- **Exit criteria:** **no diagnostic and no test failure added** beyond the frozen baseline, build green,
  attribution rendered, bogus tile URL degrades gracefully.

### Wave B3 — Federation removal (directory stays)  *(Agent B)*

- **Depends on:** A3 merged is preferred (A3.2 already rewrites `api/mod.rs`). If A3 has not merged, B
  may still delete files and **must** file the `api/mod.rs` hub request rather than editing it.
- **Files you own:** deletions: `crates/server/src/api/alliances.rs`,
  `crates/server/src/db/alliances/**`, `crates/server/src/federation/**`. Edits:
  `crates/server/src/api/node.rs`, `crates/server/src/tasks/registration.rs`,
  `crates/server/src/api/directory.rs`.
- **Task:** **A5.1** delete the three paths above; the `[federation]` config section goes via hub
  request; `/api/alliances` must 404. **A5.2** `api/node.rs` drops `communities_count` and
  `federation_enabled`; `tasks/registration.rs` advertises node identity only; `api/directory.rs`
  drops community listing/search.
- **Gate:** `cargo build --workspace` clean **after** the hub request is applied;
  `grep -rn "alliance\|federation" crates/server/src` returns only comments; boot the server and
  confirm `/api/alliances` 404s and `/api/node` carries neither removed field.
- **Exit criteria:** build clean, grep clean, 404 confirmed by curl, hub request recorded in Part 7.2.

### Wave B4 — Docs + deploy assets  *(Agent B)*

- **Depends on:** A2b, A3 and A6 merged. Writing these earlier guarantees drift.
- **Files you own:** `README.md`, `AGENTS.md`, `docs/*.md`, `deploy/nginx-komun.conf`,
  `deploy/komun.initd`, `deploy/setup.sh`, `deploy/seed.sql`, `config.example.toml`.
- **Task:** **A7.1** Rewrite the docs for the new account model, the SMTP setup, the flattened model
  and the OSM map; include the honest E2E limitation from Part 1.5 in intent. The README's "encrypted
  conversations" claim is rewritten to match what the code actually guarantees. `AGENTS.md` is written
  for a future agent reading the repo cold (code layout, build order, runes rule, crypto boundaries).
  **A7.2** Deploy assets: `nginx-komun.conf` drops the relay WebSocket proxy; `komun.initd`,
  `setup.sh`, `seed.sql` updated; `config.example.toml` gains `[email]`, `[map]`, `[geocode]`,
  `[registration]`, `[market]` and loses `[relay]`, `[federation]`, `[auth] jwt_secret`.
- **Gate:** every command documented is actually executed once (grep them out of the docs and run
  them); no doc mentions `jwt_secret`, `piggpin`, `relay` or a passphrase except as a removal note;
  the server boots with `config.example.toml` (DB URL filled in) and starts clean.
- **Exit criteria:** docs match the shipped code, the example config boots, dead references gone.

### Wave A7.3 — Bootstrap end to end  *(orchestrator, not an agent)*

Phase A's exit criterion, run against `komun-db-a` with a **fresh** database: create the DB, boot the
server, signup, verify by email (read the token out of the DB or the mail log), log in, create a post,
see it on the map, search, message someone (ciphertext in the DB), complete one match, list and revoke
a session. Then:

```bash
psql "postgres://komun:komun@komun-db-a:5432/komun_a" -c "\dt"                       # no communities/members/alliances
psql "postgres://komun:komun@komun-db-a:5432/komun_a" -c "select count(*) from messages where ciphertext is null"   # 0
grep -rn "jwt\|ed25519" crates/server/src --include="*.rs" | wc -l                   # 0
```

**Phase A is done when:** a normal web app with no `communities` table, no JWT, no passphrase prompt,
and no plaintext messages in the database. **Phase B does not start until this passes.**

---

## PART 7 — Sequencing, barriers, merge order

### 7.1 The calendar

```
Wave 0 (orchestrator, 0.1-0.6)
   |
   +--> A1 ─────────────────┐                    (Agent A, critical path)
   |                         |
   +--> B1 (geocode)  B2 (map component + /map)  <- parallel, no schema dependency
                             |
        A2a ─────────────────┤
        A2b ─────────────────┤
        A3 ──────────────────┤  barrier: A3 rewrites api/mod.rs
                             |
   B3 (federation deletion) ─┘  <- after A3 merges; deletions + hub request
        A6 ──────────────────────  barrier: A6 moves the post-detail route
                             |
        A7.3 bootstrap e2e ─── PHASE A EXIT
                             |
        B4 (docs) ───────────┤  after A6 lands, before Phase B
                             |
   ===== Phase B (Part 8) ===
```

Rules that make this safe:

1. **A1 is a hard barrier** — no other card starts before it merges, because every later card depends
   on the squashed schema and the rebuilt core models.
2. **B1 and B2 run during A1/A2a.** They touch no hub file and nothing schema-coupled, so they are
   genuinely parallel. Their hub requests are applied by the orchestrator when A is idle.
3. **B3 runs after A3 merges**, because A3.2 already removes the alliance mounts while rewriting
   `api/mod.rs`. If B3 must run earlier, it deletes files only and files the hub request; the
   orchestrator then applies the mount removal to B's branch alone and records it in 7.2.
4. **A6 is a barrier for anything that edits `web/src/routes/**`** — B2's post-detail map and B4's docs
   wait for it.
5. **One hub file, one in-flight editor.** If the orchestrator has an unapplied hub edit on one branch,
   it does not dispatch a card on the other branch that requests the same file.
6. **Never two cards in flight on the same agent** — that is how a workspace ends up half-refactored,
   and a big dispatch is the top cause of a silent, zero-output exit.

### 7.2 Merge order

1. Agent A's branch merges into `main` first, one card per commit (A1, A2a, A2b, A3, A6). The spine
   defines the world; B's leaves are judged against it.
2. Agent B's branch is rebased onto the new `main` **by the orchestrator**, from the host:
   `git -C ~/repo-agent-b rebase main`. B never rebases itself.
3. Expected conflicts, and how they resolve:
   - `api/mod.rs` (A3.2's rewrite vs B3's mount deletion): **A's rewritten file wins**; then re-verify
     B3's exit criteria (`/api/alliances` 404s, no `mod alliances`, no `mod federation`).
   - `config.rs` (`[map]`/`[geocode]` from B's requests vs A's `[email]`/`[registration]` work): union
     of both — no side loses a key.
   - `config.example.toml`: B4 owns it, so A never touches it.
   - Hub requests applied to both branches produce identical hunks — expect them to vanish on rebase.
4. After each merge: `cargo build --workspace`, `cargo clippy --release -- -D warnings`,
   `cargo test --workspace`, `cd web && npm run build && npx vitest run`, then boot + curl.
5. **No push.** Local commits and merges only, until the user explicitly approves a push (per-action).

---

## PART 8 — PHASE B (gated behind the A7.3 exit criterion)

Ownership follows the same rule as Phase A: **Agent A owns the backend and every hub file; Agent B
owns new frontend files.** No migration is needed for Phase B — the market columns and the seeded
`categories` rows already ship in `001_schema.sql`. If a genuine schema change appears, it becomes
`002_*.sql` (migrations are additive again after the A1 squash).

### B-q1 — Marketplace columns, filters, currency  *(Agent A)*

`B1.1` extend `PostRow`, `From<PostRow> for Post`, the SELECT lists and `create`; explicit parse arms
for `"listing"`/`"want"` plus a documented fallback so a newer version's kind can never silently
render as a need. New test file `crates/server/src/tests/market.rs` round-trips a listing and a want.
`B1.2` `GET /api/posts?listed=&kind=&min_price_cents=&max_price_cents=&condition=`. `B1.3` 422 when a
price or condition is attached to a non-marketplace kind. `B1.4` currency resolution: listing →
`[market] default_currency` → null. **Never invented.**
Gate: `cargo test --workspace` plus curl assertions on each filter and both 422 cases.

### B-q2 — Offers, completion, expiry parity, reviews  *(Agent A)*

`B1.5` `POST /api/conversations/{match_id}/offers` (offer/counter/decline by the responder, accept by
the post author), writing `matches.agreed_price_cents`/`currency` on accept; `respond_to_post` gains an
optional opening amount written as the first `match_offers` row. `B1.6` on `completed`, the post →
`fulfilled` with `sold_at`/`buyer_id`, and every *other* open match on that post → `withdrawn`.
`B1.7` a test pinning that marketplace posts expire on the aid-post clock. `B1.8`
`POST /api/conversations/{match_id}/review` (409 unless completed; 409 on a second review by the same
reviewer), `GET /api/users/{id}/reviews` with `{average, count}`.
Gate: the four 409/state-transition cases asserted, plus a curl walkthrough of offer → counter →
accept → complete → review.

### B-q3 — Categories API + admin CRUD  *(Agent A)*

`B1.9` `GET /api/categories?scope=` (public, active only) plus admin `POST/PATCH/DELETE
/api/admin/categories` (retire via `active = false`, never delete one in use); post list/detail LEFT
JOIN for `category_label`; the FTS trigger indexes the **label**; the rename path re-runs the FTS
update for affected posts; `db/posts.rs:72`'s `serde_json` round-trip goes.
Gate: unknown slug rejected on create; a category with posts cannot be deleted; deactivating one hides
it from the form while existing posts keep their label; searching a label finds the post; after a
rename the new label is searchable and the old one is not.

### B-q4 — Marketplace frontend  *(Agent B)*

`B2` `/market` feed with a listing/want toggle, `/market/new`, `MarketCard.svelte`, `OfferPanel.svelte`,
`DealReviewModal.svelte`, a profile ratings block, one categories store fetched at load, the aid form
filtered to `aid`/`both` and the market form to `market`/`both` (replacing the hardcoded `<option>`
list at `aid/new/+page.svelte:211`), and vitest coverage for the card and the offer panel.
Depends on B-q1..B-q3 merged. Gate: `npm run check`, `npm run build`, `npx vitest run`, and a hand
walkthrough of listing → offer → accept → complete → review in a browser.

---

## PART 9 — Orchestrator runbook

Per card:

1. Confirm the card's dependencies are merged (`git -C ~/rev log --oneline | head`).
2. Refresh `.dispatch/SPEC.md` in the target worktree if this document changed.
3. Write `.dispatch/<ID>.md` = the card's Goal/Files text + STANDING RULES.
4. Dispatch with the Part 0.3 command; watch `docker logs -f rev-broker` while it runs.
5. On report: `git -C ~/repo-agent-<x> status --short` (nothing outside the card may have moved), then
   `git -C ~/repo-agent-<x> diff` and read it. **Test files get read as suspiciously as source files**
   — a weakened assertion is not a fixed bug.
6. Re-run the card's gate yourself and capture raw output. Never accept the agent's summary as the gate.
7. `sudo chown -R computing:computing ~/repo-agent-<x>` for root-created files.
8. Apply/verify hub requests; re-run the affected build.
9. Ask the user before committing (per-action approval). Commit with the card id; never push without
   explicit approval.

**Cost control.** The broker has no allowlist and no cap, so it is enforced by dispatch discipline:

- one card in flight per agent; no speculative parallel cards on the same agent;
- prompts state the exact files, so a cheap model does not go exploring;
- `docker logs rev-broker` shows one line per model call — a small task producing hundreds of lines is
  a runaway: kill the dispatch (`docker exec <c> pkill -f claude` / `pkill -f opencode`);
- Agent B's cards are all delete-or-create; if a B card turns out to need design judgement, move it to
  Agent A rather than letting a cheap model improvise.

**Stop conditions.** Kill a dispatch and take over when: the agent edits files outside its card; it
weakens a test to make it pass; it claims a framework bug; it reports success without raw gate output;
or it starts a second, unrequested refactor.

---

## PART 10 — Verification (all phases)

```
cargo test --workspace          # compiles after A1.5; green is A2's exit criterion
cargo clippy --release          # zero warnings (user standard)
wasm-pack build crates/wasm --target web && cd web && npm run build   # orchestrator-only: the image has
                                                                      # no wasm32 target / wasm-bindgen
cargo run --bin komun-server    # then curl every endpoint touched in the wave
psql "postgres://komun:***@komun-db-a:5432/komun_a"   # verify the schema, not just the code
```

**Inherited-red gates.** Two gates in this project were red before any agent started, and neither is an
agent's fault: the workspace build (A1.2 removes the community model; A1.5 repairs it) and the frontend
`npm run check` / `npx vitest run` (14 errors + 1 failure, every one of them the community UI A6 deletes).
An inherited-red gate is measured as **"no new diagnostic, no new failure"** against a frozen list — never
as "green". The frontend list is frozen in `.dispatch/FRONTEND-BASELINE.md`; the workspace build closed with A1.5; and
the 11 inherited clippy warnings are itemised with an owner each in `.dispatch/BACKEND-LINT-BASELINE.md`.
The user's zero-warning `cargo clippy --release -- -D warnings` standard is therefore the **end of Phase A**
gate, not any single card's: each card clears its own rows. Record the number in the card's report and
compare counts, not exit codes. Note also that cargo caches lint output — a cached run prints nothing, so
every lint count must follow a `touch` of a source file.

Auth tests that must exist before A2 closes: signup → verify → login happy path; unverified account
cannot post; duplicate email rejected regardless of case; unknown-email and wrong-password responses
indistinguishable; rate limiter returns 429 and ignores a spoofed `X-Forwarded-For` from an untrusted
peer; reset token single-use and expiring; reset with the recovery code restores history and without it
does not; change-password preserves the x25519 secret; revoked and expired sessions rejected; demoted
admin loses access immediately; no endpoint ever returns key material or the recovery code after
signup; the database contains no plaintext message body.

Category tests before B-q3 closes: every seeded slug resolves; a post with an unknown slug is rejected
by the FK; a category with posts cannot be deleted (RESTRICT); deactivating one hides it from the form
while existing posts keep their label; searching a label finds the post, because the trigger indexes
the label and not the slug.

Test DB: `komun-db-a` / `komun-db-b` on `agent-net` (Part 4.0.2). The host's `127.0.0.1:5432`
Postgres is unreachable from the sandboxes and is for host-side gates only.

---

## PART 11 — Risks

| Risk | Mitigation |
|---|---|
| Squashing migrations destroys history | Acceptable only because the deployment goes fresh (A5) — recorded so the reason is not lost. |
| 684 Rust + 280 web references: a mechanical rename will miss some | Each wave greps for `communit` and `relay`; only allowlisted hits may remain at wave close. |
| Deleting the relay orphans the frontend map client | A4.3 replaces it in the same phase; `grep -ri piggpin` is a wave exit criterion. |
| Email deliverability (verification and reset land in spam) | Real From address + SPF/DKIM on the sending domain; resend rate-limited; the app never dead-ends a user on a missing email (an admin can verify manually). |
| Reset without a recovery code silently loses message history | Explicit in the reset UI, warned at signup, covered by a test; the code is offered for download at signup. |
| Users lose the recovery code because it looks optional | Shown once at signup with a download action and a plain-language consequence; reissuable while signed in. |
| Ciphertext messages are not searchable | Accepted by design; post/listing search is unaffected. |
| A malicious operator can still serve modified JS | Documented honestly in `docs/CRYPTO.md` rather than overclaimed. |
| Enum ↔ CHECK drift (the P1-P3 class) | A1.3 pins it with a test that parses the migration. |
| Nominatim/tile policies getting the deployment blocked | Rate limiter + operator contact for geocoding; configurable `tile_url`. |
| Svelte 5 runes rule | Reviewers reject `$:`, `export let`, `on:click`. |
| Renaming a category leaves the old label in `search_vector` | The label is baked in at write time; B-q3's rename re-runs the FTS update for affected posts. |
| Categories are now data, so a bad row is one `UPDATE` away | Deactivation is the retire path; the seed is validated by A1.3. |
| `cargo check` passing is not evidence | Every wave ends with a running server and curl assertions. |
| **Two agents, one tree each: a half-applied refactor inside a branch** | One card per agent at a time; agents never run git mutations; the orchestrator reviews every diff before committing. |
| **Root-owned files from the containers on the host bind mount** | `chown` after each wave; running the container as uid 1000 is the next hardening step. |
| **git in the containers fails after every container restart** | Wave 0.1 re-run per session (candidate: add it to `run-agent.sh`). |
| **Hub-file edits racing between branches** | Hub protocol (Part 5.3): only the orchestrator applies them, to both branches when safe; one in-flight hub edit at a time (7.1 rule 5). |
| **Merge conflict on `api/mod.rs` is expected, not a surprise** | Resolution rule in Part 7.2, with B3's exit criteria re-verified after the merge. |
| **No egress: a dependency the pre-warm missed blocks a card** | Wave 0.5 pre-warms both known deps; the bridge-attach opt-out is the sanctioned fix and must be disconnected and re-verified immediately after. |
| **Broker has no model allowlist or spend cap** | Dispatch discipline + `docker logs rev-broker` (Part 9); a cap is the next broker hardening step. |
| **Parallel agents on one shared test DB** | One DB per worktree, distinct names; never truncate the other agent's DB. |
| **Both agents idle the same overnight: an agent container holds context with no expiry** | Containers are disposable: a card that was interrupted is re-dispatched from its prompt file, and `git status` in the worktree shows exactly what survived. |

---

## PART 12 — Open items and confirmations

**Confirmed by the user:**
1. An unverified account can sign in but cannot post, respond, message or list items until its email
   is verified.
2. ed25519 is removed from the codebase entirely; only x25519 ECDH, ChaCha20Poly1305, Argon2 and the
   BIP39 recovery-code generator remain.
3. This document: Agent A = Claude Code/opus on the spine and all hub files; Agent B =
   opencode/deepseek-v4-flash on detached leaves (A4, A5, A7.1/A7.2, Phase B frontend); Phase B is
   gated behind Phase A's exit criterion.
4. **Deletions are the orchestrator's, always.** Claude Code's `acceptEdits` mode denies `rm`/`mv`
   ("Irreversible Local Destruction") — A1 hit exactly that wall and stalled with 19 paths still on disk.
   Agents now ask (DELETIONS REQUESTED), the orchestrator deletes, then the agent re-runs its gates.
   opencode is *able* to delete, and is under the same rule anyway so both engines behave identically.
5. **A1 gains an explicit A1.5** (minimum repair of `db/posts.rs`, `api/communities.rs`,
   `db/communities.rs`) rather than reordering A3 before A2a — A2a's `cargo test --workspace` gate needs a
   workspace that compiles.
6. **`api/node.rs`, `api/directory.rs`, `tasks/registration.rs` are B's files.** The wave cards always said
   so; the Part 5.1 list contradicted them, and that contradiction is where the 26th build error came from.

**Open — needs the user's call before Wave 0 completes:**
- **B's idle window.** As written, B has B1 + B2 and then waits for A3 to merge before B3. Either
  accept that, or give B a third detached leaf (e.g. `docs/CONVENTIONS.md` + `docs/DEVELOPMENT.md`
  drafted early against Part 1 and re-checked in B4).
- **Who runs Wave 0.5.** It is written as an orchestrator (host) step because it needs
  `--network bridge`; the alternative is a temporary bridge attach on the agent container.
- **Whether `run-agent.sh` should absorb the three pre-flight fixes** (safe.directory, `--user 1000`,
  the per-worktree DB) so a container restart does not silently re-break the workflow. Add a fourth: the
  container's `PATH` loses `/usr/local/cargo/bin` under a login shell.
- **How the wasm build survives a container restart.** `crates/wasm/pkg/` and `web/node_modules/` are
  orchestrator-built and gitignored, so they live in the worktree and survive; but no agent can rebuild
  them (`wasm32-unknown-unknown` and `wasm-bindgen` are absent from the image). Either bake both into a new
  image (needs a container recreation, cheapest while B is idle) or keep every `crates/wasm/**` change as an
  orchestrator rebuild step. A2a and A2b both edit `crates/wasm/src/lib.rs`, so this is not hypothetical.
- `[market] default_currency` ships unset, and the archived skill's push-notification code is absent
  from this clone (Part 2.1) — worth diffing against Forgejo when the homelab is back.
