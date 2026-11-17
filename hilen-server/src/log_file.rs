//! The log file a backend writes next to stdout on every start. A backend
//! that runs as one binary on a home machine loses every stdout line when
//! its terminal closes, so the file is its only log.
//!
//! The folders, the file name and the count of kept files match
//! `hilen/src/log_file.rs`, so an app and a backend log the same way. The
//! code exists twice because this crate never links the `hilen` UI crate.

use std::{
    env::var_os,
    ffi::OsStr,
    fs::{File, OpenOptions, create_dir_all, read_dir, remove_file},
    path::{Path, PathBuf},
    sync::OnceLock,
};

use anyhow::{Context, Result, bail};
use chrono::Local;

/// Set to a folder, the log file goes there instead of the platform folder.
pub const LOG_DIR_ENV: &str = "HILEN_LOG_DIR";

/// Set to `off`, `0`, `false` or `no`, no log file is written. A backend in
/// Docker keeps its lines in `docker logs`.
pub const LOG_FILE_ENV: &str = "HILEN_LOG_FILE";

/// Starts older than this many are removed, newest first by file name,
/// which sorts by date because of the name format.
const KEEP: usize = 10;

static CURRENT: OnceLock<PathBuf> = OnceLock::new();

/// The log file of this start, `None` before `tracing_init::init`, when the
/// file is turned off or when it could not be created.
pub fn log_file_path() -> Option<PathBuf> {
    CURRENT.get().cloned()
}

/// Where the platform expects a program to keep its logs.
pub fn log_dir(name: &str) -> Result<PathBuf> {
    if cfg!(target_os = "macos") {
        let home = dirs::home_dir().context("no home dir")?;
        return Ok(home.join("Library").join("Logs").join(name));
    }
    if cfg!(windows) {
        let local = dirs::data_local_dir().context("no local app data dir")?;
        return Ok(local.join(name).join("logs"));
    }
    let state = dirs::state_dir().or_else(dirs::data_local_dir).context("no state dir")?;
    Ok(state.join(name).join("logs"))
}

#[derive(Debug, PartialEq)]
enum Target {
    Off,
    Dir(PathBuf),
    Platform,
}

impl Target {
    fn from_env() -> Self {
        Self::parse(var_os(LOG_FILE_ENV).as_deref(), var_os(LOG_DIR_ENV).as_deref())
    }

    fn parse(file: Option<&OsStr>, dir: Option<&OsStr>) -> Self {
        let off = file.and_then(OsStr::to_str).is_some_and(|value| {
            matches!(value.trim().to_lowercase().as_str(), "off" | "0" | "false" | "no")
        });
        if off {
            return Self::Off;
        }
        match dir {
            Some(dir) if !dir.is_empty() => Self::Dir(PathBuf::from(dir)),
            _ => Self::Platform,
        }
    }
}

/// Opens the file of this start where the env asks for it, `None` when the
/// env turns the file off.
pub(crate) fn open(name: &str) -> Result<Option<(File, PathBuf)>> {
    let dir = match Target::from_env() {
        Target::Off => return Ok(None),
        Target::Dir(dir) => dir,
        Target::Platform => log_dir(name)?,
    };
    let (file, path) = create_in(&dir, name)?;
    if CURRENT.set(path.clone()).is_err() {
        bail!("log file already chosen");
    }
    Ok(Some((file, path)))
}

/// Creates the folder, removes old files and opens the file of this start.
fn create_in(dir: &Path, name: &str) -> Result<(File, PathBuf)> {
    create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    prune(dir, name, KEEP)?;
    let path = dir.join(format!("{name}-{}.log", Local::now().format("%Y-%m-%d_%H-%M-%S")));
    // Append, so 2 starts within one second share a file instead of the
    // second one wiping the lines of the first.
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("open {}", path.display()))?;
    Ok((file, path))
}

/// Keeps the `keep` newest log files of `name` in `dir`, by file name.
fn prune(dir: &Path, name: &str, keep: usize) -> Result<()> {
    let prefix = format!("{name}-");
    let mut logs: Vec<PathBuf> = read_dir(dir)
        .with_context(|| format!("read {}", dir.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().is_some_and(|ext| ext == "log")
                && path
                    .file_name()
                    .and_then(|file| file.to_str())
                    .is_some_and(|file| file.starts_with(&prefix))
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
        ffi::OsStr,
        fs::{create_dir_all, read_dir, remove_dir_all, write},
        path::{Path, PathBuf},
        process::id,
    };

    use anyhow::Result;

    use super::{KEEP, Target, create_in};

    fn fresh_dir(test: &str) -> PathBuf {
        temp_dir().join(format!("hilen-server-log-{test}-{}", id()))
    }

    fn names(dir: &Path) -> Result<Vec<String>> {
        let mut names: Vec<String> = read_dir(dir)?
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .collect();
        names.sort();
        Ok(names)
    }

    #[test]
    fn a_start_creates_the_folder_and_a_dated_file() -> Result<()> {
        let dir = fresh_dir("create").join("nested");

        let (_file, path) = create_in(&dir, "backend")?;

        let found = names(&dir)?;
        remove_dir_all(&dir)?;

        assert_eq!(path.parent(), Some(dir.as_path()));
        assert_eq!(found.len(), 1);
        // backend-2026-01-31_10-00-00.log
        let name = &found[0];
        assert!(name.starts_with("backend-"), "{name}");
        assert!(path.extension().is_some_and(|ext| ext == "log"), "{name}");
        assert_eq!(
            name.len(),
            "backend-".len() + "2026-01-31_10-00-00".len() + ".log".len(),
            "{name}"
        );
        Ok(())
    }

    #[test]
    fn a_start_keeps_the_newest_files_and_other_files() -> Result<()> {
        let dir = fresh_dir("prune");
        create_dir_all(&dir)?;
        for day in 1..=12 {
            write(dir.join(format!("backend-2020-01-{day:02}_10-00-00.log")), "")?;
        }
        write(dir.join("other-2020-01-01_10-00-00.log"), "")?;
        write(dir.join("backend-notes.txt"), "")?;

        create_in(&dir, "backend")?;

        let found = names(&dir)?;
        remove_dir_all(&dir)?;

        // 9 old logs and the new one stay, files of another program and of
        // another kind are untouched.
        let logs = found.iter().filter(|name| name.starts_with("backend-2")).count();
        assert_eq!(logs, KEEP);
        assert_eq!(found.len(), KEEP + 2);
        assert!(!found.contains(&"backend-2020-01-03_10-00-00.log".to_string()));
        assert!(found.contains(&"backend-2020-01-04_10-00-00.log".to_string()));
        assert!(found.contains(&"backend-2020-01-12_10-00-00.log".to_string()));
        assert!(found.contains(&"other-2020-01-01_10-00-00.log".to_string()));
        assert!(found.contains(&"backend-notes.txt".to_string()));
        Ok(())
    }

    #[test]
    fn the_env_picks_off_a_folder_or_the_platform_folder() {
        let dir = Some(OsStr::new("/tmp/logs"));

        assert_eq!(Target::parse(None, None), Target::Platform);
        assert_eq!(Target::parse(None, Some(OsStr::new(""))), Target::Platform);
        assert_eq!(Target::parse(None, dir), Target::Dir(PathBuf::from("/tmp/logs")));
        assert_eq!(
            Target::parse(Some(OsStr::new("on")), dir),
            Target::Dir(PathBuf::from("/tmp/logs"))
        );
        for off in ["off", "OFF", "0", "false", "no", " off "] {
            assert_eq!(Target::parse(Some(OsStr::new(off)), dir), Target::Off, "{off}");
        }
    }
}
