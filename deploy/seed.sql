-- Komun seed data: St. Louis mutual aid communities, users, posts, endorsements
-- Run AFTER all migrations: psql -d komun -f deploy/seed.sql

BEGIN;

-- Users (Ed25519 public keys are real keypairs generated for testing — DO NOT use in production)
INSERT INTO users (id, display_name, public_key, role, created_at) VALUES
    ('a0000000-0000-0000-0000-000000000001', 'River Martinez', decode('d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a', 'hex'), 'superadmin', now()),
    ('a0000000-0000-0000-0000-000000000002', 'Jordan Kim', decode('8c5c1b6e6c4c0c3f0f5b3a1a4e4d0c5a6f7e8d9c0b1a2f3e4d5c6b7a8f9e0d', 'hex'), 'user', now()),
    ('a0000000-0000-0000-0000-000000000003', 'Taylor Chen', decode('a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1', 'hex'), 'user', now()),
    ('a0000000-0000-0000-0000-000000000004', 'Sam Williams', decode('b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2', 'hex'), 'user', now()),
    ('a0000000-0000-0000-0000-000000000005', 'Morgan Patel', decode('c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3', 'hex'), 'user', now()),
    ('a0000000-0000-0000-0000-000000000006', 'Casey Johnson', decode('d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4', 'hex'), 'superadmin', now());

UPDATE users SET bio = 'Community organizer. I run the free food distribution in East St. Louis.' WHERE id = 'a0000000-0000-0000-0000-000000000001';
UPDATE users SET bio = 'Carpenter, tool librarian, mutual aid enthusiast.' WHERE id = 'a0000000-0000-0000-0000-000000000002';
UPDATE users SET bio = 'Transit justice organizer. Cars are optional.' WHERE id = 'a0000000-0000-0000-0000-000000000003';

-- Communities
INSERT INTO communities (id, slug, name, description, location_name, location_lat, location_lon) VALUES
    ('c0000000-0000-0000-0000-000000000001', 'eastside-free-food', 'Eastside Free Food',
     'Free food distribution for East St. Louis and surrounding neighborhoods. Every Saturday at 10am.',
     'East St. Louis, IL', 38.6152, -90.1279),
    ('c0000000-0000-0000-0000-000000000002', 'south-city-tool-share', 'South City Tool Share',
     'Tool lending library for South St. Louis. Borrow what you need — drills, saws, ladders, and more. Open Wed/Fri.',
     'South St. Louis, MO', 38.5944, -90.2498),
    ('c0000000-0000-0000-0000-000000000003', 'north-county-transit', 'North County Transit Collective',
     'Ride shares, bike repair, and transit advocacy for North St. Louis County.',
     'North St. Louis County, MO', 38.7450, -90.3050);

-- Members (add users to communities)
INSERT INTO members (id, community_id, user_id, display_name, public_key, role) VALUES
    (gen_random_uuid(), 'c0000000-0000-0000-0000-000000000001', 'a0000000-0000-0000-0000-000000000001', 'River Martinez', decode('d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a', 'hex'), 'admin'),
    (gen_random_uuid(), 'c0000000-0000-0000-0000-000000000001', 'a0000000-0000-0000-0000-000000000002', 'Jordan Kim', decode('8c5c1b6e6c4c0c3f0f5b3a1a4e4d0c5a6f7e8d9c0b1a2f3e4d5c6b7a8f9e0d', 'hex'), 'member'),
    (gen_random_uuid(), 'c0000000-0000-0000-0000-000000000001', 'a0000000-0000-0000-0000-000000000004', 'Sam Williams', decode('b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2', 'hex'), 'member'),
    (gen_random_uuid(), 'c0000000-0000-0000-0000-000000000002', 'a0000000-0000-0000-0000-000000000002', 'Jordan Kim', decode('8c5c1b6e6c4c0c3f0f5b3a1a4e4d0c5a6f7e8d9c0b1a2f3e4d5c6b7a8f9e0d', 'hex'), 'admin'),
    (gen_random_uuid(), 'c0000000-0000-0000-0000-000000000002', 'a0000000-0000-0000-0000-000000000005', 'Morgan Patel', decode('c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3', 'hex'), 'member'),
    (gen_random_uuid(), 'c0000000-0000-0000-0000-000000000003', 'a0000000-0000-0000-0000-000000000003', 'Taylor Chen', decode('a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1', 'hex'), 'admin'),
    (gen_random_uuid(), 'c0000000-0000-0000-0000-000000000003', 'a0000000-0000-0000-0000-000000000006', 'Casey Johnson', decode('d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4', 'hex'), 'member');

