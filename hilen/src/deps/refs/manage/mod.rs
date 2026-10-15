use std::collections::BTreeMap;

mod data_manager;
mod exists_managed;
#[cfg(target_arch = "wasm32")]
mod file_xhr;
mod managed;
mod resource_loader;
mod tests;

pub(crate) use data_manager::download_loaded;
pub use data_manager::{DataManager, InFlightDownloads, fetch_bytes, fetch_bytes_with};
#[cfg(not_wasm)]
pub(crate) use data_manager::{Fetched, fetch_if_changed};
pub use exists_managed::ExistsManaged;
pub use resource_loader::ResourceLoader;

pub type DataStorage<T> = BTreeMap<String, crate::deps::refs::Own<T>>;

pub trait Managed: 'static + ResourceLoader + DataManager<Self> {}
