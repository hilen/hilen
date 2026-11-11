//! Every query of the login, once per database. Postgres keeps ids as `UUID`
//! and times as `TIMESTAMPTZ` on its own clock. SQLite keeps ids as 16 byte
//! blobs the server makes and times as unix seconds. All times are the
//! database clock on both, so a server with a wrong clock cannot hand out
//! sessions that are already over.

use anyhow::{Result, anyhow};
use sqlx::types::{Uuid, uuid::Builder};

use crate::{
    Db,
    auth::{
        User,
        identity::{Identity, Provider},
    },
};

/// What decides whether a login may land in a user.
#[derive(Clone, Copy)]
pub(crate) struct LoginRules<'a> {
    /// Users who may sign in only with some providers.
    pub limits:         &'a [(Uuid, Vec<Provider>)],
    /// A first login through a new provider joins the user of its email.
    pub joins_by_email: bool,
}

impl LoginRules<'_> {
    /// No limit, and a new provider joins by the email.
    #[cfg(any(test, feature = "dev-login"))]
    pub(crate) const OPEN: LoginRules<'static> = LoginRules {
        limits:         &[],
        joins_by_email: true,
    };
}

/// A login through a provider its user may not use, see
/// `AuthConfig::with_only_providers` and `with_no_email_join`. Nothing of
/// it was saved.
#[derive(Debug)]
pub(crate) struct LoginRefused {
    pub provider: Provider,
    pub allowed:  Vec<Provider>,
    pub reason:   Refusal,
}

/// Why a login was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// The user signs in only with the providers of `allowed`.
    Providers,
    /// A link of a second provider: that account belongs to another user.
    OtherUser,
    /// A link of a second provider: that account has another email.
    OtherEmail,
}

impl std::fmt::Display for LoginRefused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "this user may not sign in with {}", self.provider.title())
    }
}

impl std::error::Error for LoginRefused {}

const SESSION_DAYS: i32 = 90;
/// A session ends this long after its sign in, also when it is used every
/// day, so a stolen one that its thief keeps using still runs out.
const SESSION_MAX_DAYS: i32 = 365;
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
pub(crate) use on_db;

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

/// Forgets the short codes nobody typed in time.
pub(crate) async fn prune_codes(db: &Db, minutes: i32) -> Result<(), sqlx::Error> {
    on_db!(
        db,
        "DELETE FROM login_codes WHERE created_at < now() - make_interval(mins => $1)",
        "DELETE FROM login_codes WHERE created_at < unixepoch() - $1 * 60",
        |sql, pool| sqlx::query(sql).bind(minutes).execute(pool).await.map(|_| ())
    )
}

/// Keeps a short code for the login of this `challenge`. False when the code
/// is taken, the caller makes another one.
pub(crate) async fn put_code(
    db: &Db,
    code: &str,
    challenge: &str,
    provider: &str,
) -> Result<bool, sqlx::Error> {
    let both = r"
INSERT INTO login_codes (code, challenge, provider) VALUES ($1, $2, $3)
ON CONFLICT (code) DO NOTHING";
    let put = on_db!(db, both, both, |sql, pool| sqlx::query(sql)
        .bind(code)
        .bind(challenge)
        .bind(provider)
        .execute(pool)
        .await
        .map(|done| done.rows_affected()))?;
    Ok(put > 0)
}

/// The challenge and the provider a code stands for, while it is in time.
/// The code stays, a page that is loaded twice works twice.
pub(crate) async fn code_target(
    db: &Db,
    code: &str,
    minutes: i32,
) -> Result<Option<(String, String)>, sqlx::Error> {
    on_db!(
        db,
        r"
SELECT challenge, provider FROM login_codes
WHERE code = $1 AND created_at > now() - make_interval(mins => $2)",
        r"
SELECT challenge, provider FROM login_codes
WHERE code = $1 AND created_at > unixepoch() - $2 * 60",
        |sql, pool| sqlx::query_as(sql).bind(code).bind(minutes).fetch_optional(pool).await
    )
}

/// A live short code leads to this challenge, so its login is one of a
/// device with no keyboard.
pub(crate) async fn has_code(db: &Db, challenge: &str, minutes: i32) -> Result<bool, sqlx::Error> {
    let found: Option<(String,)> = on_db!(
        db,
        r"
SELECT code FROM login_codes
WHERE challenge = $1 AND created_at > now() - make_interval(mins => $2)",
        r"
SELECT code FROM login_codes
WHERE challenge = $1 AND created_at > unixepoch() - $2 * 60",
        |sql, pool| sqlx::query_as(sql).bind(challenge).bind(minutes).fetch_optional(pool).await
    )?;
    Ok(found.is_some())
}

