use std::{
    fmt::{Debug, Formatter},
    marker::PhantomData,
    path::{Path, PathBuf},
};

#[cfg(all(any(feature = "login", feature = "secrets"), not_wasm))]
use hilen_session::SessionStore;
use parking_lot::Mutex;
use toml_edit::{DocumentMut, Item, Table, Value};

use crate::{filesystem::Paths, store::storable::Storable};

static ROOT_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

fn set_value<T: serde::ser::Serialize>(value: T, path: &Path) {
    if let Err(error) = try_set_value(value, path) {
        // A native store is a file of the app, a failed write there is a
        // broken install. A browser or a TV can refuse a write any time.
        #[cfg(all(not_wasm, not(tvos)))]
        panic!("{error}");
        #[cfg(any(wasm, tvos))]
        log::warn!("{error}");
    }
}

fn try_set_value<T: serde::ser::Serialize>(value: T, path: &Path) -> Result<(), StoreError> {
    let refused = |reason| StoreError {
        path: path.to_owned(),
        reason,
    };
    let text = if is_toml(path) {
        toml_text(&value, backend::read(path).as_deref()).map_err(refused)?
    } else {
        serde_json::to_string_pretty(&value).expect("Failed to serialize data")
    };
    backend::write(path, &text).map_err(refused)
}

/// A store whose file name ends in `.toml` is a TOML file a person may edit
/// by hand. Any other store is JSON.
fn is_toml(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "toml")
}

/// The TOML of `value`. Over the text the file has now, only the values
/// that changed change, so the comments, the order of the keys and the
/// blank lines a person wrote stay.
fn toml_text<T: serde::ser::Serialize>(value: &T, old: Option<&str>) -> Result<String, String> {
    let fresh =
        toml::to_string(value).map_err(|error| format!("{error}, a TOML store holds a struct or a map"))?;
    let Some(mut document) = old.and_then(|old| old.parse::<DocumentMut>().ok()) else {
        return Ok(fresh);
    };
    let new = fresh.parse::<DocumentMut>().map_err(|error| error.to_string())?;
    merge(document.as_table_mut(), new.as_table());
    Ok(document.to_string())
}

fn merge(old: &mut Table, new: &Table) {
    let gone: Vec<String> = old
        .iter()
        .map(|(key, _)| key.to_string())
        .filter(|key| !new.contains_key(key))
        .collect();
    for key in gone {
        old.remove(&key);
    }
    for (key, item) in new {
        match (old.get_mut(key), item) {
            (Some(Item::Table(old_table)), Item::Table(new_table)) => merge(old_table, new_table),
            (Some(Item::Value(old_value)), Item::Value(new_value)) => {
                if !same(old_value, new_value) {
                    let decor = old_value.decor().clone();
                    *old_value = new_value.clone();
                    *old_value.decor_mut() = decor;
                }
            }
            _ => {
                old.insert(key, item.clone());
            }
        }
    }
}

/// The 2 values hold the same, whatever the layout of their text, like an
/// array a person wrote on 1 line with no spaces.
fn same(old: &Value, new: &Value) -> bool {
    let held = |value: &Value| {
        let mut value = value.clone();
        value.decor_mut().clear();
        format!("value = {value}").parse::<toml::Table>().ok()
    };
    held(old).is_some_and(|old| Some(old) == held(new))
}

/// A write the platform refused, like a full disk or a browser storage that
/// is over its quota or blocked. `reason` is the text of the platform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreError {
    pub path:   PathBuf,
    pub reason: String,
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "failed to save {}: {}", self.path.display(), self.reason)
    }
}

impl std::error::Error for StoreError {}

/// A file that does not parse gives nothing, like a hand edit with a typo
/// or a file of an older version of the app whose type changed since. The
/// file stays as it is, so the edit can be fixed, and the next `set`
/// writes over it.
fn parse<T: Storable>(path: &Path, text: &str) -> Option<T> {
    let parsed = if is_toml(path) {
        toml::from_str(text).map_err(|error| error.to_string())
    } else {
        serde_json::from_str(text).map_err(|error| error.to_string())
    };
    match parsed {
        Ok(value) => Some(value),
        Err(error) => {
            log::warn!("{} does not parse, the default is used: {error}", path.display());
            None
        }
    }
}

