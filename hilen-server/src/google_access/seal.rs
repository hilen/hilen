//! Tokens are sealed for the device that asked, before anything is written
//! down. The device sends an X25519 public key with its sign in. This server
//! makes a key pair for one use, agrees on a secret with the device key, and
//! encrypts with AES-256-GCM under a key that HKDF-SHA256 makes from that
//! secret. The private half is dropped right after, so nothing on this
//! server can open the result again.
//!
//! A sealed text is base64url of `server public key, 32 bytes | nonce, 12
//! bytes | ciphertext and tag`. The challenge of the sign in is the
//! additional data, a sealed text does not open under another challenge.

use anyhow::{Result, anyhow};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ring::{
    aead::{AES_256_GCM, Aad, LessSafeKey, NONCE_LEN, Nonce, UnboundKey},
    agreement::{EphemeralPrivateKey, UnparsedPublicKey, X25519, agree_ephemeral},
    hkdf::{HKDF_SHA256, Salt},
    rand::{SecureRandom, SystemRandom},
};

pub(crate) const KEY_LEN: usize = 32;
const INFO: &[u8] = b"hilen google access v1";

/// The device key as the app sends it, base64url of 32 bytes.
pub(crate) fn device_key(text: &str) -> Result<[u8; KEY_LEN]> {
    let bytes = URL_SAFE_NO_PAD
        .decode(text)
        .map_err(|error| anyhow!("the device key is not base64url: {error}"))?;
    bytes.try_into().map_err(|_| anyhow!("the device key is not {KEY_LEN} bytes"))
}

fn message_key(shared: &[u8], server_public: &[u8], device_public: &[u8]) -> Result<LessSafeKey> {
    let salt = [server_public, device_public].concat();
    let prk = Salt::new(HKDF_SHA256, &salt).extract(shared);
    let okm = prk
        .expand(&[INFO], &AES_256_GCM)
        .map_err(|_| anyhow!("the message key could not be made"))?;
    Ok(LessSafeKey::new(UnboundKey::from(okm)))
}

pub(crate) fn seal(device_public: &[u8; KEY_LEN], challenge: &str, plain: &[u8]) -> Result<String> {
    let random = SystemRandom::new();
    let private =
        EphemeralPrivateKey::generate(&X25519, &random).map_err(|_| anyhow!("no key pair for a seal"))?;
    let public = private.compute_public_key().map_err(|_| anyhow!("no public key for a seal"))?;

    let key = agree_ephemeral(
        private,
        &UnparsedPublicKey::new(&X25519, device_public),
        |shared| message_key(shared, public.as_ref(), device_public),
    )
    .map_err(|_| anyhow!("the device key is not a usable X25519 key"))??;

    let mut nonce = [0; NONCE_LEN];
    random.fill(&mut nonce).map_err(|_| anyhow!("no random bytes for a nonce"))?;

    let mut sealed = plain.to_vec();
    key.seal_in_place_append_tag(
        Nonce::assume_unique_for_key(nonce),
        Aad::from(challenge.as_bytes()),
        &mut sealed,
    )
    .map_err(|_| anyhow!("the seal failed"))?;

    Ok(URL_SAFE_NO_PAD.encode([public.as_ref(), &nonce, &sealed].concat()))
}

/// What the device does with its private key. Here for the tests only, the
/// client half of this lives in the `hilen` crate.
#[cfg(test)]
pub(crate) fn open(
    device_private: EphemeralPrivateKey,
    device_public: &[u8],
    challenge: &str,
    sealed: &str,
) -> Result<Vec<u8>> {
    let bytes = URL_SAFE_NO_PAD.decode(sealed)?;
    if bytes.len() < KEY_LEN + NONCE_LEN {
        anyhow::bail!("the sealed text is too short");
    }
    let (server_public, rest) = bytes.split_at(KEY_LEN);
    let (nonce, ciphertext) = rest.split_at(NONCE_LEN);

    let key = agree_ephemeral(
        device_private,
        &UnparsedPublicKey::new(&X25519, server_public),
        |shared| message_key(shared, server_public, device_public),
    )
    .map_err(|_| anyhow!("the server key is not a usable X25519 key"))??;

    let mut plain = ciphertext.to_vec();
    let nonce = Nonce::try_assume_unique_for_key(nonce).map_err(|_| anyhow!("a bad nonce"))?;
    let len = key
        .open_in_place(nonce, Aad::from(challenge.as_bytes()), &mut plain)
        .map_err(|_| anyhow!("the sealed text does not open"))?
        .len();
    plain.truncate(len);
    Ok(plain)
}

/// A device key pair for a test.
#[cfg(test)]
pub(crate) fn test_device() -> Result<(EphemeralPrivateKey, [u8; KEY_LEN])> {
    let private = EphemeralPrivateKey::generate(&X25519, &SystemRandom::new())
        .map_err(|_| anyhow!("no device key pair"))?;
    let public = private.compute_public_key().map_err(|_| anyhow!("no device public key"))?;
    Ok((private, public.as_ref().try_into()?))
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

    use super::{device_key, open, seal, test_device as device};

    #[test]
    fn a_sealed_text_opens_with_the_device_key_only() -> Result<()> {
        let (private, public) = device()?;
        let sealed = seal(&public, "challenge", b"the tokens")?;

        assert!(!sealed.contains("the tokens"));
        assert_eq!(open(private, &public, "challenge", &sealed)?, b"the tokens");

        let (other_private, _) = device()?;
        assert!(open(other_private, &public, "challenge", &sealed).is_err());
        Ok(())
    }

    #[test]
    fn a_sealed_text_does_not_open_under_another_challenge() -> Result<()> {
        let (private, public) = device()?;
        let sealed = seal(&public, "challenge", b"the tokens")?;
        assert!(open(private, &public, "another", &sealed).is_err());
        Ok(())
    }

    #[test]
    fn two_seals_of_one_text_differ() -> Result<()> {
        let (_, public) = device()?;
        assert_ne!(seal(&public, "c", b"same")?, seal(&public, "c", b"same")?);
        Ok(())
    }

    #[test]
    fn device_key_shape() -> Result<()> {
        let (_, public) = device()?;
        assert_eq!(device_key(&URL_SAFE_NO_PAD.encode(public))?, public);
        assert!(device_key("short").is_err());
        assert!(device_key("not base64 !").is_err());
        Ok(())
    }
}
