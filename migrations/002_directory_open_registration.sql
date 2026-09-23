-- B5: directory entries advertise whether their server accepts open (public) peer registration.
--
-- Additive on purpose: 001_schema.sql is checksum-bookmarked in `_sqlx_migrations` (it was
-- hand-loaded), so it must not change. `open_registration` is deliberately separate from
-- `[registration] mode`, which governs user signup only.
ALTER TABLE directory_entries
    ADD COLUMN open_registration BOOLEAN NOT NULL DEFAULT true;
