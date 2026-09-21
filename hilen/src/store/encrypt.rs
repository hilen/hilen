use aes_gcm::{
    Aes256Gcm,
    aead::{Aead, KeyInit},
};
use anyhow::{Result, anyhow, bail};

pub(crate) const KEY_SIZE: usize = 32;
const NONCE_SIZE: usize = 12;

pub(crate) type EncryptionKey = [u8; KEY_SIZE];

/// AES-256-GCM. The nonce is fresh for every call and sits in front of the
/// sealed bytes, one key must never seal two messages under the same nonce.
pub(crate) fn encrypt(data: &[u8], key: &EncryptionKey) -> Result<Vec<u8>> {
    let mut nonce = [0; NONCE_SIZE];
    getrandom::fill(&mut nonce).map_err(|error| anyhow!("no random bytes for the nonce: {error}"))?;

    let cipher = Aes256Gcm::new(&(*key).into());
    let sealed = cipher
        .encrypt(&nonce.into(), data)
        .map_err(|error| anyhow!("failed to encrypt: {error}"))?;

    let mut out = Vec::with_capacity(NONCE_SIZE + sealed.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&sealed);
    Ok(out)
}

/// Fails on a wrong key and on changed bytes, GCM checks both.
pub(crate) fn decrypt(data: &[u8], key: &EncryptionKey) -> Result<Vec<u8>> {
    let Some((nonce, sealed)) = data.split_at_checked(NONCE_SIZE) else {
        bail!("the encrypted data is shorter than its nonce");
    };
    let nonce: [u8; NONCE_SIZE] = nonce.try_into()?;

    let cipher = Aes256Gcm::new(&(*key).into());
    cipher
        .decrypt(&nonce.into(), sealed)
        .map_err(|error| anyhow!("failed to decrypt: {error}"))
}

#[cfg(test)]
mod test {
    use anyhow::Result;

    use super::{EncryptionKey, KEY_SIZE, decrypt, encrypt};

    const KEY: EncryptionKey = [7; KEY_SIZE];
    const TEXT: &[u8] = b"a session token";

    #[test]
    fn round_trip() -> Result<()> {
        let sealed = encrypt(TEXT, &KEY)?;
        assert_ne!(sealed, TEXT);
        assert_eq!(decrypt(&sealed, &KEY)?, TEXT);
        Ok(())
    }

    #[test]
    fn every_call_has_its_own_nonce() -> Result<()> {
        assert_ne!(encrypt(TEXT, &KEY)?, encrypt(TEXT, &KEY)?);
        Ok(())
    }

    #[test]
    fn wrong_key_fails() -> Result<()> {
        let sealed = encrypt(TEXT, &KEY)?;
        assert!(decrypt(&sealed, &[8; KEY_SIZE]).is_err());
        Ok(())
    }

    #[test]
    fn changed_bytes_fail() -> Result<()> {
        let mut sealed = encrypt(TEXT, &KEY)?;
        let last = sealed.len() - 1;
        sealed[last] ^= 1;
        assert!(decrypt(&sealed, &KEY).is_err());
        Ok(())
    }

    #[test]
    fn short_data_fails() {
        assert!(decrypt(&[1, 2, 3], &KEY).is_err());
    }
}
