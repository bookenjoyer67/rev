-- Komun demo seed data (optional). Run AFTER the migrations:
--   psql "$DATABASE_URL" -f deploy/seed.sql
--
-- The taxonomy is NOT here: migrations/001_schema.sql seeds all 23 categories
-- (15 scope=market, 6 scope=both, 2 scope=aid). This file only adds a few demo
-- accounts and posts so a fresh instance has a feed, map pins and a marketplace.
-- After the server is up, those 23 rows are ordinary data an admin can edit at
-- runtime through POST/PATCH /api/admin/categories — no release needed.
--
-- The demo users carry a placeholder password_hash ("!disabled"), so nobody can sign in as
-- them; they exist to own the sample posts. Create your own account through the signup form.

BEGIN;

INSERT INTO users (id, email, display_name, password_hash, auth_salt, role, bio) VALUES
    ('a0000000-0000-0000-0000-000000000001', 'river@example.org',  'River Martinez',
     '!disabled', decode('00000000000000000000000000000000', 'hex'), 'admin',
     'Community organizer. Free food distribution, East St. Louis.'),
    ('a0000000-0000-0000-0000-000000000002', 'jordan@example.org', 'Jordan Kim',
     '!disabled', decode('00000000000000000000000000000000', 'hex'), 'user',
     'Carpenter and tool librarian.'),
    ('a0000000-0000-0000-0000-000000000003', 'taylor@example.org', 'Taylor Chen',
     '!disabled', decode('00000000000000000000000000000000', 'hex'), 'user',
     'Transit justice organizer. Cars are optional.')
ON CONFLICT (id) DO NOTHING;

INSERT INTO posts
    (id, author_id, kind, category, title, body, location_name, location_lat, location_lon,
     urgency, status, expires_at)
VALUES
    ('b0000000-0000-0000-0000-000000000001',
     'a0000000-0000-0000-0000-000000000001', 'offer', 'food',
     'Free produce boxes', 'Every Saturday, 10am, while supplies last.',
     'East St. Louis, IL', 38.6247, -90.1509, NULL, 'active', now() + interval '14 days'),
    ('b0000000-0000-0000-0000-000000000002',
     'a0000000-0000-0000-0000-000000000002', 'offer', 'tools',
     'Tool lending library', 'Drills, saws, ladders and more. Open Wed/Fri.',
     'South St. Louis, MO', 38.5944, -90.2498, NULL, 'active', now() + interval '30 days'),
    ('b0000000-0000-0000-0000-000000000003',
     'a0000000-0000-0000-0000-000000000001', 'need', 'transport',
     'Rides to medical appointments', 'Looking for help with weekday morning rides.',
     'East St. Louis, IL', 38.6152, -90.1279, 'medium', 'active', now() + interval '7 days'),
    ('b0000000-0000-0000-0000-000000000004',
     'a0000000-0000-0000-0000-000000000003', 'resource', 'education',
     'Free bike repair workshop', 'Tools and guidance provided; bring your bike.',
     'North St. Louis County, MO', 38.7450, -90.3050, NULL, 'active', NULL)
ON CONFLICT (id) DO NOTHING;

-- Marketplace facet: one `listing` and one `want`. The five market columns exist
-- only on these two kinds (`chk_posts_market_fields`); every other kind leaves
-- them at their defaults. `price_cents` is whole cents and `currency` is an
-- ISO-4217 code, so `4500` + `USD` is $45.00.
INSERT INTO posts
    (id, author_id, kind, category, title, body, location_name, location_lat, location_lon,
     status, expires_at, market_listed, price_cents, currency, price_negotiable, item_condition)
VALUES
    ('b0000000-0000-0000-0000-000000000005',
     'a0000000-0000-0000-0000-000000000002', 'listing', 'tools',
     'Cordless drill, lightly used', '18V with two batteries and a charger. Collection only.',
     'South St. Louis, MO', 38.5944, -90.2498, 'active', now() + interval '30 days',
     true, 4500, 'USD', true, 'good'),
    ('b0000000-0000-0000-0000-000000000006',
     'a0000000-0000-0000-0000-000000000003', 'want', 'bikes-vehicles',
     'Wanted: a working commuter bike', 'Medium frame, ready to ride. Happy to pay a fair price.',
     'North St. Louis County, MO', 38.7450, -90.3050, 'active', now() + interval '21 days',
     true, 12000, 'USD', true, 'good')
ON CONFLICT (id) DO NOTHING;

COMMIT;
