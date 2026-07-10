-- Fix: the codebase uses 'admin' and 'member', not the constrained set
ALTER TABLE members DROP CONSTRAINT IF EXISTS chk_members_role;
ALTER TABLE members ADD CONSTRAINT chk_members_role CHECK (role IN ('admin', 'member', 'founder', 'maintainer', 'contributor', 'reader'));
