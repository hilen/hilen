//! The pixels of a photo to crop, decoded once and turned upright by its
//! EXIF orientation, and the png of a cut out of them.

use anyhow::{Result, anyhow};
use image::{
    ExtendedColorType, ImageEncoder, RgbaImage,
    codecs::png::PngEncoder,
    imageops::{self, FilterType},
};

use crate::gm::{LossyConvert, flat::Rect};

/// The longest side the view keeps of a photo. A cut for an avatar needs far
/// less, and a phone photo of 12 megapixels would hold 48 MB.
pub(crate) const MAX_SIDE: u32 = 2048;

/// The photo in `bytes`, upright, its longest side at most `MAX_SIDE`.
#[cfg(not_wasm)]
pub(crate) async fn decode(bytes: Vec<u8>) -> Result<RgbaImage> {
    tokio::task::spawn_blocking(move || decode_now(&bytes, MAX_SIDE)).await?
}

/// A browser turns the photo itself, it reads the orientation while it
/// decodes, and the wasm build links no jpeg decoder.
#[cfg(wasm)]
pub(crate) async fn decode(bytes: Vec<u8>) -> Result<RgbaImage> {
    let raw = crate::image::decode_upright(&bytes, MAX_SIDE).await?;
    RgbaImage::from_raw(raw.size.width, raw.size.height, raw.data)
        .ok_or_else(|| anyhow!("The browser gave fewer pixels than the size of the photo"))
}

#[cfg(not_wasm)]
pub(crate) fn decode_now(bytes: &[u8], max_side: u32) -> Result<RgbaImage> {
    use image::{DynamicImage, ImageDecoder, ImageReader};

    let mut decoder = ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()?
        .into_decoder()?;
    let orientation = decoder.orientation()?;
    let mut picture = DynamicImage::from_decoder(decoder)?;
    picture.apply_orientation(orientation);
    if picture.width().max(picture.height()) > max_side {
        picture = picture.resize(max_side, max_side, FilterType::Triangle);
    }
    Ok(picture.into_rgba8())
}

/// The `cut` of `picture`, in its pixels, as a png `side` pixels square.
pub(crate) fn cut_png(picture: &RgbaImage, cut: Rect, side: u32) -> Result<Vec<u8>> {
    let (width, height) = picture.dimensions();
    if width == 0 || height == 0 || side == 0 {
        return Err(anyhow!("Nothing to cut, the photo is {width}x{height}"));
    }
    let x: u32 = cut.origin.x.round().clamp(0.0, (width - 1).lossy_convert()).lossy_convert();
    let y: u32 = cut.origin.y.round().clamp(0.0, (height - 1).lossy_convert()).lossy_convert();
    let length: u32 = cut.size.width.round().max(1.0).lossy_convert();
    let length = length.min(width - x).min(height - y);

    let piece = imageops::crop_imm(picture, x, y, length, length).to_image();
    let small = imageops::resize(&piece, side, side, FilterType::Triangle);
    let mut png = Vec::new();
    PngEncoder::new(&mut png).write_image(small.as_raw(), side, side, ExtendedColorType::Rgba8)?;
    Ok(png)
}

#[cfg(test)]
mod test {
    use image::{ExtendedColorType, ImageEncoder, Rgba, RgbaImage, codecs::jpeg::JpegEncoder};

    use super::{cut_png, decode_now};
    use crate::gm::flat::Rect;

    /// A jpeg 40 pixels wide and 20 high, red on the left half and blue on
    /// the right, with the EXIF orientation `orientation`.
    fn jpeg(orientation: u8) -> Vec<u8> {
        let picture = RgbaImage::from_fn(40, 20, |x, _| {
            if x < 20 {
                Rgba([255, 0, 0, 255])
            } else {
                Rgba([0, 0, 255, 255])
            }
        });
        let rgb = image::DynamicImage::ImageRgba8(picture).into_rgb8();
        let mut bytes = Vec::new();
        let mut encoder = JpegEncoder::new_with_quality(&mut bytes, 95);
        encoder.set_exif_metadata(exif(orientation)).unwrap();
        encoder.write_image(rgb.as_raw(), 40, 20, ExtendedColorType::Rgb8).unwrap();
        bytes
    }

    /// The smallest EXIF block, a big endian TIFF header and 1 entry, the
    /// orientation.
    fn exif(orientation: u8) -> Vec<u8> {
        let mut exif = b"MM\0\x2a\0\0\0\x08".to_vec();
        exif.extend_from_slice(&[0, 1, 0x01, 0x12, 0, 3, 0, 0, 0, 1, 0, orientation, 0, 0]);
        exif.extend_from_slice(&[0, 0, 0, 0]);
        exif
    }

    fn reddish(pixel: Rgba<u8>) -> bool {
        pixel[0] > 200 && pixel[2] < 60
    }

    #[test]
    fn a_photo_with_no_turn_stays_as_it_is() {
        let picture = decode_now(&jpeg(1), 2048).unwrap();
        assert_eq!(picture.dimensions(), (40, 20));
        assert!(reddish(*picture.get_pixel(2, 10)));
    }

    #[test]
    fn a_phone_photo_is_turned_upright() {
        // 6 is a phone held upright, the photo is shown turned a quarter
        // to the right, so its left edge is on top.
        let picture = decode_now(&jpeg(6), 2048).unwrap();
        assert_eq!(picture.dimensions(), (20, 40));
        assert!(reddish(*picture.get_pixel(10, 2)));
        assert!(!reddish(*picture.get_pixel(10, 37)));
    }

    #[test]
    fn a_big_photo_is_made_smaller() {
        let picture = decode_now(&jpeg(1), 10).unwrap();
        assert_eq!(picture.dimensions(), (10, 5));
    }

    #[test]
    fn a_cut_is_a_square_png_of_the_side() {
        let picture = decode_now(&jpeg(1), 2048).unwrap();
        let png = cut_png(&picture, Rect::new(0.0, 0.0, 20.0, 20.0), 8).unwrap();
        let cut = image::load_from_memory(&png).unwrap().into_rgba8();
        assert_eq!(cut.dimensions(), (8, 8));
        assert!(reddish(*cut.get_pixel(4, 4)));
    }

    #[test]
    fn a_cut_past_the_edge_stays_inside_the_photo() {
        let picture = decode_now(&jpeg(1), 2048).unwrap();
        let png = cut_png(&picture, Rect::new(35.0, 15.0, 20.0, 20.0), 4).unwrap();
        let cut = image::load_from_memory(&png).unwrap().into_rgba8();
        assert_eq!(cut.dimensions(), (4, 4));
        assert!(!reddish(*cut.get_pixel(2, 2)));
    }
}
