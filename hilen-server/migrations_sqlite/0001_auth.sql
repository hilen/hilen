-- The SQLite twin of migrations/0001_auth.sql. Ids are 16 byte blobs made by
-- the server, times are unix seconds.
CREATE TABLE users (
    id         BLOB PRIMARY KEY,
    google_sub TEXT NOT NULL UNIQUE,
    email      TEXT NOT NULL,
    name       TEXT NOT NULL,
    picture    TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

-- Only the SHA-256 of a session token is kept, a copy of this table logs nobody in.
CREATE TABLE sessions (
    token_hash   BLOB PRIMARY KEY,
    user_id      BLOB NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at   INTEGER NOT NULL DEFAULT (unixepoch()),
    last_used_at INTEGER NOT NULL DEFAULT (unixepoch()),
    expires_at   INTEGER NOT NULL
);

CREATE INDEX sessions_user_id ON sessions (user_id);

-- A login that is on its way through the browser. `challenge` is the SHA-256 of
-- the secret the app polls with, `state` guards the Google redirect, `user_id`
-- is set once Google has answered.
CREATE TABLE pending_logins (
    challenge  TEXT PRIMARY KEY,
    state      TEXT NOT NULL UNIQUE,
    user_id    BLOB REFERENCES users (id) ON DELETE CASCADE,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);
