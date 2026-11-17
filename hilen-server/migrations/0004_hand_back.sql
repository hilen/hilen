-- How a finished login goes back to the app that started it, see
-- `auth/hand_back.rs`. `finish_hash` is the SHA-256 of the one time code the
-- app gets that way, the poll needs that code next to the verifier.
ALTER TABLE pending_logins ADD COLUMN hand_back TEXT;
ALTER TABLE pending_logins ADD COLUMN finish_hash BYTEA;
