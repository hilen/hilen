//! The 3 calls: check, install and relaunch.

use std::{
    any::type_name,
    env::temp_dir,
    process::{Command, exit},
};

use anyhow::{Context, Result, bail};
use log::info;
use self_replace::self_replace;
use serde_json::from_str;
use tokio::fs::{remove_file, write};

use crate::{
    UpdateArtifact, UpdateInfo, UpdateManifest, UpdateSource,
    net::{download_with_progress, get_text},
    platform::{appimage_path, dir_writable, platform_key, replace_keeping_permissions, swap_target},
    verify::{newer, verify_artifact},
};

/// Fetches the manifest of `source`. `Ok(Some)` only when its version is
/// newer by semver, it has an entry for this platform, and the directory of
/// the swap target is writable. `Ok(None)` means up to date or a binary the
/// user cannot swap. A newer manifest with no entry for this platform is an
/// error.
pub async fn check(source: &UpdateSource) -> Result<Option<UpdateInfo>> {
    let body = get_text(&source.manifest_url).await?;
    let manifest = parse_manifest(&body)?;

    let Some(artifact) = pick(&manifest, &source.current_version, &platform_key())? else {
        return Ok(None);
    };

    // A binary the user cannot swap, like a deb install in /usr/bin, gets no
    // offer. Those installs update through their package.
    let target = swap_target()?;
    let dir = target
        .parent()
        .with_context(|| format!("{} has no parent dir", target.display()))?;
    if !dir_writable(dir) {
        info!("{} is not writable, no self update", dir.display());
        return Ok(None);
    }

    Ok(Some(UpdateInfo {
        version: manifest.version,
        notes: manifest.notes,
        artifact,
        verify_key: source.verify_key.clone(),
    }))
}

/// Downloads, verifies and swaps the executable. The new binary runs on the
/// next start, call `relaunch` to switch now.
pub async fn install(info: UpdateInfo) -> Result<()> {
    install_with_progress(info, |_, _| {}).await
}

/// `install` with the download reported as bytes so far and the total,
/// `None` while the server sent no Content-Length. The callback runs on the
/// download task.
pub async fn install_with_progress(
    info: UpdateInfo,
    on_progress: impl FnMut(u64, Option<u64>) + Send,
) -> Result<()> {
    let bytes = download_with_progress(&info.artifact.url, on_progress).await?;

    // Nothing is written before every check passes.
    verify_artifact(&bytes, &info.artifact, &info.verify_key)?;

    if let Some(target) = appimage_path() {
        // The running executable is inside the AppImage's read-only mount,
        // the file to swap is the AppImage itself. The temp lives next to it
        // so the rename stays on one filesystem.
        let temp = target.with_file_name(format!(".hilen-update-{}", info.version));
        write(&temp, &bytes).await?;
        let swap = replace_keeping_permissions(&temp, &target);
        if swap.is_err() {
            remove_file(&temp).await?;
        }
        swap?;
    } else {
        let temp = temp_dir().join(format!("hilen-update-{}", info.version));
        write(&temp, &bytes).await?;

        let swap = self_replace(&temp);
        remove_file(&temp).await?;
        swap?;
    }

    Ok(())
}

/// Starts the freshly installed binary at the same path and leaves this
/// process running. An app with its own shutdown, like a `hilen` app, calls
/// this and then stops itself.
pub fn start_new_binary() -> Result<()> {
    Command::new(swap_target()?).spawn()?;
    Ok(())
}

/// Starts the freshly installed binary and ends this process at once.
pub fn relaunch() -> Result<()> {
    start_new_binary()?;
    exit(0)
}

fn parse_manifest(body: &str) -> Result<UpdateManifest> {
    from_str(body).with_context(|| format!("failed to parse {} from {body}", type_name::<UpdateManifest>()))
}

