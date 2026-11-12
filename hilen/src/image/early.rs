//! Pictures a view asked for in a browser before their file came.
//!
//! A browser has no filesystem, so `Image::get` or a `Tinted` icon of a file
//! whose asset group is still downloading finds no bytes. The image is then
//! made see-through under its real name, so every view keeps the same
//! `Weak<Image>`, and the name is noted here. When the bytes land, the real
//! picture goes into that same image and the next frame is drawn whole. The
//! control center and beekeeper both showed placeholder icons for good before
//! this, from 1 early call each.

use std::collections::BTreeMap;

use parking_lot::Mutex;

/// 1 image that waits for its file: the name it is stored under, and the
/// hex of its tint when it is a tinted svg.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Waiting {
    stored: String,
    tint:   Option<String>,
}

/// The images that wait, by the name of their file.
static WAITING: Mutex<BTreeMap<String, Vec<Waiting>>> = Mutex::new(BTreeMap::new());

/// Notes that the image `stored` was made before the bytes of `file` came.
/// `tint` is the hex the svg is tinted with, `None` for a plain picture.
pub(crate) fn note(file: &str, stored: String, tint: Option<String>) {
    let mut waiting = WAITING.lock();
    let list = waiting.entry(file.to_string()).or_default();
    let entry = Waiting { stored, tint };
    if !list.contains(&entry) {
        log::debug!("{} waits for the file {file}", entry.stored);
        list.push(entry);
    }
    drop(waiting);
    #[cfg(wasm)]
    if crate::assets::has_file(file) == Some(false) {
        report(file);
    }
}

/// An image that waits for a file no asset group has stays see-through for
/// good, most often a wrong name.
#[cfg(wasm)]
fn report(file: &str) {
    log::error!("No asset group has the file {file}, its image stays see-through");
}

/// Logs every waiting file that no asset group has. Called when the manifest
/// came with the boot group, the early asks of a page come before it.
#[cfg(wasm)]
pub(crate) fn report_unknown() {
    let files: Vec<String> = WAITING.lock().keys().cloned().collect();
    for file in files {
        if crate::assets::has_file(&file) == Some(false) {
            report(&file);
        }
    }
}

/// Whether an image waits for `file`.
pub(crate) fn is_waiting(file: &str) -> bool {
    WAITING.lock().contains_key(file)
}

/// The bytes for each image that waited for `file`, tinted where it was
/// tinted. Nothing waits for `file` after it.
pub(crate) fn take(file: &str, data: &[u8]) -> Vec<(String, Vec<u8>)> {
    let Some(list) = WAITING.lock().remove(file) else {
        return Vec::new();
    };
    list.into_iter()
        .filter_map(|entry| {
            let bytes = match &entry.tint {
                None => Some(data.to_vec()),
                Some(hex) => crate::image::tinted_image::tint_text(data, hex),
            };
            if bytes.is_none() {
                log::error!("{file} came for {}, but it is no svg text to tint", entry.stored);
            }
            bytes.map(|bytes| (entry.stored, bytes))
        })
        .collect()
}

/// Puts the picture of `file` into every image that waited for it. Called
/// when a download of the file lands.
#[cfg(wasm)]
pub(crate) fn landed(file: &str, data: &[u8]) {
    if !is_waiting(file) {
        return;
    }
    let file = file.to_string();
    let data = data.to_vec();
    crate::deps::hreads::on_main(move || {
        use crate::{
            deps::refs::manage::{DataManager, ResourceLoader},
            image::Image,
        };

        let ready = take(&file, &data);
        for (stored, bytes) in &ready {
            if let Some(mut image) = Image::weak_with_name(stored) {
                *image = Image::load_data(bytes, stored);
                log::debug!("{stored} got its picture after the file {file} came");
            }
        }
        crate::image::pending::forget(&file);
        #[cfg(feature = "gpu")]
        if !ready.is_empty() {
            crate::ui::damage::whole_next("a picture asked for before its file came");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const SVG: &[u8] = br##"<svg><path fill="#000000"/></svg>"##;

    #[test]
    fn a_waiting_image_gets_its_bytes_tinted_once() {
        note(
            "early-a.svg",
            "early-a.svg:#ff0000".into(),
            Some("#ff0000".into()),
        );
        note(
            "early-a.svg",
            "early-a.svg:#ff0000".into(),
            Some("#ff0000".into()),
        );
        note("early-a.svg", "early-a.svg".into(), None);
        assert!(is_waiting("early-a.svg"));

        let ready = take("early-a.svg", SVG);
        assert_eq!(
            ready,
            vec![
                (
                    "early-a.svg:#ff0000".to_string(),
                    br##"<svg><path fill="#ff0000"/></svg>"##.to_vec()
                ),
                ("early-a.svg".to_string(), SVG.to_vec()),
            ]
        );

        assert!(!is_waiting("early-a.svg"));
        assert_eq!(take("early-a.svg", SVG), Vec::new());
    }

    #[test]
    fn a_file_nobody_waits_for_gives_nothing() {
        assert!(!is_waiting("early-none.png"));
        assert_eq!(take("early-none.png", &[1, 2, 3]), Vec::new());
    }
}
