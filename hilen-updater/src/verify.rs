//! The checks a downloaded artifact passes before anything is written.

use anyhow::{Context, Result, ensure};
use ed25519_dalek::{Signature, VerifyingKey};
use semver::Version;
use sha2::{Digest, Sha256};

use crate::UpdateArtifact;

/// Whether the manifest version is newer than the running one, by semver.
pub(crate) fn newer(manifest_version: &str, current_version: &str) -> Result<bool> {
    let manifest = Version::parse(manifest_version)?;
    let current = Version::parse(current_version)?;
    Ok(manifest > current)
}

/// Size, then sha256, then the signature, the order `install` has always
/// used.
pub(crate) fn verify_artifact(bytes: &[u8], artifact: &UpdateArtifact, key_hex: &str) -> Result<()> {
    ensure!(
        bytes.len() as u64 == artifact.size,
        "Update artifact size mismatch: expected {} bytes, downloaded {}",
        artifact.size,
        bytes.len()
    );

    verify_sha256(bytes, &artifact.sha256)?;
    verify_signature(bytes, &artifact.sig, key_hex)
}

fn verify_sha256(bytes: &[u8], expected: &str) -> Result<()> {
    let digest = hex::encode(Sha256::digest(bytes));

    ensure!(
        digest == expected.to_lowercase(),
        "Update artifact checksum mismatch: expected {expected}, got {digest}"
    );

    Ok(())
}

fn verify_signature(bytes: &[u8], sig_hex: &str, key_hex: &str) -> Result<()> {
    let key: [u8; 32] = hex::decode(key_hex)?
        .try_into()
        .ok()
        .context("Update verify key must be 32 hex encoded bytes")?;

    let sig: [u8; 64] = hex::decode(sig_hex)?
        .try_into()
        .ok()
        .context("Update artifact signature must be 64 hex encoded bytes")?;

    VerifyingKey::from_bytes(&key)?.verify_strict(bytes, &Signature::from_bytes(&sig))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        env::temp_dir,
        fs::{create_dir_all, read, remove_dir_all, write},
        process,
    };

    use ed25519_dalek::{Signer, SigningKey};
    use sha2::{Digest, Sha256};

    use super::{newer, verify_artifact, verify_sha256, verify_signature};
    use crate::UpdateArtifact;

    #[test]
    fn version_gate() {
        assert!(newer("1.0.1", "1.0.0").unwrap());
        assert!(!newer("1.0.0", "1.0.0").unwrap());
        assert!(!newer("0.9.9", "1.0.0").unwrap());
        assert!(newer("1.0.0", "1.0.0-beta.1").unwrap());
        assert!(newer("2.0.0", "1.9.9").unwrap());
        assert!(newer("10.0.0", "9.0.0").unwrap());
        assert!(newer("v1.0.0", "1.0.0").is_err());
        assert!(newer("1.0.0", "1.0").is_err());
    }

    #[test]
    fn sha256_verifies() {
        let expected = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        verify_sha256(b"hello", expected).unwrap();
        verify_sha256(b"hello", &expected.to_uppercase()).unwrap();
        assert!(verify_sha256(b"other", expected).is_err());
    }

    #[test]
    fn signature_verifies() {
        let signing = SigningKey::from_bytes(&[7; 32]);
        let key_hex = hex::encode(signing.verifying_key().to_bytes());

        let artifact = b"artifact bytes";
        let sig_hex = hex::encode(signing.sign(artifact).to_bytes());

        verify_signature(artifact, &sig_hex, &key_hex).unwrap();
        assert!(verify_signature(b"tampered", &sig_hex, &key_hex).is_err());

        let wrong_key = hex::encode(SigningKey::from_bytes(&[8; 32]).verifying_key().to_bytes());
        assert!(verify_signature(artifact, &sig_hex, &wrong_key).is_err());

        assert!(verify_signature(artifact, &sig_hex, "abcd").is_err());
        assert!(verify_signature(artifact, "abcd", &key_hex).is_err());
    }

    /// A file on disk signed with a throwaway key, described the way CI
    /// describes a release artifact, passes all 3 checks, and each wrong
    /// field alone fails it.
    #[test]
    fn a_signed_file_passes_and_every_wrong_field_fails() {
        let dir = temp_dir().join(format!("hilen-updater-verify-{}", process::id()));
        create_dir_all(&dir).unwrap();
        let path = dir.join("app");
        write(&path, b"the new binary").unwrap();
        let bytes = read(&path).unwrap();

        let signing = SigningKey::from_bytes(&[42; 32]);
        let key_hex = hex::encode(signing.verifying_key().to_bytes());
        let artifact = UpdateArtifact {
            url:    "https://example.com/app".to_string(),
            size:   bytes.len() as u64,
            sha256: hex::encode(Sha256::digest(&bytes)),
            sig:    hex::encode(signing.sign(&bytes).to_bytes()),
        };

        verify_artifact(&bytes, &artifact, &key_hex).unwrap();

        let short = UpdateArtifact {
            size: artifact.size + 1,
            ..artifact.clone()
        };
        let error = verify_artifact(&bytes, &short, &key_hex).unwrap_err();
        assert!(error.to_string().contains("size mismatch"), "{error}");

        let wrong_hash = UpdateArtifact {
            sha256: hex::encode(Sha256::digest(b"another binary")),
            ..artifact.clone()
        };
        let error = verify_artifact(&bytes, &wrong_hash, &key_hex).unwrap_err();
        assert!(error.to_string().contains("checksum mismatch"), "{error}");

        let other_key = SigningKey::from_bytes(&[43; 32]);
        let forged = UpdateArtifact {
            sig: hex::encode(other_key.sign(&bytes).to_bytes()),
            ..artifact.clone()
        };
        assert!(verify_artifact(&bytes, &forged, &key_hex).is_err());

        let wrong_trust = hex::encode(other_key.verifying_key().to_bytes());
        assert!(verify_artifact(&bytes, &artifact, &wrong_trust).is_err());

        remove_dir_all(&dir).unwrap();
    }
}
