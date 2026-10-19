#[cfg(not_wasm)]
use std::path::Path;
use std::sync::{Mutex, PoisonError};

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};

#[cfg(feature = "keychain")]
use crate::SecretStore;
use crate::{
    encrypt::{decrypt, encrypt},
    session_key::session_key,
};

/// A logged in user, as the server knows them from Google or Apple.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginUser {
    pub id:      String,
    pub email:   String,
    pub name:    String,
    /// A link to the Google profile picture. An Apple account has none.
    pub picture: Option<String>,
}

/// 1 login on this device.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub token: String,
    /// `None` only for a session an engine with 1 token stored, until the app
    /// asks the server who it belongs to.
    pub user:  Option<LoginUser>,
}

/// What the file holds. `active` is an index into `list`, `None` when nobody
/// is logged in while other sessions are kept.
#[derive(Default, Serialize, Deserialize)]
struct Sessions {
    list:   Vec<Session>,
    active: Option<usize>,
}

impl Sessions {
    fn active(&self) -> Option<&Session> {
        self.active.and_then(|index| self.list.get(index))
    }

    fn index_of(&self, user_id: &str) -> Option<usize> {
        self.list
            .iter()
            .position(|session| session.user.as_ref().is_some_and(|user| user.id == user_id))
    }

    /// Takes a session out and keeps `active` on the same session, or on
    /// nobody when it was this one.
    fn remove(&mut self, index: usize) -> Session {
        self.active = match self.active {
            Some(active) if active == index => None,
            Some(active) if active > index => Some(active - 1),
            active => active,
        };
        self.list.remove(index)
    }
}

/// Every change reads the file and writes it back, 2 changes at once would
/// lose one of them.
static CHANGE: Mutex<()> = Mutex::new(());

/// The login sessions of this app on this device, several people can be
/// logged in at once and 1 of them is active.
///
/// With the `keychain` feature the list lives in the credential store of the
/// system through `SecretStore`, on iOS and tvOS as an entry that never
/// leaves the device. So a copy of the data folder or of a phone backup holds
/// no session. A sealed file of an older engine moves in at the first load
/// and is deleted, so an update logs nobody out. Without the feature, in a
/// browser, and where the system has no credential store, the list is a file
/// sealed with AES-256-GCM under the key of `session_key.rs`.
///
/// Nothing else in the engine or in an app touches the stored bytes, so where
/// they live can change behind these calls.
pub struct SessionStore;

impl SessionStore {
    /// The token of the active session. `None` when nobody is logged in. Also
    /// when the stored bytes do not open, a file from another machine or from
    /// a build with another key, the user then logs in again.
    pub fn load() -> Option<String> {
        read().active().map(|session| session.token.clone())
    }

    /// The user of the active session, `None` also while an old stored
    /// session has no user yet.
    pub fn active_user() -> Option<LoginUser> {
        read().active().and_then(|session| session.user.clone())
    }

    /// Every session on this device, in the order they were added.
    pub fn sessions() -> Vec<Session> {
        read().list
    }

    /// Keeps a new login and makes it active. A login of a user that is here
    /// already replaces the token of that session.
    pub fn add(token: &str, user: LoginUser) -> Result<()> {
        change(|sessions| {
            let known = sessions.index_of(&user.id);
            let session = Session {
                token: token.to_owned(),
                user:  Some(user),
            };
            let index = if let Some(index) = known {
                sessions.list[index] = session;
                index
            } else {
                sessions.list.push(session);
                sessions.list.len() - 1
            };
            sessions.active = Some(index);
            Ok(())
        })
    }

    /// Puts the user the server named for a token into its session, to fill
    /// in an old session or a new name or picture. Another session of the
    /// same user, a second login before the old one had its user, is dropped.
    pub fn set_user(token: &str, user: LoginUser) -> Result<()> {
        change(|sessions| {
            let Some(index) = sessions.list.iter().position(|session| session.token == token) else {
                return Ok(());
            };
            sessions.list[index].user = Some(user.clone());
            let twin = sessions.list.iter().enumerate().position(|(other, session)| {
                other != index && session.user.as_ref().is_some_and(|known| known.id == user.id)
            });
            if let Some(twin) = twin {
                let kept = sessions.list[index].token.clone();
                let was_active = sessions.active == Some(twin);
                sessions.remove(twin);
                if was_active {
                    sessions.active = sessions.list.iter().position(|session| session.token == kept);
                }
            }
            Ok(())
        })
    }

