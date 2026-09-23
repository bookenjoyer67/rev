-- Komun demo seed data (optional). Run AFTER the migrations:
--   psql "$DATABASE_URL" -f deploy/seed.sql
--
-- The 23 categories are already seeded by migrations/001_schema.sql, so this only adds a few
-- demo accounts and posts so a fresh instance has a feed and map pins.
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

COMMIT;
