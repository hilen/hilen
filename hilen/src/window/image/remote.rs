//! Images from the network. On top of the managed download: request headers
//! for a server behind a login, a disk cache so a second launch does not
//! fetch again, and a bound on the memory the downloaded images hold.

use std::{
    fs::{create_dir_all, read, write},
    path::PathBuf,
    sync::atomic::Ordering,
};

use anyhow::Result;
use log::warn;
use sha2::{Digest, Sha256};

use crate::{
    deps::{
        hreads::on_main,
        refs::{
            __internal_deps::Mutex,
            Weak,
            manage::{DataManager, fetch_bytes_with},
        },
    },
    window::{Window, image::Image},
};

/// Frames an image counts as on screen after it was last drawn, so the bound
/// never frees what the user is looking at.
const ON_SCREEN_FRAMES: u64 = 2;

struct Remote {
    cache_dir: Option<PathBuf>,
    /// Bytes the downloaded images may hold, 0 for no bound.
    limit:     usize,
    /// The name of every downloaded image still alive, with its bytes.
    held:      Vec<(String, usize)>,
}

static REMOTE: Mutex<Remote> = Mutex::new(Remote {
    cache_dir: None,
    limit:     0,
    held:      Vec::new(),
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
        REMOTE.lock().cache_dir = dir;
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
async fn bytes_of(url: &str, headers: &[(&str, &str)]) -> Result<Vec<u8>> {
    let cached = cache_file(url);
    if let Some(path) = &cached
        && let Ok(bytes) = read(path)
    {
        return Ok(bytes);
    }

    let bytes = fetch_bytes_with(url, headers).await?;

    if let Some(path) = &cached {
        let stored = path.parent().map_or(Ok(()), create_dir_all).and_then(|()| write(path, &bytes));
        if let Err(err) = stored {
            warn!("image cache: {} not written, {err}", path.display());
        }
    }
    Ok(bytes)
}

/// Where the cache keeps the bytes of a url, none with no cache folder.
fn cache_file(url: &str) -> Option<PathBuf> {
    if cfg!(target_arch = "wasm32") {
        return None;
    }
    let dir = REMOTE.lock().cache_dir.clone()?;
    Some(dir.join(hex::encode(Sha256::digest(url.as_bytes()))))
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
