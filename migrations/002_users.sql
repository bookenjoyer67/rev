CREATE TABLE users (
    id UUID PRIMARY KEY,
    display_name TEXT NOT NULL,
    public_key BYTEA NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE posts ALTER COLUMN author_id TYPE UUID;
ALTER TABLE members ADD COLUMN user_id UUID REFERENCES users(id);
