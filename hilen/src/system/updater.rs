//! Self update for desktop apps. The app opts in by returning an
//! `UpdateSource` from `App::update_source`. `check` fetches the JSON
//! manifest, `install` downloads the platform artifact, verifies its
//! size, checksum and ed25519 signature, then swaps the running
//! executable, and `relaunch` starts the new binary. Mobile updates go
//! through the stores and a wasm app updates by rehosting, so outside
//! the desktop every call is a no-op, matching `system::Router`.

use std::collections::BTreeMap;

use anyhow::Result;
use serde::Deserialize;

/// Everything the updater needs from the app. The verify key is the hex
/// encoded ed25519 public key whose private half signs release
/// artifacts in CI, embedded in the app so a compromised host can not
/// serve a forged binary.
pub struct UpdateSource {
    pub manifest_url:    String,
    pub current_version: String,
    pub verify_key:      String,
}

#[derive(Deserialize)]
pub struct UpdateManifest {
    pub version:   String,
    #[serde(default)]
    pub notes:     String,
    pub platforms: BTreeMap<String, UpdateArtifact>,
}

#[derive(Clone, Deserialize)]
pub struct UpdateArtifact {
    pub url:    String,
    pub size:   u64,
    pub sha256: String,
    pub sig:    String,
}

/// An available update, returned by `check` and consumed by `install`.
pub struct UpdateInfo {
    pub version:    String,
    pub notes:      String,
    pub artifact:   UpdateArtifact,
    pub verify_key: String,
}

pub struct Updater;

impl Updater {
    /// `Ok(None)` means up to date, or no update source, or a platform
    /// with no self update.
    pub async fn check() -> Result<Option<UpdateInfo>> {
        #[cfg(desktop)]
        {
            use anyhow::Context;

            use crate::deps::hreads::from_main;

            let Some(source) = from_main(|| crate::app::app().update_source()).await? else {
                return Ok(None);
            };

            let manifest: UpdateManifest = crate::deps::netrun::rest::get(&source.manifest_url).await?;

            if !newer(&manifest.version, &source.current_version)? {
                return Ok(None);
            }

            let key = platform_key();

            let Some(artifact) = manifest.platforms.get(&key) else {
                anyhow::bail!(
                    "Update manifest for {} has no artifact for {key}",
                    manifest.version
                );
            };

            // A binary the user cannot swap, like a deb install in
            // /usr/bin, gets no offer. Those installs update through
            // their package.
            let target = swap_target()?;
            let dir = target
                .parent()
                .with_context(|| format!("{} has no parent dir", target.display()))?;
            if !dir_writable(dir) {
                log::info!("{} is not writable, no self update", dir.display());
                return Ok(None);
            }

            Ok(Some(UpdateInfo {
                version:    manifest.version,
                notes:      manifest.notes,
                artifact:   artifact.clone(),
                verify_key: source.verify_key,
            }))
        }
        // The API is async on every platform, here the answer is immediate.
        #[cfg(not(desktop))]
        {
            log::trace!("Updater::check outside the desktop is a no-op");
            std::future::ready(Ok(None)).await
        }
    }

    /// Downloads, verifies and swaps the executable. The new binary runs
    /// on the next start, call `relaunch` to switch now.
    pub async fn install(info: UpdateInfo) -> Result<()> {
        Self::install_with_progress(info, |_, _| {}).await
    }

