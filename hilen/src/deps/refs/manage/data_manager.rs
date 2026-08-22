use std::{
    borrow::Cow,
    collections::{BTreeMap, btree_map::Entry},
    mem::transmute,
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Result, anyhow};
use event_listener::Event;

use crate::deps::{
    hreads::on_main,
    refs::{
        __internal_deps::{Mutex, RwLockReadGuard, RwLockWriteGuard},
        Own, Weak,
        manage::{DataStorage, Managed},
    },
};

pub type InFlightDownloads = Mutex<BTreeMap<String, Arc<Event>>>;

/// Publishes `new` under `name` unless another thread already did.
/// The losing copy is dropped on the main thread. Own panics when
/// dropped anywhere else.
fn insert_or_existing<T: Managed>(
    mut storage: RwLockWriteGuard<'static, DataStorage<T>>,
    name: String,
    new: Own<T>,
) -> Weak<T> {
    match storage.entry(name) {
        Entry::Occupied(existing) => {
            let existing = existing.get().weak();
            drop(storage);
            on_main(move || drop(new));
            existing
        }
        Entry::Vacant(slot) => slot.insert(new).weak(),
    }
}

/// Fetch needs an absolute url. A relative one is resolved against the
/// document base url, so a page hosted under a path prefix points its
/// asset fetches there with a base tag instead of hitting the site root.
#[cfg(target_arch = "wasm32")]
fn absolute_url(url: &str) -> Cow<'_, str> {
    if url.contains("://") {
        return url.into();
    }

    let base = web_sys::window()
        .expect("Failed to get browser window")
        .document()
        .expect("Failed to get browser document")
        .base_uri()
        .expect("Failed to get document base URI")
        .unwrap_or_default();

    web_sys::Url::new_with_base(url, &base)
        .expect("Failed to resolve url against document base")
        .href()
        .into()
}

#[cfg(not(target_arch = "wasm32"))]
fn absolute_url(url: &str) -> Cow<'_, str> {
    url.into()
}

/// Plain HTTP fetch with the same root relative url resolution the
/// managed downloads use. For data that is not a managed resource,
/// like the asset manifest.
pub async fn fetch_bytes(url: &str) -> Result<Vec<u8>> {
    let data = reqwest::get(absolute_url(url).as_ref())
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    Ok(data.to_vec())
}

pub trait DataManager<T: Managed> {
    fn root_path() -> &'static Path;
    fn set_root_path(path: impl Into<PathBuf>);

    fn storage() -> RwLockReadGuard<'static, DataStorage<T>>;
    fn storage_mut() -> RwLockWriteGuard<'static, DataStorage<T>>;

    fn in_flight_downloads() -> &'static InFlightDownloads;

    fn full_path(name: &str) -> PathBuf {
        Self::root_path().join(name)
    }

    fn free_with_name(name: impl ToString) {
        Self::storage_mut().remove(&name.to_string());
    }

    fn free(self: Weak<Self>) {
        if self.is_null() {
            return;
        }
        let mut storage = Self::storage_mut();
        let key = storage
            .iter()
            .find(|(_, val)| val.addr() == self.addr())
            .expect("Failed to find managed object to free.")
            .0
            .clone();
        storage.remove(&key);
    }

    fn store_with_name<E>(name: &str, create: impl FnOnce() -> Result<T, E>) -> Result<Weak<T>, E> {
        if let Some(entry) = Self::storage().get(name) {
            return Ok(entry.weak());
        }

        let entry = Own::new(create()?);

        Ok(insert_or_existing(Self::storage_mut(), name.to_owned(), entry))
    }

    unsafe fn get_static(self: Weak<Self>) -> &'static T {
        let storage = Self::storage();

        let rf = storage
            .iter()
            .find(|(_, val)| val.addr() == self.addr())
            .expect("Failed to get_static managed")
            .1;

        unsafe { transmute(rf.deref()) }
    }

    fn get_existing(name: impl ToString) -> Option<Weak<T>> {
        Self::storage().get(&name.to_string()).map(Own::weak)
    }

    fn get(name: impl ToString) -> Weak<T> {
        let name = name.to_string();

        if let Some(existing) = Self::storage().get(&name) {
            return existing.weak();
        }

        let new = Own::new(T::load_path(&Self::full_path(&name)));

        insert_or_existing(Self::storage_mut(), name, new)
    }

    fn load(data: &[u8], name: impl ToString) -> Weak<T> {
        let name = name.to_string();

        if let Some(existing) = Self::storage().get(&name) {
            return existing.weak();
        }

        let new = Own::new(T::load_data(data, &name));

        insert_or_existing(Self::storage_mut(), name, new)
    }

    #[allow(async_fn_in_trait)]
    async fn download(name: impl ToString, url: &str) -> Result<Weak<T>> {
        /// Wakes the waiters even if the leading download errors,
        /// panics, or its task is dropped mid await.
        struct FinishGuard {
            in_flight: &'static InFlightDownloads,
            name:      String,
        }

        impl Drop for FinishGuard {
            fn drop(&mut self) {
                if let Some(event) = self.in_flight.lock().remove(&self.name) {
                    event.notify(usize::MAX);
                }
            }
        }

        let name = name.to_string();

        if let Some(existing) = Self::get_existing(&name) {
            return Ok(existing);
        }

        let waiter = {
            let mut in_flight = Self::in_flight_downloads().lock();

            if let Some(existing) = Self::storage().get(&name) {
                return Ok(existing.weak());
            }

            if let Some(event) = in_flight.get(&name) {
                Some(event.listen())
            } else {
                in_flight.insert(name.clone(), Arc::new(Event::new()));
                None
            }
        };

        if let Some(listener) = waiter {
            listener.await;
            return Self::get_existing(&name)
                .ok_or_else(|| anyhow!("Download of '{name}' failed in the task which started it"));
        }

        let _guard = FinishGuard {
            in_flight: Self::in_flight_downloads(),
            name:      name.clone(),
        };

        // Without the status check a 404 page would be stored as the
        // resource bytes and fail later in the parser, far from here.
        let data = reqwest::get(absolute_url(url).as_ref())
            .await?
            .error_for_status()?
            .bytes()
            .await?;

        Ok(Self::load(&data, &name))
    }
}
