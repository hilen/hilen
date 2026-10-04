-- The SQLite twin of migrations/0003_login_codes.sql. Times are unix seconds.
CREATE TABLE login_codes (
    code       TEXT PRIMARY KEY,
    challenge  TEXT NOT NULL,
    provider   TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);
