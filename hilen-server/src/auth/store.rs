//! Every query of the login, once per database. Postgres keeps ids as `UUID`
//! and times as `TIMESTAMPTZ` on its own clock. SQLite keeps ids as 16 byte
//! blobs the server makes and times as unix seconds. All times are the
//! database clock on both, so a server with a wrong clock cannot hand out
//! sessions that are already over.

use anyhow::{Result, anyhow};
use sqlx::types::{Uuid, uuid::Builder};

use crate::{
    Db,
    auth::{User, identity::Identity},
};

const SESSION_DAYS: i32 = 90;
const DAY_SECONDS: i32 = 86400;

/// Runs one statement on whichever database is in hand. The 2 arms have
/// different query types, so the body is written once and compiled twice.
macro_rules! on_db {
    ($db:expr, $postgres:expr, $sqlite:expr, | $sql:ident, $pool:ident | $body:expr) => {
        match $db {
            Db::Postgres($pool) => {
                let $sql = $postgres;
                $body
            }
            Db::Sqlite($pool) => {
                let $sql = $sqlite;
                $body
            }
        }
    };
}

/// A random id, version 4, for a new user. SQLite has no default that makes
/// one, so the server makes it for both databases.
fn new_id() -> Result<Uuid> {
    let mut bytes = [0; 16];
    getrandom::fill(&mut bytes).map_err(|error| anyhow!("no random bytes for an id: {error}"))?;
    Ok(Builder::from_random_bytes(bytes).into_uuid())
}

/// Forgets the logins nobody finished in time.
pub(crate) async fn prune_pending(db: &Db, minutes: i32) -> Result<(), sqlx::Error> {
    on_db!(
        db,
        "DELETE FROM pending_logins WHERE created_at < now() - make_interval(mins => $1)",
        "DELETE FROM pending_logins WHERE created_at < unixepoch() - $1 * 60",
        |sql, pool| sqlx::query(sql).bind(minutes).execute(pool).await.map(|_| ())
    )
}

/// Starts a login or starts it over. A reload of the page starts over. A
/// challenge that already has its user is left alone, or a second person
/// opening the same link could swap their account in under the app that is
/// waiting for the first. False for such a finished login.
pub(crate) async fn start_pending(db: &Db, challenge: &str, state: &str) -> Result<bool, sqlx::Error> {
    let started = on_db!(
        db,
        r"
INSERT INTO pending_logins (challenge, state) VALUES ($1, $2)
ON CONFLICT (challenge) DO UPDATE SET state = EXCLUDED.state, created_at = now()
WHERE pending_logins.user_id IS NULL",
        r"
INSERT INTO pending_logins (challenge, state) VALUES ($1, $2)
ON CONFLICT (challenge) DO UPDATE SET state = excluded.state, created_at = unixepoch()
WHERE pending_logins.user_id IS NULL",
        |sql, pool| sqlx::query(sql)
            .bind(challenge)
            .bind(state)
            .execute(pool)
            .await
            .map(|done| done.rows_affected())
    )?;
    Ok(started > 0)
}

/// A login waits for this `state` and is still in time.
pub(crate) async fn is_waiting(db: &Db, state: &str, minutes: i32) -> Result<bool, sqlx::Error> {
    let waiting: Option<(String,)> = on_db!(
        db,
        r"
SELECT challenge FROM pending_logins
WHERE state = $1 AND user_id IS NULL AND created_at > now() - make_interval(mins => $2)",
        r"
SELECT challenge FROM pending_logins
WHERE state = $1 AND user_id IS NULL AND created_at > unixepoch() - $2 * 60",
        |sql, pool| sqlx::query_as(sql).bind(state).bind(minutes).fetch_optional(pool).await
    )?;
    Ok(waiting.is_some())
}

/// Google has answered, the login of this `state` has its user now.
pub(crate) async fn finish_pending(db: &Db, state: &str, user_id: Uuid) -> Result<(), sqlx::Error> {
    let both = "UPDATE pending_logins SET user_id = $2 WHERE state = $1";
    on_db!(db, both, both, |sql, pool| sqlx::query(sql)
        .bind(state)
        .bind(user_id)
        .execute(pool)
        .await
        .map(|_| ()))
}

