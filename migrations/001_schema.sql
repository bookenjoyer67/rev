-- Komun fresh schema baseline (SPEC Part 1.4).
-- Replaces migrations 001-015: the server is the single point of connection, so the
-- multi-tenant and federation tables are gone. Email+password accounts with opaque DB
-- sessions, marketplace columns on posts, and the seeded category taxonomy.
-- Enum-backed text columns carry a named `chk_<table>_<column>` CHECK; crates/core's tests
-- parse this file and assert the Rust enums agree with these lists.

-- ---------------------------------------------------------------------------
-- accounts
-- ---------------------------------------------------------------------------

CREATE TABLE users (
    id UUID PRIMARY KEY,
    email TEXT NOT NULL UNIQUE CHECK (email = lower(email)),
    email_verified_at TIMESTAMPTZ,
    password_hash TEXT NOT NULL,
    auth_salt BYTEA NOT NULL,
    display_name TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'user',
    bio TEXT,
    avatar_path TEXT,
    profile_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    last_seen TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    encryption_public_key BYTEA,
    encrypted_key_bundle BYTEA,
    bundle_salt BYTEA,
    encrypted_recovery_bundle BYTEA,
    recovery_bundle_salt BYTEA,
    CONSTRAINT chk_users_role CHECK (role IN ('user', 'admin', 'superadmin'))
);

CREATE UNIQUE INDEX idx_users_email_lower ON users (lower(email));

CREATE TABLE sessions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash BYTEA NOT NULL UNIQUE,
    device_label TEXT,
    user_agent TEXT,
    ip TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ
);

CREATE INDEX idx_sessions_user ON sessions(user_id);
CREATE INDEX idx_sessions_expires ON sessions(expires_at) WHERE revoked_at IS NULL;

CREATE TABLE one_time_tokens (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    token_hash BYTEA NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT chk_one_time_tokens_kind CHECK (kind IN ('email_verify', 'password_reset'))
);

CREATE INDEX idx_one_time_tokens_user ON one_time_tokens(user_id);

