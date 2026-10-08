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

pub(crate) struct AssetsPaths {
    pub(crate) images: PathBuf,
    #[cfg(feature = "audio")]
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
            #[cfg(feature = "audio")]
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
        return var_os(HOT_ROOT).map_or_else(Self::bundle, PathBuf::from);
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
