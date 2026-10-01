#[cfg(desktop)]
use std::env::temp_dir;
use std::path::PathBuf;

use parking_lot::Mutex;
use plat::Platform;

#[cfg(desktop)]
use crate::app::hilen_project_name;

static STORAGE_PATH: Mutex<Option<String>> = Mutex::new(None);

/// A test run stores here and never in the data folder of a real app.
#[cfg(desktop)]
static TEST_STORAGE: Mutex<Option<PathBuf>> = Mutex::new(None);

pub struct Paths;

impl Paths {
    pub(crate) fn home() -> PathBuf {
        if Platform::IOS {
            dirs::document_dir()
        } else if Platform::ANDROID {
            STORAGE_PATH.lock().clone().map(PathBuf::from)
        } else {
            dirs::home_dir()
        }
        .expect("Failed to get home directory")
    }

    pub fn config() -> PathBuf {
        Self::home().join(".config")
    }

    /// The data folder of the app. On desktop `~/.config/<project_name>`,
    /// with the name from the app's `hilen.toml`, so every binary of one
    /// project shares one folder. The engine creates it and makes it the
    /// `OnDisk` root before `before_launch` runs.
    pub fn storage() -> PathBuf {
        #[cfg(wasm)]
        {
            PathBuf::default()
        }
        #[cfg(mobile)]
        {
            format!("{}/.{}", Self::home().display(), Self::executable_name()).into()
        }
        #[cfg(desktop)]
        {
            let test = TEST_STORAGE.lock().clone();
            test.unwrap_or_else(|| Self::config().join(hilen_project_name()))
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
