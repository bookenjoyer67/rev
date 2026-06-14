-- Add missing indexes for common query patterns
-- Reduces sequential scans on frequently filtered columns

CREATE INDEX idx_posts_author ON posts(author_id);
CREATE INDEX idx_matches_responder ON matches(responder_id);
CREATE INDEX idx_matches_status ON matches(status);
CREATE INDEX idx_messages_sender ON messages(sender_id);