CREATE TABLE audit_events (
    id UUID PRIMARY KEY,
    actor_id UUID REFERENCES users(id) ON DELETE SET NULL,
    action TEXT NOT NULL,
    subject_id UUID,
    detail JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_audit_events_actor ON audit_events(actor_id);
CREATE INDEX idx_audit_events_created ON audit_events(created_at DESC);

CREATE TABLE invites (
    code TEXT PRIMARY KEY,
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    uses_remaining INT,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ---------------------------------------------------------------------------
-- categories (seed data: the taxonomy is rows, not a Rust enum)
-- ---------------------------------------------------------------------------

CREATE TABLE categories (
    slug TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    scope TEXT NOT NULL,
    sort_order INT NOT NULL DEFAULT 0,
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT chk_categories_scope CHECK (scope IN ('aid', 'market', 'both'))
);

CREATE INDEX idx_categories_scope ON categories(scope) WHERE active;

INSERT INTO categories (slug, label, scope, sort_order) VALUES
    ('electronics',        'Electronics & Computers', 'market',  10),
    ('furniture',          'Furniture & Home',        'market',  20),
    ('appliances',         'Appliances',              'market',  30),
    ('tools',              'Tools & Equipment',       'market',  40),
    ('clothing',           'Clothing & Accessories',  'market',  50),
    ('bikes-vehicles',     'Bikes & Vehicles',        'market',  60),
    ('books-media',        'Books & Media',           'market',  70),
    ('garden-outdoors',    'Garden & Outdoors',       'market',  80),
    ('sports',             'Sports & Fitness',        'market',  90),
    ('toys-games',         'Toys & Games',            'market', 100),
    ('baby-kids',          'Baby & Kids',             'market', 110),
    ('building-materials', 'Building Materials',      'market', 120),
    ('art-craft',          'Art & Craft',             'market', 130),
    ('household',          'Household Goods',         'market', 140),
    ('free',               'Free / Give Away',        'market', 150),
    ('services',           'Services & Labor',        'both',   160),
    ('food',               'Food',                    'both',   170),
    ('health',             'Health',                  'both',   180),
    ('education',          'Education',               'both',   190),
    ('legal',              'Legal',                   'both',   200),
    ('other',              'Other',                   'both',   210),
    ('shelter',            'Shelter',                 'aid',    220),
    ('transport',          'Transport',               'aid',    230);

-- ---------------------------------------------------------------------------
-- posts (aid + marketplace, one table)
-- ---------------------------------------------------------------------------

CREATE TABLE posts (
    id UUID PRIMARY KEY,
    author_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    category TEXT NOT NULL REFERENCES categories(slug) ON DELETE RESTRICT,
    title TEXT NOT NULL,
    body TEXT,
    location_name TEXT,
    location_lat DOUBLE PRECISION,
    location_lon DOUBLE PRECISION,
    urgency TEXT,
    quantity INT,
    status TEXT NOT NULL DEFAULT 'active',
    visibility TEXT NOT NULL DEFAULT 'public',
    expires_at TIMESTAMPTZ,
    tags TEXT[] NOT NULL DEFAULT '{}',
    contact_method TEXT,
    images TEXT[] NOT NULL DEFAULT '{}',
    verified_by UUID REFERENCES users(id) ON DELETE SET NULL,
    verified_at TIMESTAMPTZ,
    market_listed BOOLEAN NOT NULL DEFAULT false,
    price_cents BIGINT,
    currency TEXT,
    price_negotiable BOOLEAN NOT NULL DEFAULT false,
    item_condition TEXT,
    sold_at TIMESTAMPTZ,
    buyer_id UUID REFERENCES users(id) ON DELETE SET NULL,
    search_vector tsvector,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT chk_posts_kind CHECK (kind IN ('resource', 'need', 'offer', 'listing', 'want')),
    CONSTRAINT chk_posts_urgency CHECK (urgency IN ('critical', 'high', 'medium', 'low')),
    CONSTRAINT chk_posts_status CHECK (status IN ('active', 'matched', 'fulfilled', 'expired', 'withdrawn', 'hidden', 'flagged')),
    CONSTRAINT chk_posts_visibility CHECK (visibility IN ('public', 'private')),
    CONSTRAINT chk_posts_item_condition CHECK (item_condition IN ('new', 'like_new', 'good', 'fair', 'poor', 'for_parts')),
    CONSTRAINT chk_posts_price_cents CHECK (price_cents IS NULL OR price_cents >= 0),
    CONSTRAINT chk_posts_currency CHECK (currency IS NULL OR currency ~ '^[A-Z]{3}$'),
    -- marketplace fields belong to marketplace kinds only
    CONSTRAINT chk_posts_market_fields CHECK (
        kind IN ('listing', 'want')
        OR (market_listed = false AND price_cents IS NULL AND item_condition IS NULL)
    )
);

CREATE INDEX idx_posts_author ON posts(author_id);
CREATE INDEX idx_posts_kind ON posts(kind);
CREATE INDEX idx_posts_status ON posts(status);
CREATE INDEX idx_posts_category ON posts(category);
CREATE INDEX idx_posts_expires ON posts(expires_at) WHERE expires_at IS NOT NULL AND status = 'active';
CREATE INDEX idx_posts_market ON posts(market_listed) WHERE market_listed;
CREATE INDEX idx_posts_search ON posts USING GIN(search_vector);

-- Full-text search: title (A), body (B), category LABEL and tags (C).
-- The label, not the slug, is indexed so search matches the word a human types;
-- renaming a category therefore has to re-run this update for affected posts.
CREATE OR REPLACE FUNCTION posts_search_update() RETURNS TRIGGER AS $$
DECLARE
    category_label TEXT;
BEGIN
    SELECT label INTO category_label FROM categories WHERE slug = NEW.category;
    NEW.search_vector :=
        setweight(to_tsvector('english', COALESCE(NEW.title, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(NEW.body, '')), 'B') ||
        setweight(to_tsvector('english', COALESCE(category_label, '')), 'C') ||
        setweight(to_tsvector('english', COALESCE(array_to_string(NEW.tags, ' '), '')), 'C');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_posts_search
    BEFORE INSERT OR UPDATE ON posts
    FOR EACH ROW EXECUTE FUNCTION posts_search_update();

-- ---------------------------------------------------------------------------
-- matches, messages, offers, reviews
-- ---------------------------------------------------------------------------

CREATE TABLE matches (
    id UUID PRIMARY KEY,
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    responder_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    responder_post_id UUID REFERENCES posts(id) ON DELETE SET NULL,
    message TEXT,
    status TEXT NOT NULL DEFAULT 'proposed',
    agreed_price_cents BIGINT,
    currency TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolved_at TIMESTAMPTZ,
    CONSTRAINT chk_matches_status CHECK (status IN ('proposed', 'accepted', 'completed', 'withdrawn')),
    CONSTRAINT chk_matches_agreed_price CHECK (agreed_price_cents IS NULL OR agreed_price_cents >= 0),
    CONSTRAINT chk_matches_currency CHECK (currency IS NULL OR currency ~ '^[A-Z]{3}$')
);

CREATE INDEX idx_matches_post ON matches(post_id);
CREATE INDEX idx_matches_responder ON matches(responder_id);
CREATE INDEX idx_matches_status ON matches(status);

-- Message content is never readable by the server: ciphertext only, no plaintext body.
CREATE TABLE messages (
    id UUID PRIMARY KEY,
    match_id UUID NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    sender_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    ciphertext BYTEA NOT NULL,
    nonce BYTEA,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_messages_match ON messages(match_id);
CREATE INDEX idx_messages_sender ON messages(sender_id);

CREATE TABLE match_offers (
    id UUID PRIMARY KEY,
    match_id UUID NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    actor_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    amount_cents BIGINT,
    currency TEXT,
    note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT chk_match_offers_kind CHECK (kind IN ('offer', 'counter', 'accept', 'decline')),
    CONSTRAINT chk_match_offers_amount CHECK (amount_cents IS NULL OR amount_cents >= 0),
    CONSTRAINT chk_match_offers_currency CHECK (currency IS NULL OR currency ~ '^[A-Z]{3}$')
);

CREATE INDEX idx_match_offers_match ON match_offers(match_id);

CREATE TABLE deal_reviews (
    id UUID PRIMARY KEY,
    match_id UUID NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    reviewer_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reviewee_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    rating SMALLINT NOT NULL,
    body TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT chk_deal_reviews_rating CHECK (rating BETWEEN 1 AND 5),
    UNIQUE (match_id, reviewer_id)
);

CREATE INDEX idx_deal_reviews_reviewee ON deal_reviews(reviewee_id);

CREATE TABLE endorsements (
    id UUID PRIMARY KEY,
    endorser_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    endorsee_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (endorser_id, endorsee_id)
);

CREATE INDEX idx_endorsements_endorsee ON endorsements(endorsee_id);
CREATE INDEX idx_endorsements_endorser ON endorsements(endorser_id);

-- ---------------------------------------------------------------------------
-- notifications, moderation, media, directory
-- ---------------------------------------------------------------------------

CREATE TABLE notifications (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT,
    link TEXT,
    read BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_notifications_user ON notifications(user_id);
CREATE INDEX idx_notifications_unread ON notifications(user_id) WHERE read = false;

CREATE TABLE reports (
    id UUID PRIMARY KEY,
    reporter_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    reason TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'pending',
    admin_notes TEXT,
    resolved_by UUID REFERENCES users(id) ON DELETE SET NULL,
    resolved_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_reports_status ON reports(status);
CREATE INDEX idx_reports_post_id ON reports(post_id);

CREATE TABLE avatar_uploads (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    uploaded_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_avatar_uploads_user_time ON avatar_uploads(user_id, uploaded_at DESC);

CREATE TABLE directory_entries (
    url TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    location_name TEXT,
    location_lat DOUBLE PRECISION,
    location_lon DOUBLE PRECISION,
    version TEXT,
    last_seen TIMESTAMPTZ NOT NULL DEFAULT now(),
    registered_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_directory_location ON directory_entries(location_lat, location_lon)
    WHERE location_lat IS NOT NULL AND location_lon IS NOT NULL;
