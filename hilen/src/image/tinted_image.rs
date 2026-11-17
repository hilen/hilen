use std::str::from_utf8;

use log::error;

use crate::{
    deps::refs::{Weak, manage::DataManager},
    gm::color::Color,
    image::{DEFAULT_IMAGE_DATA, Image, ToImage},
};

/// Image with tint color
/// Works only with SVG files
/// Implementation is very naive it replaces all #000000 strings (black hex) to
/// the hex of the tint. So all tinted elements in the SVG should be black
pub struct Tinted {
    pub tint: Color,
    pub name: String,
}

/// The browser has no filesystem to reread an SVG from, so the raw
/// bytes of every downloaded SVG stay in memory for tinting.
#[cfg(wasm)]
pub(crate) mod svg_sources {
    use std::collections::BTreeMap;

    use parking_lot::Mutex;

    static SOURCES: Mutex<BTreeMap<String, Vec<u8>>> = Mutex::new(BTreeMap::new());

    pub(crate) fn store(name: &str, data: &[u8]) {
        if std::path::Path::new(name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"))
        {
            SOURCES.lock().insert(name.to_string(), data.to_vec());
        }
    }

    pub(crate) fn get(name: &str) -> Option<Vec<u8>> {
        SOURCES.lock().get(name).cloned()
    }
}

#[cfg(not_wasm)]
fn svg_data(name: &str) -> Option<Vec<u8>> {
    let path = Image::full_path(name);

    // Through the filesystem layer, not std::fs. Android assets live inside
    // the APK and only the AAssetManager can read them.
    crate::filesystem::read_bytes(&path)
        .inspect_err(|err| error!("Failed to read image file: {}. Error: {err}", path.display()))
        .ok()
}

#[cfg(wasm)]
fn svg_data(name: &str) -> Option<Vec<u8>> {
    // A missing file is noted in `image/early.rs`, which logs an error once it
    // is sure that no asset group has it.
    svg_sources::get(name)
}

impl ToImage for Tinted {
    fn to_image(&self) -> Weak<Image> {
        // The rasterized image is cached under name plus tint. Checking
        // first skips the file read and the tint rewrite, which ran on
        // every call and every relayout.
        if let Some(cached) = Image::weak_with_name(&tinted_name(&self.name, self.tint)) {
            return cached;
        }

        match svg_data(&self.name) {
            Some(data) => tint_svg(&data, &self.name, self.tint),
            None => missing(&self.name, self.tint),
        }
    }
}

#[cfg(not_wasm)]
fn missing(name: &str, tint: Color) -> Weak<Image> {
    Image::from_file_data(DEFAULT_IMAGE_DATA, &tinted_name(name, tint))
}

/// The svg can still come with an asset group. The image is made see-through
/// under its real name and gets its picture when the file lands.
#[cfg(wasm)]
fn missing(name: &str, tint: Color) -> Weak<Image> {
    let stored = tinted_name(name, tint);
    crate::image::early::note(name, stored.clone(), Some(tint.as_hex()));
    Image::from_file_data(crate::image::image::WAITING_IMAGE_DATA, &stored)
}

/// The svg text with its black, `#000000`, put in as `hex`. `None` when the
/// bytes are no text.
pub(crate) fn tint_text(data: &[u8], hex: &str) -> Option<Vec<u8>> {
    from_utf8(data).ok().map(|text| text.replace("#000000", hex).into_bytes())
}

fn tinted_name(name: &str, tint: Color) -> String {
    format!("{name}:{}", tint.as_hex())
}

/// Tints SVG bytes the app or the engine already holds, like an icon
/// built in with `include_bytes`, the same way [`Tinted`] tints a file.
pub fn tint_svg(data: &[u8], name: &str, tint: Color) -> Weak<Image> {
    let stored_name = tinted_name(name, tint);

    if let Some(cached) = Image::weak_with_name(&stored_name) {
        return cached;
    }

    if let Some(tinted) = tint_text(data, &tint.as_hex()) {
        Image::from_file_data(&tinted, &stored_name)
    } else {
        error!(
            "Data for tinted image {name} is not a valid string, most likely no SVG. Returning default image"
        );
        Image::from_file_data(DEFAULT_IMAGE_DATA, &stored_name)
    }
}
