-- The SQLite twin of migrations/0005_link.sql.
ALTER TABLE pending_logins ADD COLUMN link_user BLOB REFERENCES users (id) ON DELETE CASCADE;
