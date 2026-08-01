#[macro_export]
macro_rules! managed {
    ($($refs_path:tt)::+, $type:ident) => {
        static __MANAGED_ROOT_PATH: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
        static __STORAGE: $($refs_path)::+::__internal_deps::RwLock<$($refs_path)::+::manage::DataStorage<$type>> =
            $($refs_path)::+::__internal_deps::RwLock::new(std::collections::BTreeMap::new());
        static __IN_FLIGHT_DOWNLOADS: $($refs_path)::+::manage::InFlightDownloads =
            $($refs_path)::+::__internal_deps::Mutex::new(std::collections::BTreeMap::new());

        impl $($refs_path)::+::manage::Managed for $type {}

        impl $($refs_path)::+::manage::DataManager<$type> for $type {
            fn root_path() -> &'static std::path::Path {
                static DEFAULT_PATH: std::path::PathBuf = std::path::PathBuf::new();

                if let Some(path) = __MANAGED_ROOT_PATH.get() {
                    return path;
                }

                $($refs_path)::+::__internal_deps::warn!("Managed root path for type {} is not set.", stringify!($type));
                &DEFAULT_PATH
            }

            fn set_root_path(path: impl Into<std::path::PathBuf>) {
                let path = path.into();
                __MANAGED_ROOT_PATH.set(path.to_path_buf()).expect(&format!(
                    "Managed root path for type {} was already set set.",
                    stringify!($type)
                ))
            }

            fn storage() -> $($refs_path)::+::__internal_deps::RwLockReadGuard<'static, $($refs_path)::+::manage::DataStorage<$type>> {
                __STORAGE.read()
            }

            fn storage_mut() -> $($refs_path)::+::__internal_deps::RwLockWriteGuard<'static, $($refs_path)::+::manage::DataStorage<$type>> {
                __STORAGE.write()
            }

            fn in_flight_downloads() -> &'static $($refs_path)::+::manage::InFlightDownloads {
                &__IN_FLIGHT_DOWNLOADS
            }
        }
    };

    ($type:ident) => {
        managed!(refs, $type);
    };
}
