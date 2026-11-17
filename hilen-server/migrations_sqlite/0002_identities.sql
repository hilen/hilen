-- no-transaction
-- The SQLite twin of migrations/0002_identities.sql. SQLite cannot drop a
-- UNIQUE column, so `users` is built again without it. That needs foreign keys
-- off, with them on the drop of the old table deletes every row that points
-- at a user, in the tables of the app too. The switch does nothing inside a
-- transaction, so this file runs outside of one and opens its own.
PRAGMA foreign_keys = OFF;

BEGIN;

CREATE TABLE identities (
    provider      TEXT NOT NULL,
    subject       TEXT NOT NULL,
    user_id       BLOB NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    -- Apple only, the token to revoke when the user deletes the account.
    refresh_token TEXT,
    created_at    INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (provider, subject)
);

CREATE INDEX identities_user_id ON identities (user_id);

INSERT INTO identities (provider, subject, user_id)
SELECT 'google', google_sub, id FROM users;

CREATE TABLE users_new (
    id         BLOB PRIMARY KEY,
    email      TEXT NOT NULL,
    name       TEXT NOT NULL,
    picture    TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

INSERT INTO users_new (id, email, name, picture, created_at)
SELECT id, email, name, picture, created_at FROM users;

DROP TABLE users;

ALTER TABLE users_new RENAME TO users;

-- A first login through a second provider finds its user by the email.
CREATE INDEX users_email ON users (lower(email));

COMMIT;

PRAGMA foreign_keys = ON;
