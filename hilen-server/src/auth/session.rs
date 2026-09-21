//! Session tokens. A token is 32 random bytes as hex. The database keeps only
//! its SHA-256, so a copy of the `sessions` table logs nobody in.

use anyhow::{Result, anyhow};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, types::Uuid};

use crate::auth::User;

/// All times are the database clock, so a server with a wrong clock cannot
/// hand out sessions that are already over.
const LIFETIME_DAYS: i32 = 90;

/// 64 hex characters of OS randomness. Also the `state` of a Google redirect.
pub(crate) fn new_token() -> Result<String> {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).map_err(|error| anyhow!("no random bytes for a token: {error}"))?;
    Ok(hex::encode(bytes))
}

pub(crate) fn hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

pub(crate) async fn create(db: &PgPool, user_id: Uuid) -> Result<String> {
    let token = new_token()?;

    sqlx::query(
        r"
INSERT INTO sessions (token_hash, user_id, expires_at)
VALUES ($1, $2, now() + make_interval(days => $3))",
    )
    .bind(hash(&token))
    .bind(user_id)
    .bind(LIFETIME_DAYS)
    .execute(db)
    .await?;

    Ok(token)
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
pub(crate) async fn user_of(db: &PgPool, token: &str) -> Result<Option<User>, sqlx::Error> {
    let token_hash = hash(token);

    let found: Option<SessionUser> = sqlx::query_as(
        r"
SELECT u.id, u.email, u.name, u.picture, s.last_used_at < now() - interval '1 day' AS stale
FROM sessions s
JOIN users u ON u.id = s.user_id
WHERE s.token_hash = $1 AND s.expires_at > now()",
    )
    .bind(&token_hash)
    .fetch_optional(db)
    .await?;

    let Some(found) = found else {
        return Ok(None);
    };

    if found.stale {
        sqlx::query(
            r"
UPDATE sessions SET last_used_at = now(), expires_at = now() + make_interval(days => $2)
WHERE token_hash = $1",
        )
        .bind(&token_hash)
        .bind(LIFETIME_DAYS)
        .execute(db)
        .await?;
    }

    Ok(Some(User {
        id:      found.id,
        email:   found.email,
        name:    found.name,
        picture: found.picture,
    }))
}

pub(crate) async fn delete(db: &PgPool, token: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
        .bind(hash(token))
        .execute(db)
        .await?;
    Ok(())
}

#[cfg(test)]
mod test {
    use anyhow::Result;

    use super::{hash, new_token};

    #[test]
    fn tokens_are_long_and_fresh() -> Result<()> {
        let token = new_token()?;
        assert_eq!(token.len(), 64);
        assert_ne!(token, new_token()?);
        Ok(())
    }

    #[test]
    fn hash_is_the_sha256_of_the_token() {
        // The known SHA-256 of "abc".
        assert_eq!(
            hex::encode(hash("abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
