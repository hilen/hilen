use std::{
    fmt::{Debug, Formatter},
    fs::{self, remove_file},
    marker::PhantomData,
    path::{Path, PathBuf},
};

use parking_lot::Mutex;

use crate::{filesystem::Paths, store::storable::Storable};

static ROOT_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

fn set_value<T: serde::ser::Serialize>(value: T, path: &Path) {
    let json = serde_json::to_string_pretty(&value).expect("Failed to serialize data");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, json).expect("Failed to write to file");
}

fn get_value<T: Storable>(path: &Path) -> Option<T> {
    if !path.exists() {
        return None;
    }

    let json = fs::read_to_string(path).expect("Failed to read file");
    serde_json::from_str(&json).expect("Failet to parse json")
}

fn get_or_init_value<T: Storable + Default>(path: &Path) -> T {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    if !path.exists() {
        let new = T::default();
        set_value(&new, path);
        return new;
    }
    let json = fs::read_to_string(path).expect("Failed to read file");
    serde_json::from_str(&json).expect("Failet to parse json")
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
        set_value(val, &expand_tilde(self.full_path()));
    }

    pub fn get(&self) -> Option<T> {
        get_value(&expand_tilde(self.full_path()))
    }

    pub fn reset(&self) {
        remove_file(expand_tilde(self.full_path())).expect("Failed to remove file");
    }

    fn full_path(&self) -> PathBuf {
        if let Some(root) = &*ROOT_PATH.lock() {
            root.join(&self.path)
        } else {
            self.path.clone()
        }
    }
}

impl<T: Storable + Default> OnDisk<T> {
    pub fn edit(&self, edit: impl FnOnce(&mut T)) {
        let mut val = self.get_or_init();
        edit(&mut val);
        self.set(val);
    }

    pub fn get_or_init(&self) -> T {
        get_or_init_value(&expand_tilde(self.full_path()))
    }
}

impl<T> OnDisk<T> {
    pub fn set_root_path(path: impl AsRef<Path>) {
        *ROOT_PATH.lock() = Some(path.as_ref().to_path_buf());
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

    use crate::{filesystem::Paths, store::OnDisk};

    #[derive(Debug, PartialEq, Default, Serialize, Deserialize, Clone)]
    struct Data {
        number: i32,
        string: String,
    }

    static STORED: LazyLock<OnDisk<i32>> = LazyLock::new(|| OnDisk::new("stored_i32_test.json"));
    static STORED_STRUCT: LazyLock<OnDisk<Data>> = LazyLock::new(|| OnDisk::new("stored_struct_test.json"));
    static INITIALIZED: LazyLock<OnDisk<i32>> = LazyLock::new(|| OnDisk::new("initialized_i32_test.json"));

    fn check_send<T: Send>(_send: &T) {}
    fn check_sync<T: Sync>(_sync: &T) {}

    #[test]
    fn stored() {
        OnDisk::<()>::set_root_path("~/.test_on_disk/");

        check_send(&STORED);
        check_sync(&STORED);
        check_send(&STORED_STRUCT);
        check_sync(&STORED_STRUCT);

        STORED.set(10);
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
    fn paths() {
        assert!(Paths::executable_name().starts_with("hilen"));
    }
}
