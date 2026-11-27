//! Shows a file or a folder in the file manager of the system: Finder on a
//! Mac, Explorer on Windows, the file manager of the desktop on Linux. A
//! phone and a browser have no such file manager.

use std::path::Path;

use anyhow::{Result, anyhow};

/// Opens the folder of `path` in the file manager with `path` picked in
/// it. A path that is not there is an error on every platform, the file
/// manager of Linux would open some other folder for it. On a phone and in
/// a browser it is always an error.
pub fn show_in_file_manager(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    if !path.exists() {
        log::warn!("no {} to show in the file manager", path.display());
        return Err(anyhow!("{} is not there", path.display()));
    }
    reveal(path)
}

#[cfg(desktop)]
fn reveal(path: &Path) -> Result<()> {
    opener::reveal(path)
        .map_err(|err| anyhow!("Failed to show {} in the file manager: {err}", path.display()))?;
    log::info!("showed {} in the file manager", path.display());
    Ok(())
}

#[cfg(not(desktop))]
fn reveal(path: &Path) -> Result<()> {
    log::warn!("no file manager to show {} in on this platform", path.display());
    Err(anyhow!(
        "This platform has no file manager to show {} in",
        path.display()
    ))
}

#[cfg(test)]
mod tests {
    use std::env::temp_dir;

    use super::show_in_file_manager;

    #[test]
    fn a_missing_path_is_an_error() {
        let missing = temp_dir().join("hilen-no-such-file-for-the-file-manager");
        assert!(!missing.exists());
        let err = show_in_file_manager(&missing).unwrap_err();
        assert!(err.to_string().contains("is not there"), "{err}");
    }
}
