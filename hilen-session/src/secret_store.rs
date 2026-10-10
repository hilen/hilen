use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;

use crate::session_store::{open, platform, seal};

/// When set, every store keeps its secret in a sealed file. For tests and
/// for a tool that runs where the credential store of the system must not be
/// asked, like a build machine with no user logged in.
static FILES_ONLY: AtomicBool = AtomicBool::new(false);

/// One secret of this app on this device under a name, like the refresh token
/// of an account.
///
/// With the `keychain` feature it lives in the credential store of the
/// system: the Keychain on a Mac and on iOS, the Credential Manager on
/// Windows, the Secret Service on Linux, the Keystore on Android. Other
/// users of the machine and other apps do not get it from there. Without the
/// feature, in a browser, and when the system has no credential store, a
/// Linux with no Secret Service, it is a file sealed like the session of
/// `SessionStore`.
///
/// Windows takes about 2500 bytes per secret, keep one small.
pub struct SecretStore {
    name: &'static str,
}

impl SecretStore {
    pub const fn new(name: &'static str) -> Self {
        Self { name }
    }

    /// The name secrets are filed under in the credential store of the
    /// system. `hilen` sets it to the project name of the app.
    #[cfg(not_wasm)]
    pub fn set_service(service: impl ToString) {
        #[cfg(feature = "keychain")]
        keychain::set_service(service.to_string());
        #[cfg(not(feature = "keychain"))]
        let _ = service.to_string();
    }

    /// From now on every store keeps its secret in a sealed file.
    pub fn keep_in_files() {
        FILES_ONLY.store(true, Ordering::SeqCst);
    }

    /// `None` when nothing is stored, and when what is stored does not open.
    pub fn load(&self) -> Option<String> {
        #[cfg(all(feature = "keychain", not_wasm))]
        if let Some(entry) = self.entry() {
            return keychain::get(&entry, self.name);
        }

        platform::read(&self.file())
            .and_then(|sealed| sealed.map(|sealed| open(&sealed)).transpose())
            .unwrap_or_else(|error| {
                log::warn!("the stored secret {} does not open: {error}", self.name);
                None
            })
    }

    pub fn save(&self, secret: &str) -> Result<()> {
        #[cfg(all(feature = "keychain", not_wasm))]
        if let Some(entry) = self.entry() {
            return keychain::set(&entry, secret);
        }

        platform::write(&self.file(), &seal(secret)?)
    }

    /// Clearing what is not there is fine.
    pub fn clear(&self) -> Result<()> {
        #[cfg(all(feature = "keychain", not_wasm))]
        if let Some(entry) = self.entry() {
            return keychain::delete(&entry);
        }

        platform::remove(&self.file())
    }

    fn file(&self) -> String {
        format!("{}.secret", self.name)
    }

    #[cfg(all(feature = "keychain", not_wasm))]
    fn entry(&self) -> Option<keyring_core::Entry> {
        if FILES_ONLY.load(Ordering::SeqCst) {
            return None;
        }
        keychain::entry(self.name)
    }
}

#[cfg(all(feature = "keychain", not_wasm))]
mod keychain {
    use std::sync::{LazyLock, Mutex, PoisonError};

    use anyhow::{Result, anyhow};
    use keyring_core::{Entry, Error, set_default_store};

    static SERVICE: Mutex<Option<String>> = Mutex::new(None);

    pub(super) fn set_service(service: String) {
        *SERVICE.lock().unwrap_or_else(PoisonError::into_inner) = Some(service);
    }

    fn start() -> keyring_core::Result<()> {
        #[cfg(target_os = "macos")]
        let store = keyring_store_apple::keychain::Store::new()?;
        // The protected store wants an app with a bundle id, which an iOS app
        // always is and a bare `cargo run` binary on a Mac is not.
        #[cfg(any(target_os = "ios", target_os = "tvos"))]
        let store = keyring_store_apple::protected::Store::new()?;
        #[cfg(target_os = "windows")]
        let store = keyring_store_windows::Store::new()?;
        #[cfg(target_os = "linux")]
        let store = keyring_store_linux::Store::new()?;
        #[cfg(target_os = "android")]
        let store = keyring_store_android::Store::new()?;

        set_default_store(store);
        Ok(())
    }

