use std::collections::BTreeMap;

mod data_manager;
mod exists_managed;
mod managed;
mod resource_loader;
mod tests;

pub use data_manager::{DataManager, InFlightDownloads, fetch_bytes, fetch_bytes_with};
pub use exists_managed::ExistsManaged;
pub use resource_loader::ResourceLoader;

pub type DataStorage<T> = BTreeMap<String, crate::deps::refs::Own<T>>;

pub trait Managed: 'static + ResourceLoader + DataManager<Self> {}
