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
#[cfg(not_wasm)]
use reqwest::{
    StatusCode,
    header::{ETAG, IF_NONE_MATCH},
};
use web_time::Instant;

use crate::deps::{
    hreads::on_main,
    netrun::rest::client,
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
    fetch_bytes_with(url, &[]).await
}

/// `fetch_bytes` with request headers, for a server behind a login. A 404
/// page is an error here, not bytes, or it would fail later in a parser.
pub async fn fetch_bytes_with(url: &str, headers: &[(&str, &str)]) -> Result<Vec<u8>> {
    let mut request = client().get(absolute_url(url).as_ref());
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let data = request.send().await?.error_for_status()?.bytes().await?;
    Ok(data.to_vec())
}

/// What a conditional fetch brought back.
#[cfg(not_wasm)]
pub(crate) enum Fetched {
    /// The server still has the picture the etag names.
    NotModified,
    Fresh {
        bytes: Vec<u8>,
        etag:  Option<String>,
    },
}

/// `fetch_bytes_with` that also sends `etag` as `If-None-Match` when there
/// is one, and hands back the etag of a fresh answer.
#[cfg(not_wasm)]
pub(crate) async fn fetch_if_changed(
    url: &str,
    headers: &[(&str, &str)],
    etag: Option<&str>,
) -> Result<Fetched> {
    let mut request = client().get(url);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    if let Some(etag) = etag {
        request = request.header(IF_NONE_MATCH, etag);
    }

    let response = request.send().await?;
    if response.status() == StatusCode::NOT_MODIFIED {
        return Ok(Fetched::NotModified);
    }
    let response = response.error_for_status()?;
    let etag = response
        .headers()
        .get(ETAG)
        .and_then(|value| value.to_str().ok())
        .map(ToString::to_string);
    Ok(Fetched::Fresh {
        bytes: response.bytes().await?.to_vec(),
        etag,
    })
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

    fn weak_with_name(name: &str) -> Option<Weak<T>> {
        Some(Self::storage().get(name)?.weak())
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

        // The pending bytes stay until the entry is stored. Two threads
        // asking at once then both decode the real file, and neither
        // falls through to a path that a browser does not have.
        let new = Own::new(match T::pending_data(&name) {
            Some(data) => T::load_data(&data, &name),
            None => T::load_path(&Self::full_path(&name)),
        });

        let stored = insert_or_existing(Self::storage_mut(), name.clone(), new);
        T::pending_loaded(&name);
        stored
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
        Self::download_from(name, fetch_bytes(url)).await
    }

    /// Loads the resource `name` from the bytes `source` gives. The same
    /// name asked for while that runs waits for it instead of starting a
    /// second one, and `source` runs only for the first.
    #[allow(async_fn_in_trait)]
    async fn download_from(
        name: impl ToString,
        source: impl Future<Output = Result<Vec<u8>>>,
    ) -> Result<Weak<T>> {
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

        let started = Instant::now();
        let data = source.await?;
        let fetched = started.elapsed();

        let load_started = Instant::now();
        let loaded = Self::load(&data, &name);

        log::debug!(
            "Asset {name}: {} bytes fetched in {} ms, loaded in {} ms",
            data.len(),
            fetched.as_millis(),
            load_started.elapsed().as_millis()
        );

        Ok(loaded)
    }
}