    /// Makes the session of this user active.
    pub fn switch(user_id: &str) -> Result<()> {
        change(|sessions| {
            let Some(index) = sessions.index_of(user_id) else {
                bail!("no session of user {user_id} on this device");
            };
            sessions.active = Some(index);
            Ok(())
        })
    }

    /// Drops the session of this user and gives its token back, to end it on
    /// the server. Nobody is active after it when it was the active one.
    pub fn remove(user_id: &str) -> Result<Option<String>> {
        change(|sessions| Ok(sessions.index_of(user_id).map(|index| sessions.remove(index).token)))
    }

    /// Drops the active session, the other sessions stay and none of them
    /// becomes active. Clearing with nobody active is fine.
    pub fn clear() -> Result<()> {
        change(|sessions| {
            if let Some(index) = sessions.active {
                sessions.remove(index);
            }
            Ok(())
        })
    }

    /// The folder of the session file. `hilen` sets it to the root of the
    /// `OnDisk` files of the app, a tool with no window sets the same one.
    #[cfg(not_wasm)]
    pub fn set_root(path: impl AsRef<Path>) {
        platform::set_root(path.as_ref());
    }
}

/// The name of the session file, and of its `localStorage` key in a browser.
const FILE: &str = "session.bin";

/// The list in the credential store of the system.
#[cfg(feature = "keychain")]
static KEPT: SecretStore = SecretStore::device_only("session");

pub(crate) fn seal(text: &str) -> Result<Vec<u8>> {
    encrypt(text.as_bytes(), &session_key())
}

pub(crate) fn open(sealed: &[u8]) -> Result<String> {
    Ok(String::from_utf8(decrypt(sealed, &session_key())?)?)
}

/// The sessions on this device, none when nothing is stored or it does not
/// open. It waits for a change that runs, a first load can move the file.
fn read() -> Sessions {
    let guard = CHANGE.lock().unwrap_or_else(PoisonError::into_inner);
    let sessions = read_unlocked();
    drop(guard);
    sessions
}

/// `read` for a caller that holds the lock.
fn read_unlocked() -> Sessions {
    match stored_text() {
        Ok(Some(text)) => parse(&text),
        Ok(None) => Sessions::default(),
        Err(error) => {
            log::warn!("the stored session does not open: {error}");
            Sessions::default()
        }
    }
}

/// An engine with 1 login kept the bare token in the file. It loads as the
/// only session, active, with no user until the app asks the server.
fn parse(text: &str) -> Sessions {
    from_str(text).unwrap_or_else(|_| Sessions {
        list:   vec![Session {
            token: text.to_owned(),
            user:  None,
        }],
        active: Some(0),
    })
}

fn change<T>(edit: impl FnOnce(&mut Sessions) -> Result<T>) -> Result<T> {
    let guard = CHANGE.lock().unwrap_or_else(PoisonError::into_inner);
    let mut sessions = read_unlocked();
    let result = edit(&mut sessions)?;
    if sessions.list.is_empty() {
        store_text(None)?;
    } else {
        store_text(Some(&to_string(&sessions)?))?;
    }
    drop(guard);
    Ok(result)
}

/// The stored list as JSON, or the bare token of an engine with 1 login.
fn stored_text() -> Result<Option<String>> {
    #[cfg(feature = "keychain")]
    if KEPT.in_credential_store() {
        return match KEPT.load() {
            Some(text) => Ok(Some(text)),
            None => move_file_in(),
        };
    }

    platform::read(FILE)?.map(|sealed| open(&sealed)).transpose()
}

/// `None` removes what is stored.
fn store_text(text: Option<&str>) -> Result<()> {
    #[cfg(feature = "keychain")]
    if KEPT.in_credential_store() {
        match text {
            Some(text) => KEPT.save(text)?,
            None => KEPT.clear()?,
        }
        // A file that could not move in before must not come back once
        // the store is empty again.
        return platform::remove(FILE);
    }

    match text {
        Some(text) => platform::write(FILE, &seal(text)?),
        None => platform::remove(FILE),
    }
}

