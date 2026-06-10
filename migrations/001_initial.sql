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

CREATE INDEX idx_posts_community ON posts(community_id);
CREATE INDEX idx_posts_kind ON posts(kind);
CREATE INDEX idx_posts_status ON posts(status);
CREATE INDEX idx_posts_category ON posts(category);
CREATE INDEX idx_members_community ON members(community_id);
CREATE INDEX idx_matches_post ON matches(post_id);
CREATE INDEX idx_messages_match ON messages(match_id);
