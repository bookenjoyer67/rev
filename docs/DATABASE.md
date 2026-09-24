# Database

PostgreSQL 16. All primary keys are UUIDv7 except `avatar_uploads.id` (BIGSERIAL). The
schema is `migrations/001_schema.sql` (frozen, checksum-bookmarked) plus additive migrations;
`002_directory_open_registration.sql` is the only one so far.

Queries are issued with sqlx **runtime** functions (`sqlx::query`, `sqlx::query_as`), not the
compile-time macros, so there is no `sqlx prepare` / offline-cache step.

## Tables

### Accounts

| Table | Key columns |
|---|---|
| `users` | `id`, `email` (unique, stored lowercased; `CHECK (email = lower(email))`), `email_verified_at`, `password_hash`, `auth_salt`, `display_name`, `role` (`user`/`admin`/`superadmin`), `bio`, `avatar_path`, `profile_json`, `last_seen`, plus the E2E columns: `encryption_public_key`, `encrypted_key_bundle`, `bundle_salt`, `encrypted_recovery_bundle`, `recovery_bundle_salt` |
| `sessions` | `id`, `user_id` → users (cascade), `token_hash` (unique; SHA-256 of the 256-bit token), `device_label`, `user_agent`, `ip`, `created_at`, `last_used_at`, `expires_at`, `revoked_at` |
| `one_time_tokens` | `id`, `user_id`, `kind` (`email_verify`/`password_reset`), `token_hash` (unique), `expires_at`, `used_at` |
| `audit_events` | `id`, `actor_id`, `action`, `subject_id`, `detail`, `created_at` |
| `invites` | `code` PK, `created_by`, `uses_remaining`, `expires_at` |

There is no `public_key`, `recovery_id` or `recovery_code_hash` column; those died with the
ed25519/recovery-ceremony removal.

### Taxonomy

| Table | Notes |
|---|---|
| `categories` | `slug` PK, `label`, `scope` (`aid`/`market`/`both`), `sort_order`, `active`. Seeded with 23 rows (15 market-only, 6 both, 2 aid-only) by `001_schema.sql`. `posts.category` is a FK to it with `ON DELETE RESTRICT`, so a category in use cannot be deleted — retire one with `active = false`. |

### Content

| Table | Notes |
|---|---|
| `posts` | `id`, `author_id`, `kind` (`resource`/`need`/`offer`/`listing`/`want`), `category` → categories, `title`, `body`, `location_name/lat/lon`, `urgency`, `quantity`, `status`, `visibility` (`public`/`private`), `expires_at`, `tags[]`, `contact_method`, `images[]`, verification columns, the marketplace columns (`market_listed`, `price_cents`, `currency`, `price_negotiable`, `item_condition`, `sold_at`, `buyer_id`), `search_vector tsvector`, timestamps. A CHECK keeps marketplace fields off non-marketplace kinds. |
| `matches` | `id`, `post_id`, `responder_id`, `responder_post_id`, `message`, `status` (`proposed`/`accepted`/`completed`/`withdrawn`), `agreed_price_cents`, `currency`, `created_at`, `resolved_at` |
| `messages` | `id`, `match_id`, `sender_id`, **`ciphertext BYTEA NOT NULL`**, `nonce BYTEA`, `created_at`. No plaintext body — the server cannot read messages. |
| `match_offers` | `id`, `match_id`, `actor_id`, `kind` (`offer`/`counter`/`accept`/`decline`), `amount_cents`, `currency`, `note` |
| `deal_reviews` | `id`, `match_id`, `reviewer_id`, `reviewee_id`, `rating` (1–5), `body`, `UNIQUE (match_id, reviewer_id)` |
| `endorsements` | `id`, `endorser_id`, `endorsee_id`, `note`, `UNIQUE (endorser_id, endorsee_id)` |

### Marketplace constraints (as built)

