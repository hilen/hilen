use std::path::Path;

pub trait ResourceLoader: Sized {
    fn load_path(path: &Path) -> Self;
    fn load_data(data: &[u8], name: impl ToString) -> Self;

    /// File bytes the type already holds for `name` and has not decoded
    /// yet. `DataManager::get` loads from them before it tries a path.
    fn pending_data(_name: &str) -> Option<Vec<u8>> {
        None
    }

    /// `name` is in the store now, its pending bytes can go.
    fn pending_loaded(_name: &str) {}
}
