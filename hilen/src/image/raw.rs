//! The pixels of a picture before the GPU: a decoded file and its mip
//! levels. Nothing here needs a device, so a worker thread does this part
//! and a build with no `gpu` feature has it too.

use anyhow::Result;
#[cfg(feature = "net")]
use anyhow::anyhow;
use image::GenericImageView;
#[cfg(feature = "net")]
use image::{ImageBuffer, Rgba};

use crate::{gm::flat::Size, image::Svg};

pub struct TextureRawData {
    pub data:     Vec<u8>,
    pub size:     Size<u32>,
    pub channels: u8,
}

/// The pixels of a texture with every mip level built. Building them needs
/// no device, so a load on a worker thread does it there and the main
/// thread is left with the upload alone.
pub(crate) struct TextureLevels {
    #[cfg(feature = "gpu")]
    pub(crate) levels:   Vec<(Vec<u8>, Size<u32>)>,
    pub(crate) size:     Size<u32>,
    /// 1 or 4, a picture of 3 channels gets a fourth.
    pub(crate) channels: u8,
}

/// The size `size` shrinks to so its longest side is `max_side`, with the
/// same shape. None when it already fits or `max_side` is 0, no limit.
#[cfg(any(feature = "net", feature = "camera"))]
pub(crate) fn fitted_size(size: Size<u32>, max_side: u32) -> Option<Size<u32>> {
    let longest = size.width.max(size.height);
    if max_side == 0 || longest <= max_side {
        return None;
    }
    let scale = |side: u32| {
        let scaled = u64::from(side) * u64::from(max_side) / u64::from(longest);
        u32::try_from(scaled).unwrap_or(max_side).max(1)
    };
    Some(Size::new(scale(size.width), scale(size.height)))
}

impl TextureRawData {
    pub(crate) fn from_file_bytes(bytes: &[u8]) -> Result<Self> {
        #[cfg(not_wasm)]
        if Svg::is_svg(bytes) {
            return Svg::fixed_raster(bytes);
        }
        #[cfg(wasm)]
        if Svg::is_svg(bytes) {
            anyhow::bail!("A browser draws an svg itself, see svg_browser.rs");
        }

        Self::from_dynamic_image(bytes)
    }

    fn from_dynamic_image(bytes: &[u8]) -> Result<Self> {
        let image = image::load_from_memory(bytes)?;

        let dimensions = image.dimensions();

        Ok(Self {
            data:     image.to_rgba8().to_vec(),
            size:     (dimensions.0, dimensions.1).into(),
            channels: image.color().channel_count(),
        })
    }

    /// The same picture with its longest side at most `max_side`. The data
    /// of a decoded file is always 4 bytes a pixel, whatever `channels` says.
    #[cfg(feature = "net")]
    pub(crate) fn fit_in(self, max_side: u32) -> Result<Self> {
        let Some(size) = fitted_size(self.size, max_side) else {
            return Ok(self);
        };
        let channels = self.channels;
        let buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(self.size.width, self.size.height, self.data)
            .ok_or_else(|| anyhow!("Pixel data does not match the size {:?}", self.size))?;
        let small = image::imageops::thumbnail(&buffer, size.width, size.height);
        Ok(Self {
            data: small.into_raw(),
            size,
            channels,
        })
    }

    /// The levels of the picture. A build with no `gpu` feature keeps only
    /// the size, nothing uploads the pixels.
    pub(crate) fn into_levels(self) -> TextureLevels {
        let Self { data, size, channels } = self;

        let channels = match channels {
            1 => 1,
            3 | 4 => 4,
            ch => panic!("Invalid number of channels: {ch}"),
        };

        // The whole mip chain, so an image drawn smaller than its bitmap
        // is box filtered instead of skipping texels. Svgs rasterize at
        // eight times their size and icons draw at a fraction of that.
        #[cfg(feature = "gpu")]
        let levels = hilen_pixels::mip_chain(data, size.width, size.height, channels)
            .into_iter()
            .map(|(pixels, (width, height))| (pixels, Size::new(width, height)))
            .collect();

        #[cfg(not(feature = "gpu"))]
        drop(data);
        TextureLevels {
            #[cfg(feature = "gpu")]
            levels,
            size,
            channels,
        }
    }
}

#[cfg(feature = "scene")]
impl TextureRawData {
    /// The levels of `into_levels` with the alpha of every smaller level
    /// raised so a cutout at `threshold` covers as much as at full size,
    /// see `hilen_pixels::keep_coverage`.
    pub(crate) fn into_cutout_levels(self, threshold: u8) -> TextureLevels {
        let mut levels = self.into_levels();
        if levels.channels == 4 {
            hilen_pixels::keep_coverage(&mut levels.levels, threshold);
        }
        levels
    }
}

#[cfg(all(test, feature = "net", feature = "gpu"))]
mod test {
    use super::{TextureRawData, fitted_size};
    use crate::gm::flat::Size;

    #[test]
    fn a_big_picture_fits_its_longest_side() {
        assert_eq!(fitted_size(Size::new(1000, 1500), 600), Some(Size::new(400, 600)));
        assert_eq!(fitted_size(Size::new(3000, 10), 300), Some(Size::new(300, 1)));
        assert_eq!(fitted_size(Size::new(400, 600), 600), None);
        assert_eq!(fitted_size(Size::new(4000, 6000), 0), None);
    }

    #[test]
    fn fit_in_shrinks_the_pixels() {
        let raw = TextureRawData {
            data:     vec![90; 20 * 10 * 4],
            size:     Size::new(20, 10),
            channels: 3,
        };

        let small = raw.fit_in(5).expect("fits");

        assert_eq!(small.size, Size::new(5, 2));
        assert_eq!(small.data.len(), 5 * 2 * 4);
        assert_eq!(small.channels, 3);
        assert!(small.data.iter().all(|byte| *byte == 90));
    }

    /// A unit test has no device and no main thread, so every level this
    /// returns was built in the step a worker thread runs. A mip chain
    /// moved back behind the device would leave nothing to assert here.
    #[test]
    fn mip_levels_are_built_without_a_device() {
        let raw = TextureRawData {
            data:     vec![200; 8 * 4 * 4],
            size:     Size::new(8, 4),
            channels: 4,
        };
        let given = raw.data.as_ptr();

        let levels = raw.into_levels();

        let sizes: Vec<_> = levels.levels.iter().map(|(_, size)| (size.width, size.height)).collect();
        assert_eq!(sizes, [(8, 4), (4, 2), (2, 1), (1, 1)]);
        assert_eq!(levels.levels[0].0.as_ptr(), given, "level 0 was copied");
        assert_eq!(levels.levels[3].0, [200; 4]);
    }
}
