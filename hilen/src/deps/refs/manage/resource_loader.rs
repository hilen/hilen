use std::path::Path;

pub trait ResourceLoader: Sized {
    fn load_path(path: &Path) -> Self;
    fn load_data(data: &[u8], name: impl ToString) -> Self;
}