/// The sealed file an engine before the credential store wrote. Its list
/// goes into the store and the file is deleted. When the store does not
/// take it, the file stays and is read again next time.
#[cfg(feature = "keychain")]
fn move_file_in() -> Result<Option<String>> {
    let Some(sealed) = platform::read(FILE)? else {
        return Ok(None);
    };
    let text = open(&sealed)?;

    if let Err(error) = KEPT.save(&text) {
        log::warn!(
            "the stored sessions stay in the sealed file, the credential store did not take them: {error}"
        );
        return Ok(Some(text));
    }
    platform::remove(FILE)?;
    log::info!("the stored sessions moved from the sealed file into the credential store");
    Ok(Some(text))
}

#[cfg(all(not_wasm, not(tvos)))]
pub(crate) mod platform {
    use std::{
        fs::{create_dir_all, read as read_file, remove_file, write as write_file},
        io::ErrorKind,
        path::{Path, PathBuf},
        sync::{Mutex, PoisonError},
    };

    use anyhow::{Context, Result};

    static ROOT: Mutex<Option<PathBuf>> = Mutex::new(None);

    pub(super) fn set_root(path: &Path) {
        *ROOT.lock().unwrap_or_else(PoisonError::into_inner) = Some(path.to_path_buf());
    }

    fn path(file: &str) -> PathBuf {
        let root = ROOT.lock().unwrap_or_else(PoisonError::into_inner).clone();
        root.map_or_else(|| PathBuf::from(file), |root| root.join(file))
    }

    pub(crate) fn read(file: &str) -> Result<Option<Vec<u8>>> {
        match read_file(path(file)) {
            Ok(sealed) => Ok(Some(sealed)),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error).context("failed to read the session file"),
        }
    }

    pub(crate) fn write(file: &str, sealed: &[u8]) -> Result<()> {
        let path = path(file);
        if let Some(parent) = path.parent() {
            create_dir_all(parent).context("failed to create the session folder")?;
        }
        write_file(path, sealed).context("failed to write the session file")
    }

    pub(crate) fn remove(file: &str) -> Result<()> {
        match remove_file(path(file)) {
            Err(error) if error.kind() != ErrorKind::NotFound => {
                Err(error).context("failed to remove the session file")
            }
            _ => Ok(()),
        }
    }
}

/// tvOS gives an app no folder that keeps its files, the system may empty
/// the caches folder while the app does not run. The user defaults of the
/// app stay, so every sealed file is 1 entry there, named after the file.
#[cfg(tvos)]
pub(crate) mod platform {
    use std::path::Path;

    use anyhow::Result;
    use objc2::rc::Retained;
    use objc2_foundation::{NSData, NSString, NSUserDefaults};

    pub(super) fn set_root(_path: &Path) {}

    fn key(file: &str) -> Retained<NSString> {
        NSString::from_str(&format!("hilen-{file}"))
    }

    pub(crate) fn read(file: &str) -> Result<Option<Vec<u8>>> {
        let data = NSUserDefaults::standardUserDefaults().dataForKey(&key(file));
        Ok(data.map(|data| data.to_vec()))
    }

    pub(crate) fn write(file: &str, sealed: &[u8]) -> Result<()> {
        let data = NSData::with_bytes(sealed);
        unsafe { NSUserDefaults::standardUserDefaults().setObject_forKey(Some(&data), &key(file)) };
        Ok(())
    }

    pub(crate) fn remove(file: &str) -> Result<()> {
        NSUserDefaults::standardUserDefaults().removeObjectForKey(&key(file));
        Ok(())
    }
}

/// A browser has no files. `localStorage` is per origin, other sites cannot
/// read it.
#[cfg(wasm)]
pub(crate) mod platform {
    use anyhow::{Result, anyhow};
    use base64::{Engine, engine::general_purpose::STANDARD};
    use web_sys::{Storage, window};

    /// The session keeps the key it always had.
    fn key(file: &str) -> String {
        if file == super::FILE {
            "hilen-session".to_owned()
        } else {
            format!("hilen-{file}")
        }
    }

    fn storage() -> Result<Storage> {
        window()
            .ok_or_else(|| anyhow!("no window"))?
            .local_storage()
            .map_err(|error| anyhow!("localStorage is blocked: {error:?}"))?
            .ok_or_else(|| anyhow!("no localStorage"))
    }

