use std::path::PathBuf;

use parking_lot::Mutex;
use plat::Platform;

static STORAGE_PATH: Mutex<Option<String>> = Mutex::new(None);

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

    pub fn storage() -> PathBuf {
        #[cfg(wasm)]
        {
            PathBuf::default()
        }
        #[cfg(not_wasm)]
        {
            format!("{}/.{}", Self::home().display(), Self::executable_name()).into()
        }
    }

    pub fn executable_name() -> String {
        std::env::current_exe()
            .expect("Failed to get std::env::current_exe()")
            .file_name()
            .expect("Failed to get executable name")
            .to_string_lossy()
            .into()
    }

    // Desktop always uses it for asset discovery. Mobile needs it only for
    // the inspect edit log.
    #[cfg(any(desktop, all(not_wasm, feature = "inspect")))]
    pub(crate) fn git_root() -> anyhow::Result<PathBuf> {
        #[cfg(wasm)]
        {
            Ok(PathBuf::new())
        }
        #[cfg(not_wasm)]
        {
            let output = std::process::Command::new("git")
                .args(["rev-parse", "--show-toplevel"])
                .output()?;

            if !output.status.success() {
                log::warn!("Failed to get Git repository root path");
                return Ok(PathBuf::from("~/dev/money"));
            }

            assert!(output.status.success(), "Failed to get Git repository root path");
            let git_root = String::from_utf8_lossy(&output.stdout).trim_end_matches('\n').to_string();

            Ok(PathBuf::from(git_root))
        }
    }

    pub fn set_storage_path(path: String) {
        STORAGE_PATH.lock().replace(path);
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
        #[cfg(any(mobile, wasm))]
        {
            None
        }
    }
}
