#[cfg(desktop)]
use std::env::{temp_dir, var_os};
use std::path::PathBuf;

use parking_lot::Mutex;
use plat::Platform;

#[cfg(any(desktop, hot))]
use crate::app::hilen_project_name;

static STORAGE_PATH: Mutex<Option<String>> = Mutex::new(None);

/// A test run stores here and never in the data folder of a real app.
#[cfg(desktop)]
static TEST_STORAGE: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Points the data folder of a desktop app at another folder for 1 run. A
/// check of a running app sets it to a copy of the data, so a tap that
/// deletes or syncs something never reaches the real folder or the real
/// account. Only the data folder moves, the log folder stays.
#[cfg(desktop)]
pub const DATA_DIR_VAR: &str = "HILEN_DATA_DIR";

// A test run wins over the env, it must never store in a folder a person
// named. An empty value is no value.
#[cfg(desktop)]
fn desktop_storage(
    test: Option<PathBuf>,
    asked: Option<PathBuf>,
    default: impl FnOnce() -> PathBuf,
) -> PathBuf {
    test.or_else(|| asked.filter(|path| !path.as_os_str().is_empty()))
        .unwrap_or_else(default)
}

pub struct Paths;

impl Paths {
    pub(crate) fn home() -> PathBuf {
        if Platform::IOS {
            dirs::document_dir()
        } else if Platform::ANDROID {
            Self::android_home()
        } else {
            dirs::home_dir()
        }
        .expect("Failed to get home directory")
    }

    /// The path the shell crate set, else the private files folder the
    /// system gives the app, so a shell with no Java hook stores too.
    #[cfg(android)]
    fn android_home() -> Option<PathBuf> {
        let set = STORAGE_PATH.lock().clone().map(PathBuf::from);
        set.or_else(crate::filesystem::read::android_data_path)
    }

    #[cfg(not(android))]
    fn android_home() -> Option<PathBuf> {
        STORAGE_PATH.lock().clone().map(PathBuf::from)
    }

    pub fn config() -> PathBuf {
        Self::home().join(".config")
    }

    /// The data folder of the app. On desktop `~/.config/<project_name>`,
    /// with the name from the app's `hilen.toml`, so every binary of one
    /// project shares one folder. On a phone a folder inside the documents
    /// dir on iOS and inside the private files dir on Android. The engine
    /// creates it and makes it the `OnDisk` root before `before_launch` runs.
    pub fn storage() -> PathBuf {
        #[cfg(wasm)]
        {
            PathBuf::default()
        }
        #[cfg(all(mobile, not(hot)))]
        {
            format!("{}/.{}", Self::home().display(), Self::executable_name()).into()
        }
        // One loader runs several apps in a hot build, and the exe is the
        // loader for all of them.
        #[cfg(hot)]
        {
            format!("{}/.{}", Self::home().display(), hilen_project_name()).into()
        }
        #[cfg(desktop)]
        {
            let test = TEST_STORAGE.lock().clone();
            let asked = var_os(DATA_DIR_VAR).map(PathBuf::from);
            desktop_storage(test, asked, || Self::config().join(hilen_project_name()))
        }
    }

    /// Points `storage` at a temp folder. The test runners start an app of
    /// the engine itself, which has no project name and must not touch the
    /// folder of a real app.
    #[cfg(desktop)]
    pub(crate) fn use_test_storage() {
        *TEST_STORAGE.lock() = Some(temp_dir().join("hilen-test-storage"));
    }

    pub fn executable_name() -> String {
        std::env::current_exe()
            .expect("Failed to get std::env::current_exe()")
            .file_name()
            .expect("Failed to get executable name")
            .to_string_lossy()
            .into()
    }

    pub fn set_storage_path(path: String) {
        STORAGE_PATH.lock().replace(path);
    }

    /// Whether this run stores in the temp folder of the test runners.
    #[cfg(desktop)]
    pub(crate) fn test_storage() -> bool {
        TEST_STORAGE.lock().is_some()
    }

    /// A file dialog filtered to the extensions, the picked path or None.
    #[cfg(desktop)]
    pub async fn pick_file(title: &str, extensions: &[&str]) -> Option<PathBuf> {
        use rfd::AsyncFileDialog;

        let handle = AsyncFileDialog::new()
            .set_title(title)
            .add_filter(title, extensions)
            .pick_file()
            .await?;

        Some(handle.path().to_owned())
    }

    /// The API is async on every platform, here the answer is immediate.
    #[cfg(any(mobile, wasm))]
    pub async fn pick_file(title: &str, extensions: &[&str]) -> Option<PathBuf> {
        log::debug!("no file dialog on this platform for {title} {extensions:?}");
        std::future::ready(None).await
    }

    pub async fn pick_folder() -> Option<PathBuf> {
        #[cfg(desktop)]
        {
            use rfd::AsyncFileDialog;

            let handle = AsyncFileDialog::new()
                .set_title("Select Directory")
                .set_directory("~")
                .pick_folder()
                .await?;

            Some(handle.path().to_owned())
        }
        // The API is async on every platform, here the answer is immediate.
        #[cfg(any(mobile, wasm))]
        std::future::ready(None).await
    }
}

#[cfg(all(test, desktop))]
mod tests {
    use std::path::PathBuf;

    use super::desktop_storage;

    fn folder(test: Option<&str>, asked: Option<&str>) -> PathBuf {
        desktop_storage(test.map(PathBuf::from), asked.map(PathBuf::from), || {
            PathBuf::from("/home/.config/app")
        })
    }

    #[test]
    fn the_env_moves_the_data_folder() {
        assert_eq!(folder(None, None), PathBuf::from("/home/.config/app"));
        assert_eq!(folder(None, Some("/tmp/copy")), PathBuf::from("/tmp/copy"));
        assert_eq!(folder(None, Some("")), PathBuf::from("/home/.config/app"));
    }

    // A test run must never store in a folder a person named.
    #[test]
    fn a_test_run_wins_over_the_env() {
        assert_eq!(
            folder(Some("/tmp/test"), Some("/tmp/copy")),
            PathBuf::from("/tmp/test")
        );
    }
}
