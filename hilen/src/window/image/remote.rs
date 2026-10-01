//! Images from the network. On top of the managed download: request headers
//! for a server behind a login, a disk cache so a second launch does not
//! fetch again, and a bound on the memory the downloaded images hold.

use std::{path::PathBuf, sync::atomic::Ordering};
#[cfg(not_wasm)]
use std::{thread, time::Duration};

use anyhow::Result;
#[cfg(not_wasm)]
use log::error;

#[cfg(wasm)]
use crate::deps::refs::manage::fetch_bytes_with;
#[cfg(not_wasm)]
use crate::window::image::disk_cache::DiskCache;
use crate::{
    deps::{
        hreads::on_main,
        refs::{__internal_deps::Mutex, Weak, manage::DataManager},
    },
    window::{Window, image::Image},
};

/// Frames an image counts as on screen after it was last drawn, so the bound
/// never frees what the user is looking at.
const ON_SCREEN_FRAMES: u64 = 2;

struct Remote {
    #[cfg(not_wasm)]
    cache: DiskCache,
    /// Bytes the downloaded images may hold, 0 for no bound.
    limit: usize,
    /// The name of every downloaded image still alive, with its bytes.
    held:  Vec<(String, usize)>,
}

static REMOTE: Mutex<Remote> = Mutex::new(Remote {
    #[cfg(not_wasm)]
    cache:                  DiskCache {
        dir:         None,
        limit:       0,
        recheck_age: None,
    },
    limit:                  0,
    held:                   Vec::new(),
});

impl Image {
    /// Fetches an image and stores it under `name`, see `download_with`.
    pub async fn download(name: impl ToString, url: &str) -> Result<Weak<Image>> {
        Self::download_with(name, url, &[]).await
    }

    /// Fetches an image with these request headers, like a session token,
    /// and stores it under `name`. The same name asked again gives the stored
    /// image back with no request. With a cache folder set the bytes come
    /// from disk when they are there, and go to disk when they are fetched.
    pub async fn download_with(
        name: impl ToString,
        url: &str,
        headers: &[(&str, &str)],
    ) -> Result<Weak<Image>> {
        let name = name.to_string();
        let image = Self::download_from(&name, bytes_of(url, headers)).await?;

        let bytes = image.size.width as usize * image.size.height as usize * 4;
        {
            let mut remote = REMOTE.lock();
            if !remote.held.iter().any(|(held, _)| *held == name) {
                remote.held.push((name, bytes));
            }
        }
        on_main(enforce_limit);
        Ok(image)
    }

    /// The folder downloaded images are kept in between launches, none for
    /// no disk cache, the default. A file is named by the hash of its url
    /// and is never fetched again while it is there, so a url must always
    /// give the same picture. A browser has its own cache, there this does
    /// nothing.
    pub fn set_download_cache_dir(dir: Option<PathBuf>) {
        #[cfg(not_wasm)]
        {
            REMOTE.lock().cache.dir = dir;
            trim_cache();
        }
        #[cfg(wasm)]
        log::debug!("No image disk cache in a browser, {dir:?} is not used");
    }

    /// The most bytes the cache folder holds, 0 for no bound, the default.
    /// Over it, the files used longest ago are deleted, at once and after
    /// every download that writes a file.
    pub fn set_download_cache_limit(bytes: u64) {
        #[cfg(not_wasm)]
        {
            REMOTE.lock().cache.limit = bytes;
            trim_cache();
        }
        #[cfg(wasm)]
        log::debug!("No image disk cache in a browser, the bound of {bytes} bytes is not used");
    }

    /// How many seconds a cached file is trusted before the server is asked
    /// about it again, none for never, the default. The question carries
    /// the `ETag` the server sent with the file, so a picture that did not
    /// change costs 1 small request and no download. When the server cannot
    /// be reached the cached file is used.
    pub fn set_download_cache_recheck_age(seconds: Option<u64>) {
        #[cfg(not_wasm)]
        {
            REMOTE.lock().cache.recheck_age = seconds.map(Duration::from_secs);
        }
        #[cfg(wasm)]
        log::debug!("No image disk cache in a browser, the recheck age {seconds:?} is not used");
    }

    /// The most bytes of pixels downloaded images hold, 0 for no bound, the
    /// default. Over it, the images drawn longest ago are freed until the
    /// rest fits. An image on screen is never freed. A view that held a
    /// freed image draws nothing until the app downloads it again, which
    /// the disk cache makes cheap.
    pub fn set_download_memory_limit(bytes: usize) {
        REMOTE.lock().limit = bytes;
        on_main(enforce_limit);
    }
}

/// The bytes of a url, from the disk cache when it has them.
#[cfg(not_wasm)]
async fn bytes_of(url: &str, headers: &[(&str, &str)]) -> Result<Vec<u8>> {
    let cache = REMOTE.lock().cache.clone();
    cache.bytes_of(url, headers).await
}

/// A browser has its own cache, the cache settings do nothing there.
#[cfg(wasm)]
async fn bytes_of(url: &str, headers: &[(&str, &str)]) -> Result<Vec<u8>> {
    fetch_bytes_with(url, headers).await
}

/// Brings the cache folder under its bound on a thread of its own, the
/// folder can hold thousands of files and the caller is the main thread.
#[cfg(not_wasm)]
fn trim_cache() {
    let cache = REMOTE.lock().cache.clone();
    let Some(dir) = cache.dir.clone() else {
        return;
    };
    if cache.limit == 0 {
        return;
    }
    // No file is in use by this call, so nothing is exempt.
    let spawned = thread::Builder::new()
        .name("image-cache".into())
        .spawn(move || cache.prune(&dir));
    if let Err(err) = spawned {
        error!("image cache: the trim thread did not start, {err}");
    }
}

/// Frees the downloaded images drawn longest ago until the rest fits the
/// bound. Main thread, an image has to be dropped there.
fn enforce_limit() {
    let mut remote = REMOTE.lock();

    // What the app freed by itself is no longer held.
    let mut alive: Vec<(String, usize, u64)> = remote
        .held
        .iter()
        .filter_map(|(name, bytes)| {
            let image = Image::get_existing(name)?;
            Some((name.clone(), *bytes, image.drawn_at.load(Ordering::Relaxed)))
        })
        .collect();

    let limit = remote.limit;
    let mut total: usize = alive.iter().map(|(_, bytes, _)| bytes).sum();
    if limit > 0 && total > limit {
        let now = Window::render_frame();
        alive.sort_by_key(|(_, _, drawn_at)| *drawn_at);
        alive.retain(|(name, bytes, drawn_at)| {
            let on_screen = *drawn_at > 0 && now.saturating_sub(*drawn_at) <= ON_SCREEN_FRAMES;
            if total <= limit || on_screen {
                return true;
            }
            Image::free_with_name(name);
            total -= bytes;
            false
        });
    }

    remote.held = alive.into_iter().map(|(name, bytes, _)| (name, bytes)).collect();
}