/// The artifact of `platform` when the manifest is newer than `current`.
fn pick(manifest: &UpdateManifest, current: &str, platform: &str) -> Result<Option<UpdateArtifact>> {
    if !newer(&manifest.version, current)? {
        return Ok(None);
    }

    let Some(artifact) = manifest.platforms.get(platform) else {
        bail!(
            "Update manifest for {} has no artifact for {platform}",
            manifest.version
        );
    };

    Ok(Some(artifact.clone()))
}

#[cfg(test)]
mod tests {
    use super::{parse_manifest, pick};

    const MANIFEST: &str = r#"{
        "version": "1.2.3",
        "notes": "fixes",
        "platforms": {
            "macos-aarch64": {
                "url": "https://example.com/app",
                "size": 4,
                "sha256": "abc",
                "sig": "def"
            },
            "linux-x86_64-appimage": {
                "url": "https://example.com/app.AppImage",
                "size": 9,
                "sha256": "123",
                "sig": "456"
            }
        }
    }"#;

    #[test]
    fn manifest_parses() {
        let manifest = parse_manifest(MANIFEST).unwrap();

        assert_eq!(manifest.version, "1.2.3");
        assert_eq!(manifest.notes, "fixes");
        assert_eq!(manifest.platforms.len(), 2);
        let mac = &manifest.platforms["macos-aarch64"];
        assert_eq!(mac.url, "https://example.com/app");
        assert_eq!(mac.size, 4);
        assert_eq!(mac.sha256, "abc");
        assert_eq!(mac.sig, "def");
    }

    #[test]
    fn notes_are_optional() {
        let manifest = parse_manifest(r#"{ "version": "1.0.0", "platforms": {} }"#).unwrap();
        assert_eq!(manifest.notes, "");
    }

    #[test]
    fn a_manifest_missing_a_field_does_not_parse() {
        assert!(parse_manifest(r#"{ "platforms": {} }"#).is_err());
        let no_sig = r#"{ "version": "1.0.0", "platforms": { "macos-aarch64": { "url": "u", "size": 1, "sha256": "a" } } }"#;
        let error = parse_manifest(no_sig).unwrap_err();
        assert!(format!("{error:#}").contains("sig"), "{error:#}");
        assert!(parse_manifest("<html>not found</html>").is_err());
    }

    #[test]
    fn a_newer_manifest_gives_the_entry_of_this_platform() {
        let manifest = parse_manifest(MANIFEST).unwrap();

        let mac = pick(&manifest, "1.2.2", "macos-aarch64").unwrap().unwrap();
        assert_eq!(mac.url, "https://example.com/app");

        let image = pick(&manifest, "1.0.0", "linux-x86_64-appimage").unwrap().unwrap();
        assert_eq!(image.url, "https://example.com/app.AppImage");
    }

    #[test]
    fn the_same_or_an_older_manifest_gives_nothing() {
        let manifest = parse_manifest(MANIFEST).unwrap();

        assert!(pick(&manifest, "1.2.3", "macos-aarch64").unwrap().is_none());
        assert!(pick(&manifest, "2.0.0", "macos-aarch64").unwrap().is_none());
        // No offer means no error either, even for a platform it lacks.
        assert!(pick(&manifest, "1.2.3", "windows-x86_64").unwrap().is_none());
    }

    #[test]
    fn a_newer_manifest_without_this_platform_is_an_error() {
        let manifest = parse_manifest(MANIFEST).unwrap();

        let error = pick(&manifest, "1.0.0", "windows-x86_64").unwrap_err();
        assert_eq!(
            error.to_string(),
            "Update manifest for 1.2.3 has no artifact for windows-x86_64"
        );
        // A bare linux binary does not take the AppImage entry.
        assert!(pick(&manifest, "1.0.0", "linux-x86_64").is_err());
    }

    #[test]
    fn a_version_that_is_not_semver_is_an_error() {
        let manifest = parse_manifest(MANIFEST).unwrap();
        assert!(pick(&manifest, "1.0", "macos-aarch64").is_err());
    }
}