- **Price fields belong to market kinds only.** `posts` carries `market_listed`, `price_cents`,
  `currency`, `price_negotiable`, `item_condition`, `sold_at`, `buyer_id`, but
  `chk_posts_market_fields` rejects a non-`listing`/`want` row that sets `market_listed`,
  `price_cents` or `item_condition`. `chk_posts_price_cents` forbids a negative price and
  `chk_posts_currency` pins `currency` to `^[A-Z]{3}$`. The API validates the same rules first so a
  violation is a `400`, not a `500`.
- **The deal status matrix lives in code, not in SQL.** `matches.status` is pinned to
  `proposed`/`accepted`/`completed`/`withdrawn` by `chk_matches_status`, and the legal *transitions*
  (`proposed→accepted`, `proposed→withdrawn`, `accepted→completed`, `accepted→withdrawn`, plus the
  `proposed→proposed` no-op) are enforced by `check_transition` under a row lock, not by a CHECK.
  `resolved_at` is set only for the two terminal states.
- **`match_offers` is append-only.** `id`, `match_id`, `actor_id`, `kind`
  (`offer`/`counter`/`accept`/`decline`), `amount_cents`, `currency`, `note`, `created_at`. No
  update or delete statement touches the table; the list orders by `created_at ASC, id ASC`.
  `chk_match_offers_kind`, `chk_match_offers_amount` (non-negative) and `chk_match_offers_currency`
  guard the row.
- **`deal_reviews` is append-only and one-per-deal-per-reviewer.** `UNIQUE (match_id, reviewer_id)`
  is the enforcement, mapped to a `409`; `chk_deal_reviews_rating` is `BETWEEN 1 AND 5`.
- **Completing a deal sells the listing atomically.** `db::conversations::update_status` locks the
  match and, for `completed`, the post (`FOR UPDATE OF p`), refuses a listing that already has
  `sold_at` (`409`), then writes `posts.status = 'fulfilled'` and — for a `listing`/`want` — `sold_at`
  and `buyer_id` in the same transaction.

### Moderation, media, directory

| Table | Notes |
|---|---|
| `notifications` | `id`, `user_id`, `kind`, `title`, `body`, `link`, `read` |
| `reports` | `id`, `reporter_id`, `post_id`, `reason`, `status`, `admin_notes`, `resolved_by`, `resolved_at` |
| `avatar_uploads` | `id` BIGSERIAL, `user_id`, `uploaded_at` |
| `directory_entries` | `url` PK, `name`, `description`, `location_name/lat/lon`, `version`, `last_seen`, `registered_at`, and (from migration 002) `open_registration BOOLEAN NOT NULL DEFAULT true` |

## Enums and the CHECK pinning

Text columns that map to a Rust enum carry a named `CONSTRAINT chk_<table>_<column> CHECK (col
IN (...))`. `crates/core/src/tests.rs` parses those lists out of `001_schema.sql` at test time
and asserts each enum's `as_str()` value set matches exactly; adding a CHECK without a matching
enum fails that test. The enums live in `crates/core/src/models/` and are generated by
`db_enum!`. `Category` is not an enum — it is the seeded `categories` table.

## Full-text search

`posts_search_update()` (a `BEFORE INSERT OR UPDATE` trigger) builds `posts.search_vector`
with title = A, body = B, **category label** and tags = C. Because the *label* is indexed,
renaming a category requires re-running the update for affected posts. The GIN index is
`idx_posts_search`.

## Migration conventions

1. `001_schema.sql` is **frozen** (checksum-bookmarked) — never edit it.
2. Schema changes are additive files: `002_*.sql`, `003_*.sql`, … (see `docs/DEVELOPMENT.md`
   for the provisioning order).
3. No down migrations; the migrator runs pending files on startup.

## Pool

```rust
PgPoolOptions::new()
    .max_connections(config.database.max_connections)   // [database] max_connections, default 20
    .connect(&config.database.url).await?
```