fn get_value<T: Storable>(path: &Path) -> Option<T> {
    let text = backend::read(path)?;
    parse(path, &text)
}

fn get_or_init_value<T: Storable + Default>(path: &Path) -> T {
    let Some(text) = backend::read(path) else {
        let new = T::default();
        set_value(&new, path);
        return new;
    };
    parse(path, &text).unwrap_or_default()
}

#[cfg(all(not_wasm, not(tvos)))]
mod backend {
    use std::{
        fs::{create_dir_all, read_to_string, remove_file, write as write_file},
        io::ErrorKind,
        path::Path,
    };

    pub(super) fn read(path: &Path) -> Option<String> {
        if !path.exists() {
            return None;
        }
        Some(read_to_string(path).expect("Failed to read file"))
    }

    pub(super) fn write(path: &Path, json: &str) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        write_file(path, json).map_err(|error| error.to_string())
    }

    /// A store that was never written, or was reset before, has no file.
    pub(super) fn remove(path: &Path) {
        if let Err(error) = remove_file(path)
            && error.kind() != ErrorKind::NotFound
        {
            log::warn!("failed to remove {}: {error}", path.display());
        }
    }
}

/// tvOS gives an app no folder that keeps its files, the system may empty
/// the caches folder while the app does not run. The user defaults of the
/// app stay, so every store file is 1 entry there, named after its path
/// under the root. tvOS ends an app whose defaults pass 1 MB, a store is
/// for settings, not for data.
#[cfg(tvos)]
mod backend {
    use std::path::Path;

    use objc2::rc::Retained;
    use objc2_foundation::{NSString, NSUserDefaults};

    /// A store this big is logged, the whole of the defaults has 1 MB.
    const BIG: usize = 100 * 1024;

    // The root holds the folder of the app, which the system may rename
    // with an update, so the key has only the part below it.
    fn key(path: &Path) -> Retained<NSString> {
        let root = super::ROOT_PATH.lock().clone().map(super::expand_tilde);
        let below = root.and_then(|root| path.strip_prefix(root).ok().map(Path::to_owned));
        let name = below.unwrap_or_else(|| path.to_owned());
        NSString::from_str(&format!("hilen-store:{}", name.display()))
    }

    pub(super) fn read(path: &Path) -> Option<String> {
        let text = NSUserDefaults::standardUserDefaults().stringForKey(&key(path))?;
        Some(text.to_string())
    }

    pub(super) fn write(path: &Path, json: &str) -> Result<(), String> {
        if json.len() > BIG {
            log::warn!(
                "{} is {} bytes, the user defaults of tvOS hold 1 MB in all",
                path.display(),
                json.len()
            );
        }
        let text = NSString::from_str(json);
        unsafe { NSUserDefaults::standardUserDefaults().setObject_forKey(Some(&text), &key(path)) };
        Ok(())
    }

    pub(super) fn remove(path: &Path) {
        NSUserDefaults::standardUserDefaults().removeObjectForKey(&key(path));
    }
}

/// A browser has no files. Every store file is one `localStorage` entry
/// named after its path. It is per origin and survives a reload and a
/// restart of the browser. A browser that blocks it keeps nothing, the
/// app then runs on defaults, which is logged once per call.
#[cfg(wasm)]
mod backend {
    use std::path::Path;

    use web_sys::{Storage, window};

    fn key(path: &Path) -> String {
        format!("hilen-store:{}", path.display())
    }

    fn storage() -> Option<Storage> {
        let storage = window().and_then(|window| window.local_storage().ok().flatten());
        if storage.is_none() {
            log::warn!("this browser gives no localStorage, nothing is stored");
        }
        storage
    }

    pub(super) fn read(path: &Path) -> Option<String> {
        match storage()?.get_item(&key(path)) {
            Ok(json) => json,
            Err(error) => {
                log::warn!("failed to read {} from localStorage: {error:?}", path.display());
                None
            }
        }
    }

    pub(super) fn write(path: &Path, json: &str) -> Result<(), String> {
        let Some(storage) = storage() else {
            return Err("this browser gives no localStorage".to_string());
        };
        storage
            .set_item(&key(path), json)
            .map_err(|error| format!("localStorage refused the write: {error:?}"))
    }

