-- Add missing foreign key constraints with ON DELETE CASCADE
-- Ensures referential integrity when users are deleted

ALTER TABLE posts ADD CONSTRAINT fk_posts_author FOREIGN KEY (author_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE matches ADD CONSTRAINT fk_matches_responder FOREIGN KEY (responder_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE messages ADD CONSTRAINT fk_messages_sender FOREIGN KEY (sender_id) REFERENCES users(id) ON DELETE CASCADE;

-- members already has a FK but without CASCADE — replace it
ALTER TABLE members DROP CONSTRAINT members_user_id_fkey;
ALTER TABLE members ADD CONSTRAINT fk_members_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
