//! Self update of a desktop binary, with no UI dependency, so a daemon or a
//! helper can update itself the same way a `hilen` app does. `check` fetches
//! the JSON manifest, `install` downloads the artifact of this platform,
//! verifies its size, checksum and ed25519 signature, then swaps the running
//! executable, and `relaunch` starts the new binary. The calls exist only on
//! macOS, Linux and Windows. The types are on every target.

use std::collections::BTreeMap;

use serde::Deserialize;

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
mod net;
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
mod platform;
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
mod update;
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
mod verify;

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
pub use platform::platform_key;
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
pub use update::{check, install, install_with_progress, relaunch, start_new_binary};

/// The download server a `hilen` app updates from unless it names another
/// address. The manifest of a project is `<host>/<project_name>/updater.json`.
pub const DEFAULT_UPDATE_HOST: &str = "https://get.vladas.xyz";

/// Where to look for a new version and how to trust it. The verify key is
/// the hex encoded ed25519 public key whose private half signs release
/// artifacts in CI, embedded in the binary so a compromised host can not
/// serve a forged one.
#[derive(Debug)]
pub struct UpdateSource {
    pub manifest_url:    String,
    pub current_version: String,
    pub verify_key:      String,
}

impl UpdateSource {
    pub fn new(
        manifest_url: impl Into<String>,
        current_version: impl Into<String>,
        verify_key: impl AsRef<str>,
    ) -> Self {
        Self {
            manifest_url:    manifest_url.into(),
            current_version: current_version.into(),
            // A key read with `include_str!` ends with a line break.
            verify_key:      verify_key.as_ref().trim().to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateManifest {
    pub version:   String,
    #[serde(default)]
    pub notes:     String,
    pub platforms: BTreeMap<String, UpdateArtifact>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UpdateArtifact {
    pub url:    String,
    pub size:   u64,
    pub sha256: String,
    pub sig:    String,
}

/// An available update, returned by `check` and consumed by `install`.
#[derive(Debug)]
pub struct UpdateInfo {
    pub version:    String,
    pub notes:      String,
    pub artifact:   UpdateArtifact,
    pub verify_key: String,
}

#[cfg(test)]
mod tests {
    use super::UpdateSource;

    #[test]
    fn a_source_trims_the_line_break_of_an_included_key() {
        let source = UpdateSource::new("https://example.com/updater.json", "0.1.0", "abc\n");

        assert_eq!(source.manifest_url, "https://example.com/updater.json");
        assert_eq!(source.current_version, "0.1.0");
        assert_eq!(source.verify_key, "abc");
    }
}
