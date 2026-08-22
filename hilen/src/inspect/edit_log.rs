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
        fs::{OpenOptions, create_dir_all},
        io::Write,
        path::PathBuf,
        sync::LazyLock,
    };

    use anyhow::Result;

    use crate::{filesystem::Paths, inspect::protocol::EditEntry};

    // No git root means the app runs outside a repo, on a device for example.
    // The in-memory list still works, only the file trail is skipped.
    static LOG_PATH: LazyLock<Option<PathBuf>> =
        LazyLock::new(|| Some(Paths::git_root().ok()?.join("target/inspect-edits.jsonl")));

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
}