    pub(super) fn remove(path: &Path) {
        let Some(storage) = storage() else {
            return;
        };
        if let Err(error) = storage.remove_item(&key(path)) {
            log::warn!("failed to remove {} from localStorage: {error:?}", path.display());
        }
    }
}

#[derive(Clone, Default)]
pub struct OnDisk<T> {
    path: PathBuf,
    _p:   PhantomData<T>,
}

impl<T: Storable> OnDisk<T> {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_owned(),
            _p:   PhantomData,
        }
    }

    pub fn set(&self, val: impl Into<T>) {
        let val = val.into();
        set_value(val, &rooted(&self.path));
    }

    /// `set` that gives the refusal back, so the app can tell the user that
    /// nothing was saved, like when a browser storage is full.
    pub fn try_set(&self, val: impl Into<T>) -> Result<(), StoreError> {
        try_set_value(val.into(), &rooted(&self.path))
    }

    pub fn get(&self) -> Option<T> {
        get_value(&rooted(&self.path))
    }

    pub fn reset(&self) {
        backend::remove(&rooted(&self.path));
    }
}

/// Where a store file lives on disk, under the root the app set.
pub(crate) fn rooted(path: &Path) -> PathBuf {
    let full = ROOT_PATH
        .lock()
        .as_ref()
        .map_or_else(|| path.to_owned(), |root| root.join(path));
    expand_tilde(full)
}

impl<T: Storable + Default> OnDisk<T> {
    pub fn edit(&self, edit: impl FnOnce(&mut T)) {
        let mut val = self.get_or_init();
        edit(&mut val);
        self.set(val);
    }

    /// `edit` that gives a refused write back, see `try_set`.
    pub fn try_edit(&self, edit: impl FnOnce(&mut T)) -> Result<(), StoreError> {
        let mut val = self.get().unwrap_or_default();
        edit(&mut val);
        self.try_set(val)
    }

    pub fn get_or_init(&self) -> T {
        get_or_init_value(&rooted(&self.path))
    }
}

impl<T> OnDisk<T> {
    pub fn set_root_path(path: impl AsRef<Path>) {
        *ROOT_PATH.lock() = Some(path.as_ref().to_path_buf());
        // The login session lives next to the other files, see hilen-session.
        // A `SecretStore` with no credential store of the system does too.
        #[cfg(all(any(feature = "login", feature = "secrets"), not_wasm))]
        SessionStore::set_root(expand_tilde(path.as_ref()));
    }
}

impl<T: Storable + Debug> Debug for OnDisk<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.get().fmt(f)
    }
}

fn expand_tilde<P: AsRef<Path>>(path: P) -> PathBuf {
    let p = path.as_ref();
    if !p.starts_with("~") {
        return p.to_path_buf();
    }

    if p == Path::new("~") {
        Paths::home()
    } else {
        Paths::home().join(p.strip_prefix("~").unwrap())
    }
}

#[cfg(test)]
mod test {

    use std::sync::LazyLock;

    use serde::{Deserialize, Serialize};
    use serial_test::serial;

    use crate::{filesystem::Paths, store::OnDisk};

    #[derive(Debug, PartialEq, Default, Serialize, Deserialize, Clone)]
    struct Data {
        number: i32,
        string: String,
    }

    #[derive(Debug, PartialEq, Default, Serialize, Deserialize, Clone)]
    struct Words {
        words: Vec<String>,
        count: i32,
    }

    static STORED: LazyLock<OnDisk<i32>> = LazyLock::new(|| OnDisk::new("stored_i32_test.json"));
    static STORED_STRUCT: LazyLock<OnDisk<Data>> = LazyLock::new(|| OnDisk::new("stored_struct_test.json"));
    static INITIALIZED: LazyLock<OnDisk<i32>> = LazyLock::new(|| OnDisk::new("initialized_i32_test.json"));

    fn check_send<T: Send>(_send: &T) {}
    fn check_sync<T: Sync>(_sync: &T) {}

