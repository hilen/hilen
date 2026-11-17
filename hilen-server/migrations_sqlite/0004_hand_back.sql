-- The SQLite twin of migrations/0004_hand_back.sql.
ALTER TABLE pending_logins ADD COLUMN hand_back TEXT;
ALTER TABLE pending_logins ADD COLUMN finish_hash BLOB;
