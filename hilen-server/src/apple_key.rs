//! A key from the Apple Developer page, the `.p8` file Apple hands out once,
//! and the ES256 tokens a server signs with it. Sign in with Apple and APNs
//! both take such a token in place of a fixed secret.

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow};
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use ring::{
    rand::SystemRandom,
    signature::{ECDSA_P256_SHA256_FIXED_SIGNING, EcdsaKeyPair},
};
use serde::Serialize;
use serde_json::to_vec;

#[derive(Serialize)]
struct Header<'a> {
    alg: &'a str,
    kid: &'a str,
}

/// A token of `claims`, signed with `private_key`, the text of the `.p8`
/// file, whose id is `key_id`.
pub(crate) fn signed_token(private_key: &str, key_id: &str, claims: &impl Serialize) -> Result<String> {
    let header = Header {
        alg: "ES256",
        kid: key_id,
    };
    let message = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(to_vec(&header)?),
        URL_SAFE_NO_PAD.encode(to_vec(claims)?)
    );

    let random = SystemRandom::new();
    let key = EcdsaKeyPair::from_pkcs8(
        &ECDSA_P256_SHA256_FIXED_SIGNING,
        &key_bytes(private_key)?,
        &random,
    )
    .map_err(|error| anyhow!("the Apple private key is not a P-256 key: {error}"))?;
    let signature = key
        .sign(&random, message.as_bytes())
        .map_err(|error| anyhow!("the Apple token was not signed: {error}"))?;

    Ok(format!("{message}.{}", URL_SAFE_NO_PAD.encode(signature)))
}

/// The bytes inside the `.p8` file Apple hands out, a PEM text.
pub(crate) fn key_bytes(pem: &str) -> Result<Vec<u8>> {
    let base64: String = pem.lines().map(str::trim).filter(|line| !line.starts_with("-----")).collect();
    STANDARD.decode(base64).context("the Apple private key is not a PEM text")
}

/// The seconds since 1970, the time a token states.
pub(crate) fn now() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("the clock is before 1970")?
        .as_secs())
}

/// A fresh key the way Apple hands one out, a PEM text.
#[cfg(test)]
pub(crate) fn test_key() -> Result<String> {
    let document = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &SystemRandom::new())
        .map_err(|error| anyhow!("no test key: {error}"))?;
    Ok(format!(
        "-----BEGIN PRIVATE KEY-----\n{}\n-----END PRIVATE KEY-----\n",
        STANDARD.encode(document)
    ))
}
