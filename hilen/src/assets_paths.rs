#![cfg(not(target_arch = "wasm32"))]

#[cfg(ios)]
use std::env::current_exe;
#[cfg(hot)]
use std::env::var_os;
use std::{
    path::{Path, PathBuf},
    rc::Rc,
};

/// The loader of a hot build sets it to the repo of the app it starts, see
/// `docs/hot-reload.md`. One loader runs several apps, so the assets of an
/// app are not in the bundle of the loader.
#[cfg(hot)]
const HOT_ROOT: &str = "HILEN_HOT_ROOT";
/// The loader sets it, the folder a sent library and its assets are in.
#[cfg(hot)]
const HOT_DIR: &str = "HILEN_HOT_DIR";

pub(crate) struct AssetsPaths {
    pub(crate) images: PathBuf,
    #[cfg(sound)]
    pub(crate) sounds: PathBuf,
    pub(crate) fonts:  PathBuf,
    #[cfg(feature = "scene")]
    pub(crate) models: PathBuf,
}

impl AssetsPaths {
    pub fn new(root: PathBuf) -> Rc<Self> {
        let root = Self::root(&root);
        let assets = Self::assets(&root);
        Rc::new(Self {
            images:                           assets.join("images"),
            #[cfg(sound)]
            sounds:                           assets.join("sounds"),
            fonts:                            assets.join("fonts"),
            #[cfg(feature = "scene")]
            models:                           assets.join("models"),
        })
    }
}

#[allow(clippy::used_underscore_binding)]
impl AssetsPaths {
    fn root(_base: &Path) -> PathBuf {
        #[cfg(hot)]
        return var_os(HOT_ROOT).map_or_else(Self::bundle, |root| {
            anchored(PathBuf::from(root), var_os(HOT_DIR).map(PathBuf::from))
        });
        #[cfg(all(ios, not(hot)))]
        return Self::bundle();
        #[cfg(not_ios)]
        return _base.into();
    }

    #[cfg(ios)]
    fn bundle() -> PathBuf {
        current_exe().unwrap_or_default().parent().unwrap().to_path_buf()
    }

    pub(crate) fn assets(_root: &Path) -> PathBuf {
        #[cfg(android)]
        return PathBuf::default();
        #[cfg(not_android)]
        return _root.join("assets");
    }
}

/// The root of an app that was sent to a device is kept as a path below
/// the hot folder, and is joined with the hot folder of this start. The
/// system gives an app a new data folder when it is installed again. A
/// full path kept from before then named a folder that was gone, and
/// every image of the app showed as the stand in picture. A full path is
/// the repo of the app on the Mac, in the simulator, and stays as it is.
#[cfg(any(hot, test))]
fn anchored(root: PathBuf, hot_dir: Option<PathBuf>) -> PathBuf {
    match hot_dir {
        Some(dir) if root.is_relative() => dir.join(root),
        _ => root,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::anchored;

    #[test]
    fn a_root_below_the_hot_folder_follows_the_hot_folder_of_this_start() {
        let dir = PathBuf::from("/data/NEW-ID/Library/Caches/hot");
        assert_eq!(
            anchored(PathBuf::from("apps/flixen"), Some(dir.clone())),
            PathBuf::from("/data/NEW-ID/Library/Caches/hot/apps/flixen")
        );
        // The repo of an app on the Mac, in the simulator.
        assert_eq!(
            anchored(PathBuf::from("/Users/me/dev/apps/flixen"), Some(dir)),
            PathBuf::from("/Users/me/dev/apps/flixen")
        );
        assert_eq!(
            anchored(PathBuf::from("apps/flixen"), None),
            PathBuf::from("apps/flixen")
        );
    }
}