/// Takes a finished login out and gives its user. Taking the row out is what
/// makes the hand over happen once, two polls at the same moment cannot both
/// get a session.
pub(crate) async fn take_finished(
    db: &Db,
    challenge: &str,
    minutes: i32,
) -> Result<Option<Uuid>, sqlx::Error> {
    let finished: Option<(Uuid,)> = on_db!(
        db,
        r"
DELETE FROM pending_logins
WHERE challenge = $1 AND user_id IS NOT NULL AND created_at > now() - make_interval(mins => $2)
RETURNING user_id",
        r"
DELETE FROM pending_logins
WHERE challenge = $1 AND user_id IS NOT NULL AND created_at > unixepoch() - $2 * 60
RETURNING user_id",
        |sql, pool| sqlx::query_as(sql).bind(challenge).bind(minutes).fetch_optional(pool).await
    )?;
    Ok(finished.map(|(user_id,)| user_id))
}

/// The body of `login_user`, the same statements on either database inside
/// one transaction.
macro_rules! login_user_on {
    ($pool:expr, $identity:expr) => {{
        let identity: &Identity = $identity;
        let provider = identity.provider.name();
        let mut transaction = $pool.begin().await?;

        let known: Option<(Uuid,)> =
            sqlx::query_as("SELECT user_id FROM identities WHERE provider = $1 AND subject = $2")
                .bind(provider)
                .bind(&identity.subject)
                .fetch_optional(&mut *transaction)
                .await?;

        let user_id = if let Some((user_id,)) = known {
            sqlx::query(
                r"
UPDATE users SET email = $2, name = COALESCE($3, name), picture = COALESCE($4, picture)
WHERE id = $1",
            )
            .bind(user_id)
            .bind(&identity.email)
            .bind(&identity.name)
            .bind(&identity.picture)
            .execute(&mut *transaction)
            .await?;

            sqlx::query(
                r"
UPDATE identities SET refresh_token = COALESCE($3, refresh_token)
WHERE provider = $1 AND subject = $2",
            )
            .bind(provider)
            .bind(&identity.subject)
            .bind(&identity.refresh_token)
            .execute(&mut *transaction)
            .await?;

            user_id
        } else {
            let same_email: Option<(Uuid,)> = sqlx::query_as(
                "SELECT id FROM users WHERE lower(email) = lower($1) ORDER BY created_at LIMIT 1",
            )
            .bind(&identity.email)
            .fetch_optional(&mut *transaction)
            .await?;

            let user_id = if let Some((user_id,)) = same_email {
                user_id
            } else {
                let user_id = new_id()?;
                sqlx::query("INSERT INTO users (id, email, name, picture) VALUES ($1, $2, $3, $4)")
                    .bind(user_id)
                    .bind(&identity.email)
                    .bind(identity.name.as_ref().unwrap_or(&identity.email))
                    .bind(&identity.picture)
                    .execute(&mut *transaction)
                    .await?;
                user_id
            };

            sqlx::query(
                "INSERT INTO identities (provider, subject, user_id, refresh_token) VALUES ($1, $2, $3, $4)",
            )
            .bind(provider)
            .bind(&identity.subject)
            .bind(user_id)
            .bind(&identity.refresh_token)
            .execute(&mut *transaction)
            .await?;

            user_id
        };

        transaction.commit().await?;
        user_id
    }};
}

/// The user of a login, made on the first one. An identity seen before logs
/// into its user, and the email follows the provider, the name and the picture
/// too when the provider sent them. A new identity whose email already belongs
/// to a user joins that user, so a person with Google and Apple on one email
/// is one user. Every email here is verified by its provider. A new user with
/// no name is named by the email.
pub(crate) async fn login_user(db: &Db, identity: &Identity) -> Result<Uuid> {
    Ok(match db {
        Db::Postgres(pool) => login_user_on!(pool, identity),
        Db::Sqlite(pool) => login_user_on!(pool, identity),
    })
}

