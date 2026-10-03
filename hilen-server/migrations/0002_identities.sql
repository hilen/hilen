-- A user can log in through more than 1 provider. Who they are at a provider
-- moves out of `users` into a table of its own.
CREATE TABLE identities (
    provider      TEXT NOT NULL,
    subject       TEXT NOT NULL,
    user_id       UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    -- Apple only, the token to revoke when the user deletes the account.
    refresh_token TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (provider, subject)
);

CREATE INDEX identities_user_id ON identities (user_id);

INSERT INTO identities (provider, subject, user_id)
SELECT 'google', google_sub, id FROM users;

ALTER TABLE users DROP COLUMN google_sub;

-- A first login through a second provider finds its user by the email.
CREATE INDEX users_email ON users (lower(email));