    /// `install` with the download reported as bytes so far and the
    /// total, `None` while the server sent no Content-Length.
    pub async fn install_with_progress(
        info: UpdateInfo,
        on_progress: impl FnMut(u64, Option<u64>) + Send,
    ) -> Result<()> {
        #[cfg(desktop)]
        {
            use anyhow::ensure;

            let bytes =
                crate::deps::netrun::rest::download_with_progress(&info.artifact.url, on_progress).await?;

            ensure!(
                bytes.len() as u64 == info.artifact.size,
                "Update artifact size mismatch: expected {} bytes, downloaded {}",
                info.artifact.size,
                bytes.len()
            );

            verify_sha256(&bytes, &info.artifact.sha256)?;
            verify_signature(&bytes, &info.artifact.sig, &info.verify_key)?;

            if let Some(target) = appimage_path() {
                // The running executable is inside the AppImage's
                // read-only mount, the file to swap is the AppImage
                // itself. The temp lives next to it so the rename
                // stays on one filesystem.
                let temp = target.with_file_name(format!(".hilen-update-{}", info.version));
                std::fs::write(&temp, &bytes)?;
                let swap = replace_keeping_permissions(&temp, &target);
                if swap.is_err() {
                    std::fs::remove_file(&temp)?;
                }
                swap?;
            } else {
                let temp = std::env::temp_dir().join(format!("hilen-update-{}", info.version));
                std::fs::write(&temp, &bytes)?;

                let swap = self_replace::self_replace(&temp);
                std::fs::remove_file(&temp)?;
                swap?;
            }

            Ok(())
        }
        // The API is async on every platform, here the answer is immediate.
        #[cfg(not(desktop))]
        {
            drop(on_progress);
            std::future::ready(Err(anyhow::anyhow!(
                "Self update is desktop only, cannot install version {}",
                info.version
            )))
            .await
        }
    }

    /// Spawns the freshly installed binary and closes this one.
    pub fn relaunch() -> Result<()> {
        #[cfg(desktop)]
        {
            use crate::{AppRunner, deps::hreads::on_main};

            std::process::Command::new(swap_target()?).spawn()?;

            on_main(AppRunner::stop);

            Ok(())
        }
        #[cfg(not(desktop))]
        {
            anyhow::bail!("Self update is desktop only, cannot relaunch")
        }
    }
}

#[cfg(desktop)]
fn platform_key() -> String {
    key(
        std::env::consts::OS,
        std::env::consts::ARCH,
        appimage_path().is_some(),
    )
}

// An AppImage swaps the whole image file, not the bare binary, so it
// gets its own manifest key.
#[cfg(desktop)]
fn key(os: &str, arch: &str, appimage: bool) -> String {
    if appimage {
        format!("{os}-{arch}-appimage")
    } else {
        format!("{os}-{arch}")
    }
}

/// The `AppImage` runtime exports the image path as `APPIMAGE`.
#[cfg(all(desktop, target_os = "linux"))]
fn appimage_path() -> Option<std::path::PathBuf> {
    std::env::var_os("APPIMAGE").map(Into::into)
}

#[cfg(all(desktop, not(target_os = "linux")))]
fn appimage_path() -> Option<std::path::PathBuf> {
    None
}

/// The file `install` replaces, the `AppImage` when running as one, the
/// executable itself otherwise.
#[cfg(desktop)]
fn swap_target() -> Result<std::path::PathBuf> {
    if let Some(image) = appimage_path() {
        return Ok(image);
    }
    Ok(std::env::current_exe()?)
}

// Both swap routes create a temp file in the dir and rename, so dir
// write access is what decides whether an update can install.
#[cfg(desktop)]
fn dir_writable(dir: &std::path::Path) -> bool {
    let probe = dir.join(format!(".hilen-update-probe-{}", std::process::id()));
    match std::fs::OpenOptions::new().write(true).create_new(true).open(&probe) {
        Ok(file) => {
            drop(file);
            if let Err(err) = std::fs::remove_file(&probe) {
                log::warn!("Failed to remove write probe {}: {err}", probe.display());
            }
            true
        }
        Err(_) => false,
    }
}

#[cfg(desktop)]
fn replace_keeping_permissions(temp: &std::path::Path, target: &std::path::Path) -> Result<()> {
    let permissions = std::fs::metadata(target)?.permissions();
    std::fs::set_permissions(temp, permissions)?;
    std::fs::rename(temp, target)?;
    Ok(())
}