/// What Apple has to be told to forget before the user goes.
pub(crate) async fn apple_refresh_tokens(db: &Db, user_id: Uuid) -> Result<Vec<String>, sqlx::Error> {
    let both = r"
SELECT refresh_token FROM identities
WHERE user_id = $1 AND provider = 'apple' AND refresh_token IS NOT NULL";
    let tokens: Vec<(String,)> = on_db!(db, both, both, |sql, pool| sqlx::query_as(sql)
        .bind(user_id)
        .fetch_all(pool)
        .await)?;
    Ok(tokens.into_iter().map(|(token,)| token).collect())
}

/// Removes the user. The identities, the sessions and every row of an app
/// table that points at the user with a cascade go along.
pub(crate) async fn delete_user(db: &Db, user_id: Uuid) -> Result<(), sqlx::Error> {
    let both = "DELETE FROM users WHERE id = $1";
    on_db!(db, both, both, |sql, pool| sqlx::query(sql)
        .bind(user_id)
        .execute(pool)
        .await
        .map(|_| ()))
}

pub(crate) async fn create_session(db: &Db, token_hash: &[u8], user_id: Uuid) -> Result<(), sqlx::Error> {
    match db {
        Db::Postgres(pool) => {
            sqlx::query(
                r"
INSERT INTO sessions (token_hash, user_id, expires_at)
VALUES ($1, $2, now() + make_interval(days => $3))",
            )
            .bind(token_hash)
            .bind(user_id)
            .bind(SESSION_DAYS)
            .execute(pool)
            .await?;
        }
        Db::Sqlite(pool) => {
            sqlx::query(
                r"
INSERT INTO sessions (token_hash, user_id, expires_at)
VALUES ($1, $2, unixepoch() + $3)",
            )
            .bind(token_hash)
            .bind(user_id)
            .bind(SESSION_DAYS * DAY_SECONDS)
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
struct SessionUser {
    id:      Uuid,
    email:   String,
    name:    String,
    picture: Option<String>,
    /// Not used for a day or more.
    stale:   bool,
}

/// The user a token belongs to. A session in use moves its end forward, at
/// most once a day so that a busy app does not write on every request.
pub(crate) async fn session_user(db: &Db, token_hash: &[u8]) -> Result<Option<User>, sqlx::Error> {
    let found: Option<SessionUser> = on_db!(
        db,
        r"
SELECT u.id, u.email, u.name, u.picture, s.last_used_at < now() - interval '1 day' AS stale
FROM sessions s
JOIN users u ON u.id = s.user_id
WHERE s.token_hash = $1 AND s.expires_at > now()",
        r"
SELECT u.id, u.email, u.name, u.picture, s.last_used_at < unixepoch() - 86400 AS stale
FROM sessions s
JOIN users u ON u.id = s.user_id
WHERE s.token_hash = $1 AND s.expires_at > unixepoch()",
        |sql, pool| sqlx::query_as(sql).bind(token_hash).fetch_optional(pool).await
    )?;

    let Some(found) = found else {
        return Ok(None);
    };

    if found.stale {
        match db {
            Db::Postgres(pool) => {
                sqlx::query(
                    r"
UPDATE sessions SET last_used_at = now(), expires_at = now() + make_interval(days => $2)
WHERE token_hash = $1",
                )
                .bind(token_hash)
                .bind(SESSION_DAYS)
                .execute(pool)
                .await?;
            }
            Db::Sqlite(pool) => {
                sqlx::query(
                    r"
UPDATE sessions SET last_used_at = unixepoch(), expires_at = unixepoch() + $2
WHERE token_hash = $1",
                )
                .bind(token_hash)
                .bind(SESSION_DAYS * DAY_SECONDS)
                .execute(pool)
                .await?;
            }
        }
    }

    Ok(Some(User {
        id:      found.id,
        email:   found.email,
        name:    found.name,
        picture: found.picture,
    }))
}

pub(crate) async fn delete_session(db: &Db, token_hash: &[u8]) -> Result<(), sqlx::Error> {
    let both = "DELETE FROM sessions WHERE token_hash = $1";
    on_db!(db, both, both, |sql, pool| sqlx::query(sql)
        .bind(token_hash)
        .execute(pool)
        .await
        .map(|_| ()))
}