    pub(crate) fn read(file: &str) -> Result<Option<Vec<u8>>> {
        let stored = storage()?
            .get_item(&key(file))
            .map_err(|error| anyhow!("failed to read localStorage: {error:?}"))?;
        Ok(stored.map(|text| STANDARD.decode(text)).transpose()?)
    }

    pub(crate) fn write(file: &str, sealed: &[u8]) -> Result<()> {
        storage()?
            .set_item(&key(file), &STANDARD.encode(sealed))
            .map_err(|error| anyhow!("failed to write localStorage: {error:?}"))
    }

    pub(crate) fn remove(file: &str) -> Result<()> {
        storage()?
            .remove_item(&key(file))
            .map_err(|error| anyhow!("failed to clear localStorage: {error:?}"))
    }
}

#[cfg(all(test, not_wasm))]
mod test {
    use std::env::temp_dir;

    use anyhow::Result;
    use serial_test::serial;

    use super::{FILE, LoginUser, SessionStore, platform, seal};
    use crate::SecretStore;

    /// The sealed file, also in a build with the credential store.
    fn use_test_root() -> Result<()> {
        SecretStore::keep_in_files();
        SessionStore::set_root(temp_dir().join("hilen-session-test"));
        platform::remove(FILE)
    }

    /// An app updates to an engine with the credential store. The file of
    /// the old engine moves in at the first load and is gone, the logins
    /// stay, and from then on nothing is written to a file.
    #[cfg(feature = "keychain")]
    #[test]
    #[serial]
    fn a_sealed_file_of_today_moves_into_the_credential_store() -> Result<()> {
        use super::KEPT;

        use_test_root()?;
        SessionStore::add("token ann", user("ann"))?;
        SessionStore::add("token bob", user("bob"))?;
        assert!(platform::read(FILE)?.is_some());

        SecretStore::set_service("hilen-session-test");
        SecretStore::use_credential_store();
        KEPT.clear()?;
        assert!(KEPT.in_credential_store());

        assert_eq!(SessionStore::load().as_deref(), Some("token bob"));
        assert_eq!(platform::read(FILE)?, None, "the file is gone after the move");
        assert_eq!(user_ids(), ["ann", "bob"]);
        assert!(KEPT.load().is_some_and(|text| text.contains("token ann")));

        SessionStore::switch("ann")?;
        SessionStore::clear()?;
        assert_eq!(user_ids(), ["bob"]);
        assert_eq!(platform::read(FILE)?, None, "a change writes no file");

        SessionStore::remove("bob")?;
        assert_eq!(KEPT.load(), None);
        assert_eq!(SessionStore::sessions(), []);
        use_test_root()
    }

    /// A bare token of an engine with 1 login moves in the same way.
    #[cfg(feature = "keychain")]
    #[test]
    #[serial]
    fn a_bare_token_of_an_old_engine_moves_into_the_credential_store() -> Result<()> {
        use super::KEPT;

        use_test_root()?;
        platform::write(FILE, &seal("old token")?)?;

        SecretStore::set_service("hilen-session-test");
        SecretStore::use_credential_store();
        KEPT.clear()?;

        assert_eq!(SessionStore::load().as_deref(), Some("old token"));
        assert_eq!(platform::read(FILE)?, None);
        SessionStore::set_user("old token", user("ann"))?;
        assert_eq!(SessionStore::active_user(), Some(user("ann")));

        SessionStore::clear()?;
        assert_eq!(KEPT.load(), None);
        use_test_root()
    }

    fn user(id: &str) -> LoginUser {
        LoginUser {
            id:      id.to_owned(),
            email:   format!("{id}@example.com"),
            name:    id.to_uppercase(),
            picture: None,
        }
    }

    fn user_ids() -> Vec<String> {
        SessionStore::sessions()
            .into_iter()
            .filter_map(|session| session.user.map(|user| user.id))
            .collect()
    }