#[cfg(desktop)]
fn newer(manifest_version: &str, current_version: &str) -> Result<bool> {
    let manifest = semver::Version::parse(manifest_version)?;
    let current = semver::Version::parse(current_version)?;
    Ok(manifest > current)
}

#[cfg(desktop)]
fn verify_sha256(bytes: &[u8], expected: &str) -> Result<()> {
    use sha2::{Digest, Sha256};

    let digest = hex::encode(Sha256::digest(bytes));

    anyhow::ensure!(
        digest == expected.to_lowercase(),
        "Update artifact checksum mismatch: expected {expected}, got {digest}"
    );

    Ok(())
}

#[cfg(desktop)]
fn verify_signature(bytes: &[u8], sig_hex: &str, key_hex: &str) -> Result<()> {
    use anyhow::Context;
    use ed25519_dalek::{Signature, VerifyingKey};

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

#[cfg(all(test, desktop))]
mod tests {
    use ed25519_dalek::{Signer, SigningKey};

    use super::{
        UpdateManifest, dir_writable, key, newer, replace_keeping_permissions, verify_sha256,
        verify_signature,
    };

    #[test]
    fn manifest_parses() {
        let json = r#"{
            "version": "1.2.3",
            "notes": "fixes",
            "platforms": {
                "macos-aarch64": {
                    "url": "https://example.com/app",
                    "size": 4,
                    "sha256": "abc",
                    "sig": "def"
                }
            }
        }"#;

        let manifest: UpdateManifest = serde_json::from_str(json).unwrap();

        assert_eq!(manifest.version, "1.2.3");
        assert_eq!(manifest.notes, "fixes");
        assert_eq!(manifest.platforms["macos-aarch64"].size, 4);
    }

    #[test]
    fn notes_are_optional() {
        let json = r#"{ "version": "1.0.0", "platforms": {} }"#;
        let manifest: UpdateManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.notes, "");
    }

    #[test]
    fn version_gate() {
        assert!(newer("1.0.1", "1.0.0").unwrap());
        assert!(!newer("1.0.0", "1.0.0").unwrap());
        assert!(!newer("0.9.9", "1.0.0").unwrap());
        assert!(newer("1.0.0", "1.0.0-beta.1").unwrap());
        assert!(newer("2.0.0", "1.9.9").unwrap());
        assert!(newer("10.0.0", "9.0.0").unwrap());
    }

    #[test]
    fn sha256_verifies() {
        let expected = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        verify_sha256(b"hello", expected).unwrap();
        verify_sha256(b"hello", &expected.to_uppercase()).unwrap();
        assert!(verify_sha256(b"other", expected).is_err());
    }

    #[test]
    fn platform_keys() {
        assert_eq!(key("macos", "aarch64", false), "macos-aarch64");
        assert_eq!(key("linux", "x86_64", false), "linux-x86_64");
        assert_eq!(key("linux", "x86_64", true), "linux-x86_64-appimage");
    }

    #[test]
    fn writable_probe() {
        let dir = std::env::temp_dir().join(format!("hilen-writable-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(dir_writable(&dir));
        // Root writes into a read only directory anyway, and the linux CI
        // containers run as root, so the negative half only holds for a
        // plain user.
        #[cfg(unix)]
        if unsafe { libc::geteuid() } != 0 {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
            assert!(!dir_writable(&dir));
            std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn replace_keeps_permissions() {
        let dir = std::env::temp_dir().join(format!("hilen-replace-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("app.AppImage");
        let temp = dir.join(".hilen-update-1.0.0");
        std::fs::write(&target, b"old").unwrap();
        std::fs::write(&temp, b"new").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        replace_keeping_permissions(&temp, &target).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"new");
        assert!(!temp.exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&target).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o755);
        }
        std::fs::remove_dir_all(&dir).unwrap();
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
    }
}