    // The root is one global, the start test in `app_starter` sets it too.
    #[test]
    #[serial(on_disk_root)]
    fn stored() {
        OnDisk::<()>::set_root_path("~/.test_on_disk/");

        check_send(&STORED);
        check_sync(&STORED);
        check_send(&STORED_STRUCT);
        check_sync(&STORED_STRUCT);

        STORED.set(10);
        STORED.reset();
        assert_eq!(STORED.get(), None);
        // The second reset finds no file. It panicked, and an app that
        // reset a store at its start died there.
        STORED.reset();
        assert_eq!(STORED.get(), None);

        let data = Data {
            number: 555,
            string: "Helloyyyy".to_string(),
        };

        STORED_STRUCT.set(data.clone());

        let loaded_data = STORED_STRUCT.get();

        assert_eq!(data, loaded_data.unwrap());

        assert_eq!(INITIALIZED.get_or_init(), 0);
        assert_eq!(INITIALIZED.get(), Some(0));
        INITIALIZED.reset();
    }

    #[test]
    #[serial(on_disk_root)]
    fn a_refused_write_is_an_error() {
        let root = std::env::temp_dir().join(format!("hilen_refused_write_{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        // A file where the store wants a folder makes every write fail.
        std::fs::write(root.join("blocked"), "").unwrap();
        OnDisk::<()>::set_root_path(&root);

        let store = OnDisk::<i32>::new("blocked/value.json");
        let error = store.try_set(5).unwrap_err();
        assert!(error.path.ends_with("blocked/value.json"), "{error}");
        assert!(store.try_edit(|value| *value += 1).is_err());

        let fine = OnDisk::<i32>::new("fine.json");
        fine.try_set(5).unwrap();
        assert_eq!(fine.get(), Some(5));

        std::fs::remove_dir_all(&root).unwrap();
    }

    // The failure this prevents: a setting file with a typo from a hand
    // edit panicked the app at its next read.
    #[test]
    #[serial(on_disk_root)]
    fn a_file_that_does_not_parse_gives_the_default() {
        use std::{
            env::temp_dir,
            fs::{create_dir_all, read_to_string, remove_dir_all, write},
            process::id,
        };

        let root = temp_dir().join(format!("hilen_bad_json_{}", id()));
        create_dir_all(&root).unwrap();
        write(root.join("bad.json"), "[\"far\", ").unwrap();
        OnDisk::<()>::set_root_path(&root);

        let store = OnDisk::<Vec<String>>::new("bad.json");
        assert_eq!(store.get(), None);
        assert_eq!(store.get_or_init(), Vec::<String>::new());
        // The file stays for a fix by hand.
        assert_eq!(read_to_string(root.join("bad.json")).unwrap(), "[\"far\", ");

        remove_dir_all(&root).unwrap();
    }

    #[test]
    #[serial(on_disk_root)]
    fn a_toml_store_keeps_what_a_person_wrote() {
        use std::{
            env::temp_dir,
            fs::{create_dir_all, read_to_string, remove_dir_all, write},
            process::id,
        };

        let root = temp_dir().join(format!("hilen_toml_store_{}", id()));
        create_dir_all(&root).unwrap();
        OnDisk::<()>::set_root_path(&root);
        let store = OnDisk::<Data>::new("data.toml");

        let data = Data {
            number: 5,
            string: "a".to_string(),
        };
        store.set(data.clone());
        assert_eq!(
            read_to_string(root.join("data.toml")).unwrap(),
            "number = 5\nstring = \"a\"\n"
        );
        assert_eq!(store.get(), Some(data));

        let by_hand = "# how many\nnumber = 5   # five\n\n# the name\nstring = \"a\"\n";
        write(root.join("data.toml"), by_hand).unwrap();
        store.set(Data {
            number: 6,
            string: "a".to_string(),
        });
        assert_eq!(
            read_to_string(root.join("data.toml")).unwrap(),
            "# how many\nnumber = 6   # five\n\n# the name\nstring = \"a\"\n"
        );

        // A list with only another layout than the plain one stays as typed.
        let words = OnDisk::<Words>::new("words.toml");
        write(
            root.join("words.toml"),
            "words = [\"a\",\"b\"]  # kept\ncount = 1\n",
        )
        .unwrap();
        words.set(Words {
            words: vec!["a".to_string(), "b".to_string()],
            count: 2,
        });
        assert_eq!(
            read_to_string(root.join("words.toml")).unwrap(),
            "words = [\"a\",\"b\"]  # kept\ncount = 2\n"
        );

        write(root.join("data.toml"), "number = ").unwrap();
        assert_eq!(store.get(), None);

        remove_dir_all(&root).unwrap();
    }

    #[test]
    fn paths() {
        assert!(Paths::executable_name().starts_with("hilen"));
    }
}
