//! Every query of the Google access, once per database. Times are the
//! database clock on both, like in the login store.

use crate::{
    Db,
    auth::store::on_db,
    google_access::wire::{Role, StartQuery},
};

/// Forgets the sign ins nobody finished or picked up in time.
pub(crate) async fn prune(db: &Db, minutes: i32) -> Result<(), sqlx::Error> {
    on_db!(
        db,
        "DELETE FROM google_pending WHERE created_at < now() - make_interval(mins => $1)",
        "DELETE FROM google_pending WHERE created_at < unixepoch() - $1 * 60",
        |sql, pool| sqlx::query(sql).bind(minutes).execute(pool).await.map(|_| ())
    )
}

/// Starts a sign in or starts it over. One that already has its tokens is
/// left alone, or a second person opening the same link could put their
/// account under the app that waits for the first. False for such a one.
pub(crate) async fn start(db: &Db, query: &StartQuery, state: &str) -> Result<bool, sqlx::Error> {
    let started = on_db!(
        db,
        r"
INSERT INTO google_pending (challenge, state, device_key, role, platform, app_version)
VALUES ($1, $2, $3, $4, $5, $6)
ON CONFLICT (challenge) DO UPDATE SET state = EXCLUDED.state, created_at = now()
WHERE google_pending.sealed IS NULL",
        r"
INSERT INTO google_pending (challenge, state, device_key, role, platform, app_version)
VALUES ($1, $2, $3, $4, $5, $6)
ON CONFLICT (challenge) DO UPDATE SET state = excluded.state, created_at = unixepoch()
WHERE google_pending.sealed IS NULL",
        |sql, pool| sqlx::query(sql)
            .bind(&query.challenge)
            .bind(state)
            .bind(&query.key)
            .bind(query.role.name())
            .bind(&query.platform)
            .bind(&query.version)
            .execute(pool)
            .await
            .map(|done| done.rows_affected())
    )?;
    Ok(started > 0)
}

/// A sign in that waits for Google to answer.
pub(crate) struct Waiting {
    pub challenge:   String,
    pub device_key:  String,
    pub role:        Role,
    pub platform:    String,
    pub app_version: String,
}

pub(crate) async fn waiting(db: &Db, state: &str, minutes: i32) -> Result<Option<Waiting>, sqlx::Error> {
    let row: Option<(String, String, String, String, String)> = on_db!(
        db,
        r"
SELECT challenge, device_key, role, platform, app_version FROM google_pending
WHERE state = $1 AND sealed IS NULL AND created_at > now() - make_interval(mins => $2)",
        r"
SELECT challenge, device_key, role, platform, app_version FROM google_pending
WHERE state = $1 AND sealed IS NULL AND created_at > unixepoch() - $2 * 60",
        |sql, pool| sqlx::query_as(sql).bind(state).bind(minutes).fetch_optional(pool).await
    )?;

    Ok(
        row.map(|(challenge, device_key, role, platform, app_version)| Waiting {
            challenge,
            device_key,
            role: Role::of(&role),
            platform,
            app_version,
        }),
    )
}

/// Google has answered, the sign in of this `state` has its sealed tokens.
pub(crate) async fn finish(db: &Db, state: &str, sealed: &str) -> Result<(), sqlx::Error> {
    let both = "UPDATE google_pending SET sealed = $2 WHERE state = $1";
    on_db!(db, both, both, |sql, pool| sqlx::query(sql)
        .bind(state)
        .bind(sealed)
        .execute(pool)
        .await
        .map(|_| ()))
}

/// Takes a finished sign in out and gives its sealed tokens. Taking the row
/// out is what makes the hand over happen once.
pub(crate) async fn take(db: &Db, challenge: &str, minutes: i32) -> Result<Option<String>, sqlx::Error> {
    let taken: Option<(String,)> = on_db!(
        db,
        r"
DELETE FROM google_pending
WHERE challenge = $1 AND sealed IS NOT NULL AND created_at > now() - make_interval(mins => $2)
RETURNING sealed",
        r"
DELETE FROM google_pending
WHERE challenge = $1 AND sealed IS NOT NULL AND created_at > unixepoch() - $2 * 60
RETURNING sealed",
        |sql, pool| sqlx::query_as(sql).bind(challenge).bind(minutes).fetch_optional(pool).await
    )?;
    Ok(taken.map(|(sealed,)| sealed))
}

/// A main account signed in. Its row is made on the first time.
pub(crate) async fn main_signed_in(
    db: &Db,
    subject: &str,
    email: &str,
    name: &str,
    platform: &str,
    app_version: &str,
) -> Result<(), sqlx::Error> {
    on_db!(
        db,
        r"
INSERT INTO google_users (subject, email, name, platform, app_version) VALUES ($1, $2, $3, $4, $5)
ON CONFLICT (subject) DO UPDATE SET email = EXCLUDED.email, name = EXCLUDED.name,
    platform = EXCLUDED.platform, app_version = EXCLUDED.app_version, last_seen = now()",
        r"
INSERT INTO google_users (subject, email, name, platform, app_version) VALUES ($1, $2, $3, $4, $5)
ON CONFLICT (subject) DO UPDATE SET email = excluded.email, name = excluded.name,
    platform = excluded.platform, app_version = excluded.app_version, last_seen = unixepoch()",
        |sql, pool| sqlx::query(sql)
            .bind(subject)
            .bind(email)
            .bind(name)
            .bind(platform)
            .bind(app_version)
            .execute(pool)
            .await
            .map(|_| ())
    )
}

/// A main account renewed its token. A subject with no row is a linked
/// account, and nothing is written for it.
pub(crate) async fn main_seen(
    db: &Db,
    subject: &str,
    platform: &str,
    app_version: &str,
    linked_accounts: Option<i32>,
) -> Result<(), sqlx::Error> {
    on_db!(
        db,
        r"
UPDATE google_users SET last_seen = now(), platform = $2, app_version = $3,
    linked_accounts = COALESCE($4, linked_accounts)
WHERE subject = $1",
        r"
UPDATE google_users SET last_seen = unixepoch(), platform = $2, app_version = $3,
    linked_accounts = COALESCE($4, linked_accounts)
WHERE subject = $1",
        |sql, pool| sqlx::query(sql)
            .bind(subject)
            .bind(platform)
            .bind(app_version)
            .bind(linked_accounts)
            .execute(pool)
            .await
            .map(|_| ())
    )
}
