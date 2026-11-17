use parking_lot::Mutex;

use crate::inspect::protocol::EditEntry;

static EDITS: Mutex<Vec<EditEntry>> = Mutex::new(Vec::new());

pub(crate) fn record(entry: EditEntry) {
    #[cfg(not_wasm)]
    if let Err(err) = file_trail::append(&entry) {
        log::error!("Failed to write inspect edit log: {err}");
    }
    EDITS.lock().push(entry);
}

pub fn all() -> Vec<EditEntry> {
    EDITS.lock().clone()
}

// A browser has no filesystem, the in-memory list is the whole log there.
#[cfg(not_wasm)]
mod file_trail {
    use std::{
        env::current_exe,
        fs::{OpenOptions, create_dir_all},
        io::Write,
        path::{Path, PathBuf},
        sync::LazyLock,
    };

    use anyhow::Result;

    use crate::inspect::protocol::EditEntry;

    // An exe outside a `target` folder is an installed app or a device build.
    // The in-memory list still works, only the file trail is skipped.
    static LOG_PATH: LazyLock<Option<PathBuf>> =
        LazyLock::new(|| Some(target_dir(&current_exe().ok()?)?.join("inspect-edits.jsonl")));

    fn target_dir(exe: &Path) -> Option<&Path> {
        exe.ancestors().find(|dir| dir.file_name().is_some_and(|name| name == "target"))
    }

    pub(super) fn append(entry: &EditEntry) -> Result<()> {
        let Some(path) = LOG_PATH.as_ref() else {
            return Ok(());
        };

        if let Some(dir) = path.parent() {
            create_dir_all(dir)?;
        }

        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        writeln!(file, "{}", serde_json::to_string(entry)?)?;

        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use std::path::Path;

        use super::target_dir;

        #[test]
        fn dev_build_logs_into_its_target_folder() {
            let exe = Path::new("/repo/target/aarch64-apple-darwin/debug/demo");
            assert_eq!(target_dir(exe), Some(Path::new("/repo/target")));
        }

        #[test]
        fn installed_app_has_no_file_trail() {
            assert_eq!(
                target_dir(Path::new("/Applications/Demo.app/Contents/MacOS/demo")),
                None
            );
        }
    }
}
