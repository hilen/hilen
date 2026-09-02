use log::error;

use crate::{
    deps::refs::Weak,
    gm::color::Color,
    window::image::{DEFAULT_IMAGE_DATA, Image, ToImage},
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
    use crate::deps::refs::manage::DataManager;

    let path = Image::full_path(name);

    // Through the filesystem layer, not std::fs. Android assets live inside
    // the APK and only the AAssetManager can read them.
    crate::filesystem::read_bytes(&path)
        .inspect_err(|err| error!("Failed to read image file: {}. Error: {err}", path.display()))
        .ok()
}

#[cfg(wasm)]
fn svg_data(name: &str) -> Option<Vec<u8>> {
    let data = svg_sources::get(name);
    if data.is_none() {
        error!("No downloaded SVG data for {name}. Is its asset group loaded?");
    }
    data
}

impl ToImage for Tinted {
    fn to_image(&self) -> Weak<Image> {
        use crate::deps::refs::manage::DataManager;

        let stored_name = format!("{}:{}", self.name, self.tint.as_hex());

        // The rasterized image is cached under name plus tint. Checking
        // first skips the file read and the tint rewrite, which ran on
        // every call and every relayout.
        if let Some(cached) = Image::weak_with_name(&stored_name) {
            return cached;
        }

        let Some(data) = svg_data(&self.name) else {
            return Image::from_file_data(DEFAULT_IMAGE_DATA, &stored_name);
        };

        match std::str::from_utf8(&data) {
            Ok(text) => {
                let tinted = text.replace("#000000", &self.tint.as_hex());
                Image::from_file_data(tinted.as_bytes(), &stored_name)
            }
            Err(err) => {
                error!(
                    "Data for tinted image {} is not a valid string. Most likely is not an SVG: {err}. \
                     Returning default image",
                    self.name
                );
                Image::from_file_data(DEFAULT_IMAGE_DATA, &stored_name)
            }
        }
    }
}
