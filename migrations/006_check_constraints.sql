-- Add CHECK constraints on enum-like text columns
-- Database-enforces valid values, preventing invalid data from application bugs

ALTER TABLE users ADD CONSTRAINT chk_users_role CHECK (role IN ('user', 'superadmin'));
ALTER TABLE posts ADD CONSTRAINT chk_posts_kind CHECK (kind IN ('resource', 'need', 'offer'));
ALTER TABLE posts ADD CONSTRAINT chk_posts_status CHECK (status IN ('active', 'fulfilled', 'withdrawn', 'expired'));
ALTER TABLE posts ADD CONSTRAINT chk_posts_visibility CHECK (visibility IN ('local', 'federated', 'private'));
ALTER TABLE matches ADD CONSTRAINT chk_matches_status CHECK (status IN ('proposed', 'accepted', 'rejected', 'resolved', 'withdrawn'));
ALTER TABLE members ADD CONSTRAINT chk_members_role CHECK (role IN ('founder', 'maintainer', 'contributor', 'reader'));
ALTER TABLE communities ADD CONSTRAINT chk_communities_visibility CHECK (visibility IN ('local', 'federated', 'private'));
