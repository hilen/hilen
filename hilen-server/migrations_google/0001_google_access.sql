-- A Google sign in that is on its way through the browser. `challenge` is the
-- SHA-256 of the secret the app polls with, `state` guards the Google
-- redirect. `sealed` is set once Google has answered: the tokens, encrypted
-- for `device_key`, so this table never holds a token this server can read.
CREATE TABLE google_pending (
    challenge   TEXT PRIMARY KEY,
    state       TEXT NOT NULL UNIQUE,
    device_key  TEXT NOT NULL,
    role        TEXT NOT NULL,
    platform    TEXT NOT NULL,
    app_version TEXT NOT NULL,
    sealed      TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- One row per main account, for metrics. A linked account never gets a row.
CREATE TABLE google_users (
    subject         TEXT PRIMARY KEY,
    email           TEXT NOT NULL,
    name            TEXT NOT NULL,
    platform        TEXT NOT NULL,
    app_version     TEXT NOT NULL,
    linked_accounts INTEGER NOT NULL DEFAULT 0,
    first_seen      TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen       TIMESTAMPTZ NOT NULL DEFAULT now()
);
