//! Decodes the embedded images of a model off the main thread.

use super::parse::ModelSource;

/// Decodes every embedded image and builds its mip levels, spread over
/// the cores, so the upload on the main thread only makes the textures. A
/// map with 1510 png images froze its window while the main thread
/// decoded them. An image that does not decode keeps no levels, the upload
/// then loads it the old way and reports it. A browser decodes a jpeg
/// itself, so wasm leaves every image to the upload. Any thread.
pub(crate) fn decode(source: &mut ModelSource) {
    #[cfg(not_wasm)]
    {
        use std::{num::NonZeroUsize, thread};

        use crate::image::TextureRawData;

        let images = &mut source.images;
        if images.is_empty() {
            return;
        }
        let workers = thread::available_parallelism().map_or(1, NonZeroUsize::get).min(images.len());
        let chunk = images.len().div_ceil(workers);
        thread::scope(|scope| {
            for slice in images.chunks_mut(chunk) {
                scope.spawn(move || {
                    for image in slice {
                        match TextureRawData::from_file_bytes(&image.bytes) {
                            Ok(raw) => image.levels = Some(raw.into_levels()),
                            Err(err) => {
                                log::warn!("{} did not decode, the upload tries again: {err}", image.name);
                            }
                        }
                    }
                });
            }
        });
    }
    #[cfg(wasm)]
    let _ = source;
}

#[cfg(all(test, not_wasm))]
mod test {
    use std::{fs, path::PathBuf};

    use super::*;
    use crate::scene::model::parse::parse_glb;

    #[test]
    fn every_embedded_image_is_decoded_with_its_mips() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/models/textured_cube.glb");
        let mut source = parse_glb(&fs::read(&path).unwrap(), "textured_cube.glb").unwrap();
        assert!(source.images.iter().all(|image| image.levels.is_none()));

        decode(&mut source);

        let levels = source.images[0].levels.as_ref().expect("the jpeg decodes");
        assert_eq!(levels.channels, 4);
        assert!(levels.size.width > 0);
        #[cfg(feature = "gpu")]
        assert!(levels.levels.len() > 1, "the mips are built");
    }

    #[test]
    fn an_image_that_does_not_decode_is_left_to_the_upload() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/models/textured_cube.glb");
        let mut source = parse_glb(&fs::read(&path).unwrap(), "textured_cube.glb").unwrap();
        source.images[0].bytes = b"not a picture".to_vec();

        decode(&mut source);

        assert!(source.images[0].levels.is_none());
    }
}
