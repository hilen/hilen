#[cfg(any(test, feature = "ui-tests", target_os = "windows", target_os = "linux"))]
use anyhow::Result;
#[cfg(any(test, feature = "ui-tests", target_os = "windows", target_os = "linux"))]
use image::{ExtendedColorType, ImageEncoder, codecs::jpeg::JpegEncoder};

/// The file format of a photo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhotoFormat {
    Jpeg,
    /// HEVC in a HEIF file, about half the size of a JPEG of the same look.
    /// Apple cameras have it.
    Heic,
}

impl PhotoFormat {
    /// The format of the bytes of a file, by their first bytes.
    pub fn of(bytes: &[u8]) -> Option<Self> {
        if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return Some(Self::Jpeg);
        }
        let brand = bytes.get(8..12)?;
        let heif = bytes.get(4..8) == Some(b"ftyp".as_slice())
            && [b"heic", b"heix", b"heim", b"heis", b"hevc", b"mif1"]
                .iter()
                .any(|known| brand == known.as_slice());
        heif.then_some(Self::Heic)
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Heic => "heic",
        }
    }

    pub fn mime_type(self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Heic => "image/heic",
        }
    }
}

/// A photo as the camera encoded it, ready to save as a file.
#[derive(Debug, Clone)]
pub struct Photo {
    pub bytes:  Vec<u8>,
    pub format: PhotoFormat,
    pub width:  u32,
    pub height: u32,
}

/// JPEG at the quality a camera app saves with.
#[cfg(any(test, feature = "ui-tests", target_os = "windows", target_os = "linux"))]
pub(super) fn encode_jpeg(pixels: &[u8], width: u32, height: u32, color: ExtendedColorType) -> Result<Photo> {
    const QUALITY: u8 = 95;

    let mut bytes = Vec::new();
    JpegEncoder::new_with_quality(&mut bytes, QUALITY).write_image(pixels, width, height, color)?;
    Ok(Photo {
        bytes,
        format: PhotoFormat::Jpeg,
        width,
        height,
    })
}
