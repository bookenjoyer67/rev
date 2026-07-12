CREATE TABLE endorsements (
    id UUID PRIMARY KEY,
    endorser_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    endorsee_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(endorser_id, endorsee_id)
);

CREATE INDEX idx_endorsements_endorsee ON endorsements(endorsee_id);
CREATE INDEX idx_endorsements_endorser ON endorsements(endorser_id);

ALTER TABLE users ADD COLUMN linked_servers TEXT[] DEFAULT '{}';