    #[test]
    #[serial]
    fn add_load_clear() -> Result<()> {
        use_test_root()?;

        SessionStore::clear()?;
        assert_eq!(SessionStore::load(), None);

        SessionStore::add("token one", user("ann"))?;
        assert_eq!(SessionStore::load().as_deref(), Some("token one"));
        assert_eq!(SessionStore::active_user(), Some(user("ann")));

        SessionStore::clear()?;
        assert_eq!(SessionStore::load(), None);
        assert_eq!(SessionStore::sessions(), []);
        assert_eq!(platform::read(FILE)?, None);
        // Clearing twice is fine, logout must not fail on a missing file.
        SessionStore::clear()
    }

    #[test]
    #[serial]
    fn a_second_login_keeps_the_first() -> Result<()> {
        use_test_root()?;

        SessionStore::add("token ann", user("ann"))?;
        SessionStore::add("token bob", user("bob"))?;

        assert_eq!(user_ids(), ["ann", "bob"]);
        assert_eq!(SessionStore::load().as_deref(), Some("token bob"));

        SessionStore::switch("ann")?;
        assert_eq!(SessionStore::load().as_deref(), Some("token ann"));
        assert!(SessionStore::switch("nobody").is_err());

        // The same user again gets a new token in its old place.
        SessionStore::add("token ann 2", user("ann"))?;
        assert_eq!(user_ids(), ["ann", "bob"]);
        assert_eq!(SessionStore::load().as_deref(), Some("token ann 2"));

        use_test_root()
    }

    #[test]
    #[serial]
    fn clear_drops_only_the_active_session() -> Result<()> {
        use_test_root()?;

        SessionStore::add("token ann", user("ann"))?;
        SessionStore::add("token bob", user("bob"))?;
        SessionStore::clear()?;

        assert_eq!(user_ids(), ["ann"]);
        // Nobody else is made active behind the back of the app.
        assert_eq!(SessionStore::load(), None);

        use_test_root()
    }

    #[test]
    #[serial]
    fn remove_keeps_the_active_session_in_place() -> Result<()> {
        use_test_root()?;

        SessionStore::add("token ann", user("ann"))?;
        SessionStore::add("token bob", user("bob"))?;
        SessionStore::add("token cid", user("cid"))?;

        assert_eq!(SessionStore::remove("ann")?.as_deref(), Some("token ann"));
        assert_eq!(SessionStore::load().as_deref(), Some("token cid"));
        assert_eq!(SessionStore::remove("ann")?, None);

        assert_eq!(SessionStore::remove("cid")?.as_deref(), Some("token cid"));
        assert_eq!(SessionStore::load(), None);
        assert_eq!(user_ids(), ["bob"]);

        use_test_root()
    }

    #[test]
    #[serial]
    fn a_bare_token_of_an_old_engine_loads_as_the_active_session() -> Result<()> {
        use_test_root()?;
        platform::write(FILE, &seal("old token")?)?;

        assert_eq!(SessionStore::load().as_deref(), Some("old token"));
        assert_eq!(SessionStore::active_user(), None);

        SessionStore::set_user("old token", user("ann"))?;
        assert_eq!(SessionStore::active_user(), Some(user("ann")));
        assert_eq!(SessionStore::load().as_deref(), Some("old token"));

        use_test_root()
    }

    #[test]
    #[serial]
    fn set_user_drops_a_twin_of_the_same_user() -> Result<()> {
        use_test_root()?;
        platform::write(FILE, &seal("old token")?)?;
        // The server could not be reached at launch, so the old session got no
        // user, and the same person logged in again.
        SessionStore::add("new token", user("ann"))?;
        SessionStore::switch("ann")?;
        SessionStore::set_user("old token", user("ann"))?;

        let sessions = SessionStore::sessions();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].token, "old token");
        assert_eq!(SessionStore::load().as_deref(), Some("old token"));

        use_test_root()
    }

    #[test]
    #[serial]
    fn token_is_not_on_disk_as_text() -> Result<()> {
        use_test_root()?;

        SessionStore::add("a very recognizable token", user("ann"))?;
        let bytes = platform::read(FILE)?.expect("the file was just written");
        assert!(!String::from_utf8_lossy(&bytes).contains("recognizable"));

        SessionStore::clear()
    }

    #[test]
    #[serial]
    fn broken_file_loads_as_none() -> Result<()> {
        use_test_root()?;

        platform::write(FILE, b"not sealed by this key at all, only some text")?;
        assert_eq!(SessionStore::load(), None);

        SessionStore::clear()
    }
}
