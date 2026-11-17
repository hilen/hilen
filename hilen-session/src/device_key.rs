//! The device half of a hand over that a server cannot read. The device makes
//! a key pair for one use and sends the public half along with its request.
//! The server seals its answer for that key, see `google_access/seal.rs` in
//! `hilen-server`, and only this private half opens it.
//!
//! X25519 for the agreement, HKDF-SHA256 for the message key, AES-256-GCM
//! for the message. A sealed text is base64url of `server public key, 32
//! bytes | nonce, 12 bytes | ciphertext and tag`.

use aes_gcm::{
    Aes256Gcm,
    aead::{Aead, KeyInit, Payload},
};
use anyhow::{Result, anyhow, bail};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hkdf::Hkdf;
use sha2::Sha256;
use x25519_dalek::{PublicKey, StaticSecret};

const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const INFO: &[u8] = b"hilen google access v1";

pub struct DeviceKey {
    secret: StaticSecret,
    public: PublicKey,
}

impl DeviceKey {
    pub fn new() -> Result<Self> {
        let mut bytes = [0; KEY_LEN];
        getrandom::fill(&mut bytes).map_err(|error| anyhow!("no random bytes for a device key: {error}"))?;
        let secret = StaticSecret::from(bytes);
        let public = PublicKey::from(&secret);
        Ok(Self { secret, public })
    }

    /// The public half the way a server takes it, base64url of 32 bytes.
    pub fn public(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.public.as_bytes())
    }

    /// Opens what a server sealed for this key. `context` is the text both
    /// sides bound the message to, a sealed text does not open under another.
    pub fn open(&self, context: &str, sealed: &str) -> Result<Vec<u8>> {
        let bytes = URL_SAFE_NO_PAD
            .decode(sealed)
            .map_err(|error| anyhow!("the sealed text is not base64url: {error}"))?;
        if bytes.len() < KEY_LEN + NONCE_LEN {
            bail!("the sealed text is too short");
        }
        let (server_public, rest) = bytes.split_at(KEY_LEN);
        let (nonce, ciphertext) = rest.split_at(NONCE_LEN);
        let server_key: [u8; KEY_LEN] = server_public.try_into()?;
        let nonce: [u8; NONCE_LEN] = nonce.try_into()?;

        let shared = self.secret.diffie_hellman(&PublicKey::from(server_key));
        let salt = [server_public, self.public.as_bytes()].concat();
        let mut key = [0; KEY_LEN];
        Hkdf::<Sha256>::new(Some(&salt), shared.as_bytes())
            .expand(INFO, &mut key)
            .map_err(|error| anyhow!("the message key could not be made: {error}"))?;

        Aes256Gcm::new(&key.into())
            .decrypt(
                &nonce.into(),
                Payload {
                    msg: ciphertext,
                    aad: context.as_bytes(),
                },
            )
            .map_err(|_| anyhow!("the sealed text does not open"))
    }
}

#[cfg(test)]
mod test {
    use anyhow::Result;

    use super::DeviceKey;

    #[test]
    fn a_key_is_fresh_and_32_bytes_as_base64url() -> Result<()> {
        let key = DeviceKey::new()?;
        assert_eq!(key.public().len(), 43);
        assert_ne!(key.public(), DeviceKey::new()?.public());
        Ok(())
    }

    #[test]
    fn junk_does_not_open() -> Result<()> {
        let key = DeviceKey::new()?;
        assert!(key.open("context", "short").is_err());
        assert!(key.open("context", &"A".repeat(120)).is_err());
        Ok(())
    }
}
