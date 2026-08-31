//! The log file every launch writes next to stdout. A GUI build on Windows
//! has no console and a Finder or dock launch on mac keeps no terminal, so
//! stdout alone loses every line the moment something goes wrong.

use std::{
    env::current_exe,
    fs::{create_dir_all, read_dir, remove_file},
    path::PathBuf,
    sync::OnceLock,
};

use anyhow::{Context, Result, bail};
use chrono::Local;

/// Launches older than this many are removed, newest first by file name,
/// which sorts by date because of the name format.
const KEEP: usize = 10;

static CURRENT: OnceLock<PathBuf> = OnceLock::new();

/// The log file of this launch, `None` before logging is set up or when
/// the file could not be created.
pub fn log_file_path() -> Option<PathBuf> {
    CURRENT.get().cloned()
}

/// Picks the file for this launch, creates its dir and prunes old ones.
pub(crate) fn create() -> Result<PathBuf> {
    let app = app_name()?;
    let dir = log_dir(&app)?;
    create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    prune(&dir, &app, KEEP)?;
    let path = dir.join(format!("{app}-{}.log", Local::now().format("%Y-%m-%d_%H-%M-%S")));
    if CURRENT.set(path.clone()).is_err() {
        bail!("log file already chosen");
    }
    Ok(path)
}

fn app_name() -> Result<String> {
    let exe = current_exe().context("current exe")?;
    let Some(stem) = exe.file_stem().and_then(|s| s.to_str()) else {
        bail!("exe {} has no name", exe.display());
    };
    Ok(stem.to_string())
}

/// Where the platform expects an app to keep its logs.
pub fn log_dir(app: &str) -> Result<PathBuf> {
    if cfg!(target_os = "macos") {
        let home = dirs::home_dir().context("no home dir")?;
        return Ok(home.join("Library").join("Logs").join(app));
    }
    if cfg!(windows) {
        let local = dirs::data_local_dir().context("no local app data dir")?;
        return Ok(local.join(app).join("logs"));
    }
    let state = dirs::state_dir().or_else(dirs::data_local_dir).context("no state dir")?;
    Ok(state.join(app).join("logs"))
}

/// Keeps the `keep` newest log files of `app` in `dir`, by name.
fn prune(dir: &PathBuf, app: &str, keep: usize) -> Result<()> {
    let prefix = format!("{app}-");
    let mut logs: Vec<PathBuf> = read_dir(dir)
        .with_context(|| format!("read {}", dir.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().is_some_and(|ext| ext == "log")
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(&prefix))
        })
        .collect();
    logs.sort();
    // The new file is not written yet, so one more slot goes to it.
    let old = logs.len().saturating_sub(keep.saturating_sub(1));
    for path in logs.into_iter().take(old) {
        remove_file(&path).with_context(|| format!("remove {}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        env::temp_dir,
        fs::{create_dir_all, read_dir, remove_dir_all, write},
        process::id,
    };

    use anyhow::Result;

    use super::prune;

    #[test]
    fn prune_keeps_the_newest_and_other_files() -> Result<()> {
        let dir = temp_dir().join(format!("hilen-log-prune-{}", id()));
        create_dir_all(&dir)?;
        for day in 1..=12 {
            write(dir.join(format!("app-2026-01-{day:02}_10-00-00.log")), "")?;
        }
        write(dir.join("other-2026-01-01_10-00-00.log"), "")?;
        write(dir.join("app-notes.txt"), "")?;

        prune(&dir, "app", 10)?;

        let mut names: Vec<String> = read_dir(&dir)?
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .collect();
        names.sort();
        remove_dir_all(&dir)?;

        // 9 newest app logs stay, the slot for the launch about to write is
        // free, and files of other apps or other kinds are untouched.
        assert_eq!(names.len(), 11);
        assert!(!names.contains(&"app-2026-01-01_10-00-00.log".to_string()));
        assert!(!names.contains(&"app-2026-01-03_10-00-00.log".to_string()));
        assert!(names.contains(&"app-2026-01-04_10-00-00.log".to_string()));
        assert!(names.contains(&"app-2026-01-12_10-00-00.log".to_string()));
        assert!(names.contains(&"other-2026-01-01_10-00-00.log".to_string()));
        assert!(names.contains(&"app-notes.txt".to_string()));
        Ok(())
    }
}
