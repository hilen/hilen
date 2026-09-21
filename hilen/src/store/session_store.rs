use anyhow::Result;

use crate::store::{
    encrypt::{decrypt, encrypt},
    session_key::session_key,
};

/// The login session token of this app on this device, sealed with AES-256-GCM
/// under the key of `session_key.rs`. Nothing else in the engine or in an app
/// touches the stored bytes, so where they live can change behind these three
/// calls.
pub struct SessionStore;

impl SessionStore {
    /// `None` when nobody is logged in. Also when the stored bytes do not
    /// open, a file from another machine or from a build with another key,
    /// the user then logs in again.
    pub fn load() -> Option<String> {
        platform::read()
            .and_then(|sealed| sealed.map(|sealed| open(&sealed)).transpose())
            .unwrap_or_else(|error| {
                log::warn!("the stored session does not open: {error}");
                None
            })
    }

    pub fn save(token: &str) -> Result<()> {
        platform::write(&seal(token)?)
    }

    pub fn clear() -> Result<()> {
        platform::remove()
    }
}

fn seal(token: &str) -> Result<Vec<u8>> {
    encrypt(token.as_bytes(), &session_key())
}

fn open(sealed: &[u8]) -> Result<String> {
    Ok(String::from_utf8(decrypt(sealed, &session_key())?)?)
}

#[cfg(not_wasm)]
mod platform {
    use std::{
        fs::{create_dir_all, read as read_file, remove_file, write as write_file},
        io::ErrorKind,
        path::{Path, PathBuf},
    };

    use anyhow::{Context, Result};

    use crate::store::on_disk::rooted;

    fn path() -> PathBuf {
        rooted(Path::new("session.bin"))
    }

    pub(super) fn read() -> Result<Option<Vec<u8>>> {
        match read_file(path()) {
            Ok(sealed) => Ok(Some(sealed)),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error).context("failed to read the session file"),
        }
    }

    pub(super) fn write(sealed: &[u8]) -> Result<()> {
        let path = path();
        if let Some(parent) = path.parent() {
            create_dir_all(parent).context("failed to create the session folder")?;
        }
        write_file(path, sealed).context("failed to write the session file")
    }

    pub(super) fn remove() -> Result<()> {
        match remove_file(path()) {
            Err(error) if error.kind() != ErrorKind::NotFound => {
                Err(error).context("failed to remove the session file")
            }
            _ => Ok(()),
        }
    }
}

/// A browser has no files. `localStorage` is per origin, other sites cannot
/// read it.
#[cfg(wasm)]
mod platform {
    use anyhow::{Result, anyhow};
    use base64::{Engine, engine::general_purpose::STANDARD};
    use web_sys::{Storage, window};

    const KEY: &str = "hilen-session";

    fn storage() -> Result<Storage> {
        window()
            .ok_or_else(|| anyhow!("no window"))?
            .local_storage()
            .map_err(|error| anyhow!("localStorage is blocked: {error:?}"))?
            .ok_or_else(|| anyhow!("no localStorage"))
    }

    pub(super) fn read() -> Result<Option<Vec<u8>>> {
        let stored = storage()?
            .get_item(KEY)
            .map_err(|error| anyhow!("failed to read localStorage: {error:?}"))?;
        Ok(stored.map(|text| STANDARD.decode(text)).transpose()?)
    }

    pub(super) fn write(sealed: &[u8]) -> Result<()> {
        storage()?
            .set_item(KEY, &STANDARD.encode(sealed))
            .map_err(|error| anyhow!("failed to write localStorage: {error:?}"))
    }

    pub(super) fn remove() -> Result<()> {
        storage()?
            .remove_item(KEY)
            .map_err(|error| anyhow!("failed to clear localStorage: {error:?}"))
    }
}

#[cfg(all(test, not_wasm))]
mod test {
    use anyhow::Result;
    use serial_test::serial;

    use super::SessionStore;
    use crate::store::OnDisk;

    /// The root is one global shared with the `OnDisk` test, which sets the
    /// same folder, so the two never pull it apart.
    fn use_test_root() {
        OnDisk::<()>::set_root_path("~/.test_on_disk/");
    }

    #[test]
    #[serial]
    fn save_load_clear() -> Result<()> {
        use_test_root();

        SessionStore::clear()?;
        assert_eq!(SessionStore::load(), None);

        SessionStore::save("token one")?;
        assert_eq!(SessionStore::load().as_deref(), Some("token one"));

        SessionStore::save("token two")?;
        assert_eq!(SessionStore::load().as_deref(), Some("token two"));

        SessionStore::clear()?;
        assert_eq!(SessionStore::load(), None);
        // Clearing twice is fine, logout must not fail on a missing file.
        SessionStore::clear()
    }

    #[test]
    #[serial]
    fn token_is_not_on_disk_as_text() -> Result<()> {
        use_test_root();

        SessionStore::save("a very recognizable token")?;
        let bytes = super::platform::read()?.expect("the file was just written");
        assert!(!String::from_utf8_lossy(&bytes).contains("recognizable"));

        SessionStore::clear()
    }

    #[test]
    #[serial]
    fn broken_file_loads_as_none() -> Result<()> {
        use_test_root();

        super::platform::write(b"not sealed by this key at all, only some text")?;
        assert_eq!(SessionStore::load(), None);

        SessionStore::clear()
    }
}
