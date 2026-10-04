-- A short code a person types on a phone to finish the login of a device with
-- no keyboard, like a TV. It leads to the login page the device would have
-- opened itself, `challenge` is the one the device polls with.
CREATE TABLE login_codes (
    code       TEXT PRIMARY KEY,
    challenge  TEXT NOT NULL,
    provider   TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
