//! Pictures of a downloaded asset group that nothing has asked for yet.
//!
//! A browser has no filesystem, so a group has to be in memory before a
//! sync `Image::get` can serve it. Decoding the whole group up front cost
//! a page its life on a small device: an LG C1 TV dropped the WebGL
//! context of the demo at about 100 MB of textures plus 109 MB of wasm
//! memory, most of it for pictures no page on screen used. So a group
//! download keeps only the file bytes here, and `Image::get` decodes a
//! picture the first time a view names it.

use std::{collections::BTreeMap, sync::Arc};

use anyhow::{Result, anyhow};
use event_listener::Event;
use parking_lot::Mutex;
use web_time::Instant;

use crate::{
    deps::refs::manage::{DataManager, fetch_bytes},
    window::image::{Image, svg_sources},
};

enum Slot {
    Fetching(Arc<Event>),
    Ready(Vec<u8>),
}

static SLOTS: Mutex<BTreeMap<String, Slot>> = Mutex::new(BTreeMap::new());

/// The bytes of `name`, when its file is here and nothing decoded it yet.
pub(crate) fn get(name: &str) -> Option<Vec<u8>> {
    match SLOTS.lock().get(name) {
        Some(Slot::Ready(data)) => Some(data.clone()),
        _ => None,
    }
}

/// Drops the bytes of `name` once its picture is in the image store.
pub(crate) fn forget(name: &str) {
    let mut slots = SLOTS.lock();
    if matches!(slots.get(name), Some(Slot::Ready(_))) {
        slots.remove(name);
    }
}

/// Fetches the file of `name` and keeps its bytes for the first
/// `Image::get`. The same name asked for while that runs waits for it
/// instead of fetching twice.
pub(crate) async fn download(name: &str, url: &str) -> Result<()> {
    let waiter = {
        let mut slots = SLOTS.lock();

        if Image::get_existing(name).is_some() {
            return Ok(());
        }

        match slots.get(name) {
            Some(Slot::Ready(_)) => return Ok(()),
            Some(Slot::Fetching(event)) => Some(event.listen()),
            None => {
                slots.insert(name.to_string(), Slot::Fetching(Arc::new(Event::new())));
                None
            }
        }
    };

    if let Some(listener) = waiter {
        listener.await;
        return if SLOTS.lock().contains_key(name) || Image::get_existing(name).is_some() {
            Ok(())
        } else {
            Err(anyhow!(
                "Download of '{name}' failed in the task which started it"
            ))
        };
    }

    let started = Instant::now();
    let fetched = fetch_bytes(url).await;

    if let Ok(data) = &fetched {
        log::debug!(
            "Asset {name}: {} bytes fetched in {} ms, decoded on first use",
            data.len(),
            started.elapsed().as_millis()
        );
    }

    let mut slots = SLOTS.lock();
    let Some(Slot::Fetching(event)) = slots.remove(name) else {
        return Err(anyhow!("The fetch slot of '{name}' is gone"));
    };

    let result = fetched.map(|data| {
        // A tint needs the svg text before anything decoded the picture.
        svg_sources::store(name, &data);
        slots.insert(name.to_string(), Slot::Ready(data));
    });

    drop(slots);
    event.notify(usize::MAX);

    result
}
