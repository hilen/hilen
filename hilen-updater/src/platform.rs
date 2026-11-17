//! Which manifest entry this binary takes and which file the swap replaces.

#[cfg(target_os = "linux")]
use std::env::var_os;
use std::{
    env::{
        consts::{ARCH, OS},
        current_exe,
    },
    fs::{OpenOptions, metadata, remove_file, rename, set_permissions},
    path::{Path, PathBuf},
    process,
};

use anyhow::Result;
use log::warn;

/// The manifest key of this binary, `std::env::consts::OS` plus `-` plus
/// `ARCH`, like `macos-aarch64`, with `-appimage` added when it runs as an
/// `AppImage`.
pub fn platform_key() -> String {
    key(OS, ARCH, appimage_path().is_some())
}

// An AppImage swaps the whole image file, not the bare binary, so it gets
// its own manifest key.
fn key(os: &str, arch: &str, appimage: bool) -> String {
    if appimage {
        format!("{os}-{arch}-appimage")
    } else {
        format!("{os}-{arch}")
    }
}

/// The `AppImage` runtime exports the image path as `APPIMAGE`.
#[cfg(target_os = "linux")]
pub(crate) fn appimage_path() -> Option<PathBuf> {
    var_os("APPIMAGE").map(Into::into)
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn appimage_path() -> Option<PathBuf> {
    None
}

/// The file `install` replaces, the `AppImage` when running as one, the
/// executable itself otherwise.
pub(crate) fn swap_target() -> Result<PathBuf> {
    if let Some(image) = appimage_path() {
        return Ok(image);
    }
    Ok(current_exe()?)
}

// Both swap routes create a temp file in the dir and rename, so dir write
// access is what decides whether an update can install.
pub(crate) fn dir_writable(dir: &Path) -> bool {
    let probe = dir.join(format!(".hilen-update-probe-{}", process::id()));
    match OpenOptions::new().write(true).create_new(true).open(&probe) {
        Ok(file) => {
            drop(file);
            if let Err(err) = remove_file(&probe) {
                warn!("Failed to remove write probe {}: {err}", probe.display());
            }
            true
        }
        Err(_) => false,
    }
}

pub(crate) fn replace_keeping_permissions(temp: &Path, target: &Path) -> Result<()> {
    let permissions = metadata(target)?.permissions();
    set_permissions(temp, permissions)?;
    rename(temp, target)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        env::{
            consts::{ARCH, OS},
            temp_dir,
        },
        fs::{create_dir_all, read, remove_dir_all, write},
        process,
    };

    use super::{appimage_path, dir_writable, key, platform_key, replace_keeping_permissions};

    #[test]
    fn platform_keys() {
        assert_eq!(key("macos", "aarch64", false), "macos-aarch64");
        assert_eq!(key("windows", "x86_64", false), "windows-x86_64");
        assert_eq!(key("linux", "x86_64", false), "linux-x86_64");
        assert_eq!(key("linux", "x86_64", true), "linux-x86_64-appimage");
    }

    /// The key of the test binary itself, which runs as a bare file, never
    /// as an `AppImage`.
    #[test]
    fn this_binary_has_the_plain_key() {
        if appimage_path().is_none() {
            assert_eq!(platform_key(), format!("{OS}-{ARCH}"));
        }
    }

    #[test]
    fn writable_probe() {
        let dir = temp_dir().join(format!("hilen-writable-{}", process::id()));
        create_dir_all(&dir).unwrap();
        assert!(dir_writable(&dir));
        // Root writes into a read only directory anyway, and the linux CI
        // containers run as root, so the negative half only holds for a
        // plain user.
        #[cfg(unix)]
        if unsafe { libc::geteuid() } != 0 {
            use std::{
                fs::{Permissions, set_permissions},
                os::unix::fs::PermissionsExt,
            };
            set_permissions(&dir, Permissions::from_mode(0o555)).unwrap();
            assert!(!dir_writable(&dir));
            set_permissions(&dir, Permissions::from_mode(0o755)).unwrap();
        }
        remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn replace_keeps_permissions() {
        let dir = temp_dir().join(format!("hilen-replace-{}", process::id()));
        create_dir_all(&dir).unwrap();
        let target = dir.join("app.AppImage");
        let temp = dir.join(".hilen-update-1.0.0");
        write(&target, b"old").unwrap();
        write(&temp, b"new").unwrap();
        #[cfg(unix)]
        {
            use std::{
                fs::{Permissions, set_permissions},
                os::unix::fs::PermissionsExt,
            };
            set_permissions(&target, Permissions::from_mode(0o755)).unwrap();
        }
        replace_keeping_permissions(&temp, &target).unwrap();
        assert_eq!(read(&target).unwrap(), b"new");
        assert!(!temp.exists());
        #[cfg(unix)]
        {
            use std::{fs::metadata, os::unix::fs::PermissionsExt};
            let mode = metadata(&target).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o755);
        }
        remove_dir_all(&dir).unwrap();
    }
}