    /// False when the system has no credential store. It is asked once, and
    /// the reason goes to the log once.
    static STARTED: LazyLock<bool> = LazyLock::new(|| match start() {
        Ok(()) => true,
        Err(error) => {
            log::warn!("no credential store on this system, secrets go to sealed files: {error}");
            false
        }
    });

    pub(super) fn entry(name: &str) -> Option<Entry> {
        if !*STARTED {
            return None;
        }

        let service = SERVICE.lock().unwrap_or_else(PoisonError::into_inner).clone();
        let Some(service) = service else {
            log::error!("no service name for the secret {name}, call SecretStore::set_service first");
            return None;
        };

        Entry::new(&service, name)
            .inspect_err(|error| log::error!("no credential store entry for {name}: {error}"))
            .ok()
    }

    pub(super) fn get(entry: &Entry, name: &str) -> Option<String> {
        match entry.get_password() {
            Ok(secret) => Some(secret),
            Err(Error::NoEntry) => None,
            Err(error) => {
                log::warn!("the stored secret {name} does not open: {error}");
                None
            }
        }
    }

    pub(super) fn set(entry: &Entry, secret: &str) -> Result<()> {
        entry
            .set_password(secret)
            .map_err(|error| anyhow!("the credential store did not take the secret: {error}"))
    }

    pub(super) fn delete(entry: &Entry) -> Result<()> {
        match entry.delete_credential() {
            Ok(()) | Err(Error::NoEntry) => Ok(()),
            Err(error) => Err(anyhow!("the credential store did not drop the secret: {error}")),
        }
    }
}

#[cfg(all(test, not_wasm))]
mod test {
    use std::env::temp_dir;

    use anyhow::Result;
    use serial_test::serial;

    use super::SecretStore;
    use crate::{SessionStore, session_store::platform};

    static STORE: SecretStore = SecretStore::new("test-account");

    fn use_files() {
        SessionStore::set_root(temp_dir().join("hilen-secret-test"));
        SecretStore::keep_in_files();
    }

    #[test]
    #[serial]
    fn save_load_clear() -> Result<()> {
        use_files();

        STORE.clear()?;
        assert_eq!(STORE.load(), None);

        STORE.save("secret one")?;
        assert_eq!(STORE.load().as_deref(), Some("secret one"));

        STORE.save("secret two")?;
        assert_eq!(STORE.load().as_deref(), Some("secret two"));

        STORE.clear()?;
        assert_eq!(STORE.load(), None);
        STORE.clear()
    }

    #[test]
    #[serial]
    fn a_secret_is_not_on_disk_as_text_and_not_in_the_session_file() -> Result<()> {
        use_files();
        SessionStore::clear()?;

        STORE.save("a very recognizable secret")?;
        let bytes = platform::read("test-account.secret")?.expect("the file was just written");
        assert!(!String::from_utf8_lossy(&bytes).contains("recognizable"));
        assert_eq!(SessionStore::load(), None);

        STORE.clear()
    }

    /// Asks the real credential store of this machine. Not for a build
    /// machine, a Mac may show a dialog:
    /// `cargo test -p hilen-session --features keychain -- --ignored`
    #[cfg(feature = "keychain")]
    #[test]
    #[ignore = "asks the real credential store of the system"]
    fn the_real_credential_store_keeps_a_secret() -> Result<()> {
        use std::sync::atomic::Ordering;

        static REAL: SecretStore = SecretStore::new("real-store-test");

        super::FILES_ONLY.store(false, Ordering::SeqCst);
        SecretStore::set_service("hilen-session-test");
        SessionStore::set_root(temp_dir().join("hilen-secret-test-real"));

        REAL.clear()?;
        assert_eq!(REAL.load(), None);
        REAL.save("kept by the system")?;
        assert_eq!(REAL.load().as_deref(), Some("kept by the system"));
        // Nothing went to a file, the system has it.
        assert_eq!(platform::read("real-store-test.secret")?, None);
        REAL.clear()?;
        assert_eq!(REAL.load(), None);
        Ok(())
    }
}
