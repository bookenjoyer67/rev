-- Make invites.created_by nullable and add FK for ON DELETE SET NULL
-- When a user is deleted after 90 days of inactivity, their invites survive
ALTER TABLE invites ALTER COLUMN created_by DROP NOT NULL;
ALTER TABLE invites ADD CONSTRAINT fk_invites_created_by FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE SET NULL;