/// Starts a login or starts it over. A reload of the page starts over. A
/// challenge that already has its user is left alone, or a second person
/// opening the same link could swap their account in under the app that is
/// waiting for the first. False for such a finished login. `hand_back` is
/// the text of a `HandBack`.
pub(crate) async fn start_pending(
    db: &Db,
    challenge: &str,
    state: &str,
    hand_back: &str,
) -> Result<bool, sqlx::Error> {
    let started = on_db!(
        db,
        r"
INSERT INTO pending_logins (challenge, state, hand_back) VALUES ($1, $2, $3)
ON CONFLICT (challenge) DO UPDATE
SET state = EXCLUDED.state, hand_back = EXCLUDED.hand_back, created_at = now()
WHERE pending_logins.user_id IS NULL",
        r"
INSERT INTO pending_logins (challenge, state, hand_back) VALUES ($1, $2, $3)
ON CONFLICT (challenge) DO UPDATE
SET state = excluded.state, hand_back = excluded.hand_back, created_at = unixepoch()
WHERE pending_logins.user_id IS NULL",
        |sql, pool| sqlx::query(sql)
            .bind(challenge)
            .bind(state)
            .bind(hand_back)
            .execute(pool)
            .await
            .map(|done| done.rows_affected())
    )?;
    Ok(started > 0)
}

/// The challenge and the hand back of the login that waits for this `state`
/// while it is still in time. A row of an engine before the hand back has
/// none and comes back as `None`, its app cannot get the session anyway.
pub(crate) async fn waiting(
    db: &Db,
    state: &str,
    minutes: i32,
) -> Result<Option<(String, String, Option<Uuid>)>, sqlx::Error> {
    on_db!(
        db,
        r"
SELECT challenge, hand_back, link_user FROM pending_logins
WHERE state = $1 AND user_id IS NULL AND hand_back IS NOT NULL
AND created_at > now() - make_interval(mins => $2)",
        r"
SELECT challenge, hand_back, link_user FROM pending_logins
WHERE state = $1 AND user_id IS NULL AND hand_back IS NOT NULL
AND created_at > unixepoch() - $2 * 60",
        |sql, pool| sqlx::query_as(sql).bind(state).bind(minutes).fetch_optional(pool).await
    )
}

/// The provider has answered, the login of this `state` has its user now.
/// `finish_hash` is the SHA-256 of the code that goes back to the app, `None`
/// for the login of a device with no keyboard.
pub(crate) async fn finish_pending(
    db: &Db,
    state: &str,
    user_id: Uuid,
    finish_hash: Option<&[u8]>,
) -> Result<(), sqlx::Error> {
    let both = "UPDATE pending_logins SET user_id = $2, finish_hash = $3 WHERE state = $1";
    on_db!(db, both, both, |sql, pool| sqlx::query(sql)
        .bind(state)
        .bind(user_id)
        .bind(finish_hash)
        .execute(pool)
        .await
        .map(|_| ()))
}

/// Takes a finished login out and gives its user. Taking the row out is what
/// makes the hand over happen once, two polls at the same moment cannot both
/// get a session. The verifier alone is not enough: the poll also needs the
/// code that went back to the app, `code_hash` is its SHA-256. Only the
/// login of a device with no keyboard goes without one.
pub(crate) async fn take_finished(
    db: &Db,
    challenge: &str,
    code_hash: Option<&[u8]>,
    minutes: i32,
) -> Result<Option<Uuid>, sqlx::Error> {
    let finished: Option<(Uuid,)> = on_db!(
        db,
        r"
DELETE FROM pending_logins
WHERE challenge = $1 AND user_id IS NOT NULL
AND (hand_back = 'code' OR finish_hash = $2)
AND created_at > now() - make_interval(mins => $3)
RETURNING user_id",
        r"
DELETE FROM pending_logins
WHERE challenge = $1 AND user_id IS NOT NULL
AND (hand_back = 'code' OR finish_hash = $2)
AND created_at > unixepoch() - $3 * 60
RETURNING user_id",
        |sql, pool| sqlx::query_as(sql)
            .bind(challenge)
            .bind(code_hash)
            .bind(minutes)
            .fetch_optional(pool)
            .await
    )?;
    Ok(finished.map(|(user_id,)| user_id))
}

