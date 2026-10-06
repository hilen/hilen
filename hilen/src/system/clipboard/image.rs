//! A picture of the clipboard, kept as the bytes of a png file.

#[cfg(ios)]
use std::io::Cursor;

use anyhow::{Result, ensure};
#[cfg(ios)]
use image::ImageFormat;
#[cfg(ios)]
use image::ImageReader;
use image::{
    ExtendedColorType, ImageEncoder,
    codecs::png::{CompressionType, FilterType, PngEncoder},
};
#[cfg(desktop)]
use image::{ImageFormat, load_from_memory_with_format};

/// A picture from the clipboard: the bytes of a png file and its size in
/// pixels. The bytes go as they are into a file, an upload or
/// `Image::load`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardImage {
    pub png:    Vec<u8>,
    pub width:  u32,
    pub height: u32,
}

/// What a desktop clipboard holds for a picture, the pixels
/// `ClipboardImage::from_rgba` takes.
#[cfg(desktop)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Pixels {
    pub(super) rgba:   Vec<u8>,
    pub(super) width:  u32,
    pub(super) height: u32,
}

impl ClipboardImage {
    /// Makes the png file of RGBA pixels, 4 bytes per pixel, rows from
    /// the top, alpha not multiplied into the colors.
    ///
    /// The default compression with the adaptive filter. For a picture of
    /// 3000 by 2000 pixels it takes 50 ms when the picture is a window
    /// and 500 ms when it is noise, like a photo. The fast compression
    /// takes half of that for a window and makes a file 80 times bigger.
    pub fn from_rgba(rgba: &[u8], width: u32, height: u32) -> Result<Self> {
        ensure!(
            width > 0 && height > 0,
            "A picture of {width} by {height} pixels has no pixels"
        );
        // The encoder panics on a buffer of another length.
        ensure!(
            u64::try_from(rgba.len())? == u64::from(width) * u64::from(height) * 4,
            "{} bytes are not {width} by {height} pixels of RGBA",
            rgba.len()
        );

        let mut png = Vec::new();
        PngEncoder::new_with_quality(&mut png, CompressionType::Default, FilterType::Adaptive).write_image(
            rgba,
            width,
            height,
            ExtendedColorType::Rgba8,
        )?;

        Ok(Self { png, width, height })
    }
}

#[cfg(desktop)]
impl ClipboardImage {
    pub(super) fn encode(pixels: &Pixels) -> Result<Self> {
        Self::from_rgba(&pixels.rgba, pixels.width, pixels.height)
    }

    pub(super) fn decode(&self) -> Result<Pixels> {
        let picture = load_from_memory_with_format(&self.png, ImageFormat::Png)?.into_rgba8();
        let (width, height) = picture.dimensions();
        Ok(Pixels {
            rgba: picture.into_raw(),
            width,
            height,
        })
    }
}

#[cfg(ios)]
impl ClipboardImage {
    /// A png file the system made, its size is read from its header.
    pub(super) fn from_png(png: Vec<u8>) -> Result<Self> {
        let (width, height) =
            ImageReader::with_format(Cursor::new(&png), ImageFormat::Png).into_dimensions()?;
        Ok(Self { png, width, height })
    }
}

#[cfg(all(test, desktop))]
mod tests {
    use std::time::Instant;

    use anyhow::Result;

    use super::{ClipboardImage, Pixels};

    /// 4 pixels with 4 different colors, one of them half transparent.
    const RGBA: [u8; 16] = [255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 10, 20, 30, 128];

    fn pixels(rgba: &[u8], width: u32, height: u32) -> Pixels {
        Pixels {
            rgba: rgba.to_vec(),
            width,
            height,
        }
    }

    #[test]
    fn the_bytes_are_a_png_file_with_the_same_pixels() -> Result<()> {
        let image = ClipboardImage::encode(&pixels(&RGBA, 2, 2))?;

        assert_eq!(&image.png[..8], b"\x89PNG\r\n\x1a\n");
        // The size in the header of the file, big endian, after the 8
        // bytes of the signature and the 8 of the chunk head.
        assert_eq!(&image.png[16..24], &[0, 0, 0, 2, 0, 0, 0, 2]);
        assert_eq!((image.width, image.height), (2, 2));

        assert_eq!(image.decode()?, pixels(&RGBA, 2, 2));
        Ok(())
    }

    #[test]
    fn a_picture_that_is_not_square_keeps_its_sides() -> Result<()> {
        let image = ClipboardImage::encode(&pixels(&RGBA, 4, 1))?;

        assert_eq!((image.width, image.height), (4, 1));
        assert_eq!(image.decode()?, pixels(&RGBA, 4, 1));
        Ok(())
    }

    #[test]
    fn pixels_that_do_not_fill_the_size_are_refused() {
        assert!(ClipboardImage::encode(&pixels(&RGBA, 3, 2)).is_err());
        assert!(ClipboardImage::encode(&pixels(&[], 0, 0)).is_err());
    }

    #[test]
    fn bytes_that_are_no_png_file_are_refused() {
        let image = ClipboardImage {
            png:    b"not a picture".to_vec(),
            width:  2,
            height: 2,
        };

        assert!(image.decode().is_err());
    }

    /// A picture like a window: flat panels, a gradient, and rows of
    /// small dark marks where text would be.
    fn window(width: u32, height: u32) -> Vec<u8> {
        let mut rgba = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let text = y % 24 < 14 && (x * 7 + y * 13) % 5 < 2 && x % 400 > 40;
                let pixel = if text {
                    [30, 30, 34]
                } else if x < width / 5 {
                    [236, 236, 240]
                } else if y < 80 {
                    [u8::try_from(x * 255 / width).unwrap_or(u8::MAX), 120, 200]
                } else {
                    [255, 255, 255]
                };
                rgba.extend_from_slice(&pixel);
                rgba.push(255);
            }
        }
        rgba
    }

    /// Not a check, the numbers behind the choice to encode off the main
    /// thread: `cargo test -p hilen --lib encoding_time -- --ignored
    /// --nocapture`, with `--release` for the speed of a shipped app.
    #[test]
    #[ignore = "a measurement, run by hand"]
    fn encoding_time_of_a_big_picture() -> Result<()> {
        let (width, height) = (3000, 2000);
        let noise: Vec<u8> = (0..width * height * 4).map(|_| fastrand::u8(..)).collect();

        for (name, rgba) in [("window", window(width, height)), ("noise", noise)] {
            let picture = pixels(&rgba, width, height);
            let started = Instant::now();
            let image = ClipboardImage::encode(&picture)?;
            println!(
                "{name}: {} ms, {} KB",
                started.elapsed().as_millis(),
                image.png.len() / 1024
            );
        }
        Ok(())
    }
}
