//! Session tokens. A token is 32 random bytes as hex. The database keeps only
//! its SHA-256, so a copy of the `sessions` table logs nobody in.

use anyhow::{Result, anyhow};
use sha2::{Digest, Sha256};
use sqlx::types::Uuid;

use crate::{
    Db,
    auth::{User, store},
};

/// 64 hex characters of OS randomness. Also the `state` of a Google redirect.
pub(crate) fn new_token() -> Result<String> {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).map_err(|error| anyhow!("no random bytes for a token: {error}"))?;
    Ok(hex::encode(bytes))
}

pub(crate) fn hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

pub(crate) async fn create(db: &Db, user_id: Uuid) -> Result<String> {
    let token = new_token()?;
    store::create_session(db, &hash(&token), user_id).await?;
    Ok(token)
}

pub(crate) async fn user_of(db: &Db, token: &str) -> Result<Option<User>, sqlx::Error> {
    store::session_user(db, &hash(token)).await
}

pub(crate) async fn delete(db: &Db, token: &str) -> Result<(), sqlx::Error> {
    store::delete_session(db, &hash(token)).await
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