-- Posts (needs, offers, resources)
INSERT INTO posts (id, community_id, author_id, kind, category, title, body, location_name, urgency, status, tags) VALUES
    -- Eastside Free Food posts
    (gen_random_uuid(), 'c0000000-0000-0000-0000-000000000001', 'a0000000-0000-0000-0000-000000000001', 'need', 'food',
     'Volunteers needed for Saturday distribution',
     'Looking for 4-5 volunteers to help pack and distribute food boxes this Saturday from 9am-12pm. No experience needed — we will train you on site. Heavy lifting helpful but not required.',
     'East St. Louis, IL', 'high', 'active', '{volunteer,food,weekend}'),
    (gen_random_uuid(), 'c0000000-0000-0000-0000-000000000001', 'a0000000-0000-0000-0000-000000000001', 'offer', 'food',
     'Free hot meals — Saturday lunch',
     'Hot vegetarian meals available this Saturday 12-2pm at the community center. Rice and beans, vegetable stew, fresh bread. No questions asked, everyone welcome.',
     'East St. Louis, IL', 'critical', 'active', '{hot-meal,vegetarian,weekly}'),
    (gen_random_uuid(), 'c0000000-0000-0000-0000-000000000001', 'a0000000-0000-0000-0000-000000000002', 'offer', 'food',
     'Extra produce from garden — free',
     'My garden produced way more tomatoes, peppers, and squash than I can use. Free to anyone who can pick up in East St. Louis. Bring your own bags.',
     'East St. Louis, IL', 'medium', 'active', '{produce,garden,fresh}'),
    (gen_random_uuid(), 'c0000000-0000-0000-0000-000000000001', 'a0000000-0000-0000-0000-000000000004', 'need', 'food',
     'Need: rice and dry beans for community pantry',
     'Our community pantry is running low on dry rice and beans (our most requested items). Can anyone donate bulk quantities? We serve about 50 families per week.',
     NULL, 'high', 'active', '{pantry,staples,bulk}'),
    (gen_random_uuid(), 'c0000000-0000-0000-0000-000000000001', 'a0000000-0000-0000-0000-000000000001', 'resource', 'food',
     'Community fridge locations map',
     'Updated map of all community fridges in the metro area. Currently 6 active fridges maintained by different groups. DM for exact addresses.',
     'St. Louis Metro', NULL, 'active', '{fridge,map,resource}'),
    -- South City Tool Share posts
    (gen_random_uuid(), 'c0000000-0000-0000-0000-000000000002', 'a0000000-0000-0000-0000-000000000002', 'offer', 'tools',
     'Power drill + bit set available to borrow',
     'DeWalt 20V cordless drill with full bit set (wood, metal, masonry). Available for 3-day loan. Must return clean and charged. First come first served.',
     'South St. Louis, MO', 'medium', 'active', '{power-tool,drill,loan}'),
    (gen_random_uuid(), 'c0000000-0000-0000-0000-000000000002', 'a0000000-0000-0000-0000-000000000005', 'need', 'tools',
     'Need: pressure washer for community garden cleanup',
     'Our community garden fence and walkways need a good cleaning before our spring planting day. Would anyone lend a pressure washer for a weekend?',
     'South St. Louis, MO', 'low', 'active', '{garden,cleanup,weekend}'),
    (gen_random_uuid(), 'c0000000-0000-0000-0000-000000000002', 'a0000000-0000-0000-0000-000000000002', 'resource', 'tools',
     'Tool library catalog online now',
     'We have uploaded our full inventory. 142 tools available: woodworking, plumbing, electrical, automotive, gardening. Check the catalog before you come.',
     'South St. Louis, MO', NULL, 'active', '{catalog,inventory,library}'),
    -- North County Transit posts
    (gen_random_uuid(), 'c0000000-0000-0000-0000-000000000003', 'a0000000-0000-0000-0000-000000000003', 'offer', 'transport',
     'Ride share to grocery stores — Tuesdays',
     'I drive a van to Aldi and Schnucks every Tuesday morning. Can take up to 4 people. Priority for seniors and people without car access. North County only.',
     'North St. Louis County, MO', 'high', 'active', '{ride-share,grocery,weekly}'),
    (gen_random_uuid(), 'c0000000-0000-0000-0000-000000000003', 'a0000000-0000-0000-0000-000000000006', 'need', 'transport',
     'Need: bike tubes and patch kits',
     'Fixing up donated bikes for the community bike library. Need 26" and 700c inner tubes, patch kits, tire levers. Any bike shop castoffs welcome.',
     'North St. Louis County, MO', 'medium', 'active', '{bike,repair,donation}'),
    (gen_random_uuid(), 'c0000000-0000-0000-0000-000000000003', 'a0000000-0000-0000-0000-000000000003', 'resource', 'transport',
     'Bike repair workshop — every Thursday 6pm',
     'Free bike repair workshop at the community center. We have tools and stands. Bring your bike or come help fix others. All skill levels welcome.',
     'North St. Louis County, MO', NULL, 'active', '{workshop,bike,weekly}'),
    -- Cross-community posts
    (gen_random_uuid(), 'c0000000-0000-0000-0000-000000000001', 'a0000000-0000-0000-0000-000000000001', 'need', 'housing',
     'Emergency housing for family displaced by fire',
     'A family of 4 in East St. Louis lost their apartment to a fire last night. They need temporary housing for 1-2 weeks while insurance processes. Can anyone host or know of available space?',
     'East St. Louis, IL', 'critical', 'active', '{emergency,housing,family}'),
    (gen_random_uuid(), 'c0000000-0000-0000-0000-000000000002', 'a0000000-0000-0000-0000-000000000005', 'offer', 'other',
     'Free winter coats — all sizes',
     'Clearing out a storage unit of winter coats collected last season. Sizes infant through adult 3XL. Good condition. Pick up in South City this weekend.',
     'South St. Louis, MO', 'high', 'active', '{clothing,winter,donation}');

