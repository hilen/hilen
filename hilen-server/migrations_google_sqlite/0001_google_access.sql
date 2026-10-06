-- The SQLite twin of migrations_google/0001_google_access.sql. Times are
-- unix seconds.
CREATE TABLE google_pending (
    challenge   TEXT PRIMARY KEY,
    state       TEXT NOT NULL UNIQUE,
    device_key  TEXT NOT NULL,
    role        TEXT NOT NULL,
    platform    TEXT NOT NULL,
    app_version TEXT NOT NULL,
    sealed      TEXT,
    created_at  INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE google_users (
    subject         TEXT PRIMARY KEY,
    email           TEXT NOT NULL,
    name            TEXT NOT NULL,
    platform        TEXT NOT NULL,
    app_version     TEXT NOT NULL,
    linked_accounts INTEGER NOT NULL DEFAULT 0,
    first_seen      INTEGER NOT NULL DEFAULT (unixepoch()),
    last_seen       INTEGER NOT NULL DEFAULT (unixepoch())
);