/// The body of `login_user`, the same statements on either database inside
/// one transaction.
macro_rules! login_user_on {
    ($pool:expr, $identity:expr, $rules:expr, $link:expr) => {{
        let identity: &Identity = $identity;
        let rules: LoginRules = $rules;
        let link: Option<Uuid> = $link;
        let limits = rules.limits;
        let provider = identity.provider.name();
        let mut transaction = $pool.begin().await?;

        let known: Option<(Uuid,)> =
            sqlx::query_as("SELECT user_id FROM identities WHERE provider = $1 AND subject = $2")
                .bind(provider)
                .bind(&identity.subject)
                .fetch_optional(&mut *transaction)
                .await?;

        let user_id = if let Some((user_id,)) = known {
            if link.is_some_and(|linked| linked != user_id) {
                return Err(LoginRefused {
                    provider: identity.provider,
                    allowed:  Vec::new(),
                    reason:   Refusal::OtherUser,
                }
                .into());
            }
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
            // A dev account is never joined, so a Google or Apple login on
            // a dev server never lands in an account made with no password.
            let same_email: Option<(Uuid,)> = if identity.provider.joins_by_email() {
                sqlx::query_as(
                    r"
SELECT id FROM users
WHERE lower(email) = lower($1)
AND NOT EXISTS (SELECT 1 FROM identities WHERE identities.user_id = users.id AND identities.provider = 'dev')
ORDER BY created_at LIMIT 1",
                )
                .bind(&identity.email)
                .fetch_optional(&mut *transaction)
                .await?
            } else {
                None
            };

            // A link lands only in the user who asked for it, and only with
            // the same email: the user proved both accounts in 1 browser.
            if let Some(linked) = link
                && same_email.map(|(user_id,)| user_id) != Some(linked)
            {
                return Err(LoginRefused {
                    provider: identity.provider,
                    allowed:  Vec::new(),
                    reason:   Refusal::OtherEmail,
                }
                .into());
            }

            let user_id = if let Some((user_id,)) = same_email {
                if !rules.joins_by_email && link.is_none() {
                    let names: Vec<(String,)> = sqlx::query_as(
                        "SELECT provider FROM identities WHERE user_id = $1 ORDER BY provider",
                    )
                    .bind(user_id)
                    .fetch_all(&mut *transaction)
                    .await?;
                    return Err(LoginRefused {
                        provider: identity.provider,
                        allowed:  names.iter().filter_map(|(name,)| Provider::of_name(name)).collect(),
                        reason:   Refusal::Providers,
                    }
                    .into());
                }
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

        // Checked before the commit, so a refused login joins nothing and
        // changes nothing of the user.
        if let Some((_, allowed)) = limits.iter().find(|(limited, _)| *limited == user_id)
            && !allowed.contains(&identity.provider)
        {
            return Err(LoginRefused {
                provider: identity.provider,
                allowed:  allowed.clone(),
                reason:   Refusal::Providers,
            }
            .into());
        }

        transaction.commit().await?;
        user_id
    }};
}

/// The user of a login, made on the first one. An identity seen before logs
/// into its user, and the email follows the provider, the name and the picture
/// too when the provider sent them. A new identity whose email already belongs
/// to a user joins that user, so a person with Google and Apple on one email
/// is one user. Every email here is verified by its provider. A new user with
/// no name is named by the email. A dev account joins nobody and nobody joins
/// it, its email is not verified by anyone.
#[cfg(any(test, feature = "dev-login"))]
pub(crate) async fn login_user(db: &Db, identity: &Identity) -> Result<Uuid> {
    login_user_limited(db, identity, LoginRules::OPEN, None).await
}

/// The same under `rules`. A user of its limits signs in only with its own
/// providers, and with no join by email a new provider does not land in the
/// user of its email. Such a login fails with [`LoginRefused`]. With `link`
/// the login adds its provider to that user, who asked for it while signed
/// in, see `link_pending`, and to no other.
pub(crate) async fn login_user_limited(
    db: &Db,
    identity: &Identity,
    rules: LoginRules<'_>,
    link: Option<Uuid>,
) -> Result<Uuid> {
    Ok(match db {
        Db::Postgres(pool) => login_user_on!(pool, identity, rules, link),
        Db::Sqlite(pool) => login_user_on!(pool, identity, rules, link),
    })
}

/// Marks the login that will start with `challenge` as a link for `user`,
/// who asked with a session. Only that app knows the challenge, it made the
/// verifier behind it. `state` is a placeholder that no provider brings
/// back, the start of the login sets the real one, and a row with no way
/// back cannot finish. False when a login of this challenge has its user
/// already.
pub(crate) async fn link_pending(
    db: &Db,
    challenge: &str,
    state: &str,
    user: Uuid,
) -> Result<bool, sqlx::Error> {
    let linked = on_db!(
        db,
        r"
INSERT INTO pending_logins (challenge, state, link_user) VALUES ($1, $2, $3)
ON CONFLICT (challenge) DO UPDATE SET link_user = EXCLUDED.link_user, created_at = now()
WHERE pending_logins.user_id IS NULL",
        r"
INSERT INTO pending_logins (challenge, state, link_user) VALUES ($1, $2, $3)
ON CONFLICT (challenge) DO UPDATE SET link_user = excluded.link_user, created_at = unixepoch()
WHERE pending_logins.user_id IS NULL",
        |sql, pool| sqlx::query(sql)
            .bind(challenge)
            .bind(state)
            .bind(user)
            .execute(pool)
            .await
            .map(|done| done.rows_affected())
    )?;
    Ok(linked > 0)
}

/// The providers `user` signs in with, by their column names.
pub(crate) async fn providers_of(db: &Db, user: Uuid) -> Result<Vec<String>, sqlx::Error> {
    let both = "SELECT provider FROM identities WHERE user_id = $1 ORDER BY provider";
    let names: Vec<(String,)> = on_db!(db, both, both, |sql, pool| sqlx::query_as(sql)
        .bind(user)
        .fetch_all(pool)
        .await)?;
    Ok(names.into_iter().map(|(name,)| name).collect())
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
WHERE s.token_hash = $1 AND s.expires_at > now()
  AND s.created_at > now() - make_interval(days => $2)",
        r"
SELECT u.id, u.email, u.name, u.picture, s.last_used_at < unixepoch() - 86400 AS stale
FROM sessions s
JOIN users u ON u.id = s.user_id
WHERE s.token_hash = $1 AND s.expires_at > unixepoch()
  AND s.created_at > unixepoch() - $2 * 86400",
        |sql, pool| sqlx::query_as(sql)
            .bind(token_hash)
            .bind(SESSION_MAX_DAYS)
            .fetch_optional(pool)
            .await
    )?;

    let Some(found) = found else {
        return Ok(None);
    };

    if found.stale {
        match db {
            Db::Postgres(pool) => {
                sqlx::query(
                    r"
UPDATE sessions SET last_used_at = now(),
    expires_at = LEAST(now() + make_interval(days => $2), created_at + make_interval(days => $3))
WHERE token_hash = $1",
                )
                .bind(token_hash)
                .bind(SESSION_DAYS)
                .bind(SESSION_MAX_DAYS)
                .execute(pool)
                .await?;
            }
            Db::Sqlite(pool) => {
                sqlx::query(
                    r"
UPDATE sessions SET last_used_at = unixepoch(),
    expires_at = MIN(unixepoch() + $2, created_at + $3)
WHERE token_hash = $1",
                )
                .bind(token_hash)
                .bind(SESSION_DAYS * DAY_SECONDS)
                .bind(SESSION_MAX_DAYS * DAY_SECONDS)
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

/// The user of an id, `None` when there is none.
pub(crate) async fn user(db: &Db, id: Uuid) -> Result<Option<User>, sqlx::Error> {
    const SQL: &str = "SELECT id, email, name, picture FROM users WHERE id = $1";

    let found: Option<(Uuid, String, String, Option<String>)> = on_db!(db, SQL, SQL, |sql, pool| {
        sqlx::query_as(sql).bind(id).fetch_optional(pool).await
    })?;
    Ok(found.map(|(id, email, name, picture)| User {
        id,
        email,
        name,
        picture,
    }))
}

/// Ends every session of the user, on every device.
pub(crate) async fn delete_user_sessions(db: &Db, user_id: Uuid) -> Result<u64, sqlx::Error> {
    let both = "DELETE FROM sessions WHERE user_id = $1";
    on_db!(db, both, both, |sql, pool| sqlx::query(sql)
        .bind(user_id)
        .execute(pool)
        .await
        .map(|done| done.rows_affected()))
}

pub(crate) async fn delete_session(db: &Db, token_hash: &[u8]) -> Result<(), sqlx::Error> {
    let both = "DELETE FROM sessions WHERE token_hash = $1";
    on_db!(db, both, both, |sql, pool| sqlx::query(sql)
        .bind(token_hash)
        .execute(pool)
        .await
        .map(|_| ()))
}