-- Endorsements (web of trust)
INSERT INTO endorsements (id, endorser_id, endorsee_id, note, created_at) VALUES
    (gen_random_uuid(), 'a0000000-0000-0000-0000-000000000002', 'a0000000-0000-0000-0000-000000000001',
     'River has been running the Eastside food distribution for 3 years. Reliable and trustworthy.', now() - interval '90 days'),
    (gen_random_uuid(), 'a0000000-0000-0000-0000-000000000004', 'a0000000-0000-0000-0000-000000000001',
     'River connected my family with food when we needed it most.', now() - interval '60 days'),
    (gen_random_uuid(), 'a0000000-0000-0000-0000-000000000001', 'a0000000-0000-0000-0000-000000000002',
     'Jordan built the shelves for our pantry — skilled carpenter and generous with their time.', now() - interval '45 days'),
    (gen_random_uuid(), 'a0000000-0000-0000-0000-000000000005', 'a0000000-0000-0000-0000-000000000002',
     'Jordan taught me how to use a table saw safely. Patient teacher.', now() - interval '30 days'),
    (gen_random_uuid(), 'a0000000-0000-0000-0000-000000000006', 'a0000000-0000-0000-0000-000000000003',
     'Taylor coordinates the bike library — amazing organizer.', now() - interval '20 days'),
    (gen_random_uuid(), 'a0000000-0000-0000-0000-000000000001', 'a0000000-0000-0000-0000-000000000003',
     'Taylor has been a transit justice advocate for years. Deep knowledge of the bus system.', now() - interval '10 days'),
    (gen_random_uuid(), 'a0000000-0000-0000-0000-000000000003', 'a0000000-0000-0000-0000-000000000006',
     'Casey helps with bike repairs every Thursday — consistent and kind.', now() - interval '5 days'),
    (gen_random_uuid(), 'a0000000-0000-0000-0000-000000000004', 'a0000000-0000-0000-0000-000000000005',
     'Morgan is always the first to volunteer when someone needs help moving.', now() - interval '3 days');

COMMIT;
