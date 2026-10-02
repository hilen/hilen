//! The assets an app packed into its binary with `embed_assets!`. An
//! installed desktop app has no `assets` folder next to it, and its updater
//! swaps only the executable, so its images, fonts, sounds and models have
//! to live in the binary. A file on disk always wins, a dev run from the
//! repo never reads the packed copy.

use std::{
    collections::HashMap,
    path::{Component, Path},
    sync::OnceLock,
};

use crate::app::hilen_embedded_assets;

static BY_NAME: OnceLock<HashMap<&'static str, &'static [u8]>> = OnceLock::new();

fn by_name() -> &'static HashMap<&'static str, &'static [u8]> {
    BY_NAME.get_or_init(|| hilen_embedded_assets().iter().copied().collect())
}

/// How many files the app packed, 0 for an app with no `embed_assets!`.
pub(crate) fn count() -> usize {
    by_name().len()
}

/// The packed bytes of the file at `path`, when it sits in `assets_dir` and
/// the app packed it.
pub(crate) fn read(assets_dir: &Path, path: &Path) -> Option<&'static [u8]> {
    by_name().get(name(assets_dir, path)?.as_str()).copied()
}

/// The name a file is packed under: its path below the assets folder with
/// `/` between the parts, whatever the system joins paths with. `None` for a
/// path outside the folder.
fn name(assets_dir: &Path, path: &Path) -> Option<String> {
    let below = path.strip_prefix(assets_dir).ok()?;
    let parts: Option<Vec<&str>> = below
        .components()
        .map(|part| match part {
            Component::Normal(part) => part.to_str(),
            _ => None,
        })
        .collect();
    let parts = parts.filter(|parts| !parts.is_empty())?;
    Some(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::name;

    #[test]
    fn a_file_in_the_assets_folder_gets_its_name_below_it() {
        let assets = Path::new("/apps/flixen/assets");

        assert_eq!(
            name(assets, &assets.join("images").join("play.svg")).as_deref(),
            Some("images/play.svg")
        );
        assert_eq!(
            name(assets, &assets.join("images").join("prince/01a.png")).as_deref(),
            Some("images/prince/01a.png")
        );
    }

    /// An installed app found no folder, its root is empty and the assets
    /// folder is the bare relative `assets`.
    #[test]
    fn the_name_is_the_same_with_no_assets_folder_on_disk() {
        let assets = PathBuf::default().join("assets");

        assert_eq!(
            name(&assets, &assets.join("fonts").join("Inter.ttf")).as_deref(),
            Some("fonts/Inter.ttf")
        );
    }

    #[test]
    fn a_path_outside_the_assets_folder_has_no_name() {
        let assets = Path::new("/apps/flixen/assets");

        assert_eq!(name(assets, Path::new("/apps/flixen/Cargo.toml")), None);
        assert_eq!(name(assets, Path::new("/tmp/poster.png")), None);
        assert_eq!(name(assets, assets), None);
        assert_eq!(name(assets, &assets.join("images/../../secret")), None);
    }
}
