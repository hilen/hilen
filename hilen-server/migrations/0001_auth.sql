CREATE TABLE users (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    google_sub TEXT NOT NULL UNIQUE,
    email      TEXT NOT NULL,
    name       TEXT NOT NULL,
    picture    TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Only the SHA-256 of a session token is kept, a copy of this table logs nobody in.
CREATE TABLE sessions (
    token_hash   BYTEA PRIMARY KEY,
    user_id      UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at   TIMESTAMPTZ NOT NULL
);

CREATE INDEX sessions_user_id ON sessions (user_id);

-- A login that is on its way through the browser. `challenge` is the SHA-256 of
-- the secret the app polls with, `state` guards the Google redirect, `user_id`
-- is set once Google has answered.
CREATE TABLE pending_logins (
    challenge  TEXT PRIMARY KEY,
    state      TEXT NOT NULL UNIQUE,
    user_id    UUID REFERENCES users (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
