//! The pixel size of a png, jpeg, gif or webp from its first bytes, for a
//! browser build whose wasm has no decoder for some of them. The browser
//! decodes the pixels, see `browser_decode`.

fn u16_be(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from(u16::from_be_bytes([
        *bytes.get(at)?,
        *bytes.get(at + 1)?,
    ])))
}

fn u16_le(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from(u16::from_le_bytes([
        *bytes.get(at)?,
        *bytes.get(at + 1)?,
    ])))
}

fn u24_le(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes([
        *bytes.get(at)?,
        *bytes.get(at + 1)?,
        *bytes.get(at + 2)?,
        0,
    ]))
}

fn u32_be(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

pub(crate) fn is_jpeg(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xFF, 0xD8, 0xFF])
}

/// Width and height, None for another format or a cut file.
pub(crate) fn dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some((u32_be(bytes, 16)?, u32_be(bytes, 20)?));
    }
    if bytes.starts_with(b"GIF8") {
        return Some((u16_le(bytes, 6)?, u16_le(bytes, 8)?));
    }
    if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        return webp(bytes);
    }
    if is_jpeg(bytes) {
        return jpeg(bytes);
    }
    None
}

fn webp(bytes: &[u8]) -> Option<(u32, u32)> {
    match bytes.get(12..16)? {
        b"VP8 " => Some((u16_le(bytes, 26)? & 0x3FFF, u16_le(bytes, 28)? & 0x3FFF)),
        b"VP8L" => {
            let bits = u32::from_le_bytes(bytes.get(21..25)?.try_into().ok()?);
            Some(((bits & 0x3FFF) + 1, ((bits >> 14) & 0x3FFF) + 1))
        }
        b"VP8X" => Some((u24_le(bytes, 24)? + 1, u24_le(bytes, 27)? + 1)),
        _ => None,
    }
}

/// The size from the first start of frame marker, the segments before it
/// are skipped by their lengths.
fn jpeg(bytes: &[u8]) -> Option<(u32, u32)> {
    let mut at = 2;
    loop {
        while *bytes.get(at)? == 0xFF && *bytes.get(at + 1)? == 0xFF {
            at += 1;
        }
        if *bytes.get(at)? != 0xFF {
            return None;
        }
        let marker = *bytes.get(at + 1)?;
        // The start of frame markers, without DHT, JPG and DAC.
        if matches!(marker, 0xC0..=0xCF) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
            return Some((u16_be(bytes, at + 7)?, u16_be(bytes, at + 5)?));
        }
        at += 2 + usize::try_from(u16_be(bytes, at + 2)?).ok()?;
    }
}

#[cfg(test)]
mod tests {
    use image::ImageReader;

    use super::*;

    fn from_image_crate(bytes: &[u8]) -> (u32, u32) {
        ImageReader::new(std::io::Cursor::new(bytes))
            .with_guessed_format()
            .unwrap()
            .into_dimensions()
            .unwrap()
    }

    #[test]
    fn the_sizes_match_the_image_crate() {
        for name in ["bricks.jpg", "unused/full_hd.jpg", "cat.png", "colors.png"] {
            let bytes = std::fs::read(format!("../assets/images/{name}")).unwrap();
            assert_eq!(dimensions(&bytes), Some(from_image_crate(&bytes)), "{name}");
        }
    }

    #[test]
    fn made_files_of_every_format() {
        let picture = image::RgbaImage::from_pixel(37, 91, image::Rgba([10, 20, 30, 255]));
        for format in [
            image::ImageFormat::Png,
            image::ImageFormat::Jpeg,
            image::ImageFormat::Gif,
            image::ImageFormat::WebP,
        ] {
            let mut bytes = std::io::Cursor::new(Vec::new());
            let encoded = if format == image::ImageFormat::Jpeg {
                image::DynamicImage::ImageRgba8(picture.clone())
                    .to_rgb8()
                    .write_to(&mut bytes, format)
            } else {
                picture.write_to(&mut bytes, format)
            };
            encoded.unwrap();
            assert_eq!(dimensions(bytes.get_ref()), Some((37, 91)), "{format:?}");
        }
    }

    #[test]
    fn other_bytes_have_no_size() {
        assert_eq!(dimensions(b"<svg></svg>"), None);
        assert_eq!(dimensions(&[0xFF, 0xD8, 0xFF]), None);
    }
}
