CREATE TABLE users (
    id UUID PRIMARY KEY,
    display_name TEXT NOT NULL,
    public_key BYTEA NOT NULL UNIQUE,
    encryption_public_key BYTEA,
    encrypted_key_bundle BYTEA,
    bundle_salt BYTEA,
    recovery_id BYTEA UNIQUE,
    role TEXT NOT NULL DEFAULT 'user',
    last_seen TIMESTAMPTZ DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE communities (
    id UUID PRIMARY KEY,
    slug TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    location_name TEXT,
    location_lat DOUBLE PRECISION,
    location_lon DOUBLE PRECISION,
    visibility TEXT NOT NULL DEFAULT 'federated',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE members (
    id UUID PRIMARY KEY,
    community_id UUID NOT NULL REFERENCES communities(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id),
    display_name TEXT NOT NULL,
    public_key BYTEA NOT NULL,
    role TEXT NOT NULL DEFAULT 'member',
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(community_id, public_key)
);

CREATE TABLE posts (
    id UUID PRIMARY KEY,
    community_id UUID NOT NULL REFERENCES communities(id) ON DELETE CASCADE,
    author_id UUID NOT NULL,
    kind TEXT NOT NULL,
    category TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT,
    location_name TEXT,
    location_lat DOUBLE PRECISION,
    location_lon DOUBLE PRECISION,
    urgency TEXT,
    quantity INT,
    status TEXT NOT NULL DEFAULT 'active',
    visibility TEXT NOT NULL DEFAULT 'federated',
    expires_at TIMESTAMPTZ,
    tags TEXT[] DEFAULT '{}',
    contact_method TEXT,
    verified_by UUID REFERENCES members(id),
    verified_at TIMESTAMPTZ,
    federated_id TEXT,
    origin_node TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE matches (
    id UUID PRIMARY KEY,
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    responder_id UUID NOT NULL,
    responder_post_id UUID REFERENCES posts(id),
    message TEXT,
    status TEXT NOT NULL DEFAULT 'proposed',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolved_at TIMESTAMPTZ
);

CREATE TABLE messages (
    id UUID PRIMARY KEY,
    match_id UUID NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    sender_id UUID NOT NULL,
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE invites (
    code TEXT PRIMARY KEY,
    community_id UUID NOT NULL REFERENCES communities(id) ON DELETE CASCADE,
    created_by UUID NOT NULL,
    uses_remaining INT,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE alliances (
    id UUID PRIMARY KEY,
    remote_domain TEXT NOT NULL,
    remote_name TEXT,
    remote_public_key BYTEA,
    status TEXT NOT NULL DEFAULT 'pending',
    initiated_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE directory_entries (
    url TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    location_name TEXT,
    location_lat DOUBLE PRECISION,
    location_lon DOUBLE PRECISION,
    communities_count BIGINT DEFAULT 0,
    version TEXT,
    last_seen TIMESTAMPTZ NOT NULL DEFAULT now(),
    registered_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE notifications (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT,
    link TEXT,
    read BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_posts_community ON posts(community_id);
CREATE INDEX idx_posts_kind ON posts(kind);
CREATE INDEX idx_posts_status ON posts(status);
CREATE INDEX idx_posts_category ON posts(category);
CREATE INDEX idx_posts_expires ON posts(expires_at) WHERE expires_at IS NOT NULL AND status = 'active';
CREATE INDEX idx_members_community ON members(community_id);
CREATE INDEX idx_matches_post ON matches(post_id);
CREATE INDEX idx_messages_match ON messages(match_id);
CREATE INDEX idx_directory_location ON directory_entries(location_lat, location_lon)
    WHERE location_lat IS NOT NULL AND location_lon IS NOT NULL;
CREATE INDEX idx_notifications_user ON notifications(user_id);
CREATE INDEX idx_notifications_unread ON notifications(user_id) WHERE read = false;
CREATE INDEX idx_users_recovery ON users(recovery_id) WHERE recovery_id IS NOT NULL;
