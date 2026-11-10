#[cfg(any(feature = "gpu", feature = "net"))]
use std::sync::atomic::AtomicU64;
#[cfg(feature = "gpu")]
use std::sync::atomic::Ordering;
use std::{convert::Infallible, path::Path};

use anyhow::Result;
use log::error;

use crate::{
    deps::{
        hreads::from_main,
        refs::{
            Weak,
            manage::{DataManager, ResourceLoader},
        },
    },
    filesystem::read_bytes as read,
    gm::flat::Size,
    image::{Svg, TextureLevels, TextureRawData},
    managed,
};

#[cfg(feature = "gpu")]
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// The id of a new picture or svg. The drawer keeps the GPU side of it by
/// this id, see `image/gpu.rs`.
#[cfg(feature = "gpu")]
pub(crate) fn next_gpu_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug)]
pub struct Image {
    pub size:            Size<u32>,
    pub channels:        u8,
    /// The drawer keeps the GPU texture of the image by this id, see
    /// `image/gpu.rs`.
    #[cfg(feature = "gpu")]
    pub(crate) id:       u64,
    filter:              ImageFilter,
    /// Present for an svg, so an `ImageView` can rasterize it at the
    /// exact size it draws. The texture then holds the old fixed raster
    /// that sprites and levels still draw.
    pub svg:             Option<Svg>,
    /// The render frame an `ImageView` last drew it on, 0 for never. The
    /// memory bound of downloaded images frees the ones drawn longest ago.
    #[cfg(any(feature = "gpu", feature = "net"))]
    pub(crate) drawn_at: AtomicU64,
}

impl Image {
    /// What describes a picture, with a new id and no GPU entry yet.
    pub(crate) fn described(size: Size<u32>, channels: u8, svg: Option<Svg>) -> Self {
        Self {
            size,
            channels,
            #[cfg(feature = "gpu")]
            id: next_gpu_id(),
            filter: ImageFilter::Linear,
            svg,
            #[cfg(any(feature = "gpu", feature = "net"))]
            drawn_at: AtomicU64::new(0),
        }
    }

    /// The picture of `levels`. With the `gpu` feature its texture goes to
    /// the GPU now, on the main thread. Without it the image keeps only
    /// its size, a build with no window draws nothing.
    fn with_levels(levels: TextureLevels, name: &str, svg: Option<Svg>) -> Self {
        #[cfg(feature = "gpu")]
        return Self::from_texture(&crate::image::Texture::from_levels(levels, name), svg);
        #[cfg(not(feature = "gpu"))]
        {
            log::trace!("{name} keeps only its size, this build has no GPU");
            Self::described(levels.size, levels.channels, svg)
        }
    }

    fn load_to_wgpu(name: &str, data: &[u8]) -> Result<Self> {
        #[cfg(wasm)]
        if Svg::is_svg(data) {
            return Ok(Self::from_svg(Svg::parse(data)?));
        }
        #[cfg(wasm)]
        if crate::image::image_header::is_jpeg(data) {
            return Self::from_browser(name, data);
        }
        let levels = TextureRawData::from_file_bytes(data)?.into_levels();
        let svg = Svg::is_svg(data).then(|| Svg::parse(data)).transpose()?;
        Ok(Self::with_levels(levels, name, svg))
    }

    /// Decodes a downloaded file and stores it under `name`, its longest
    /// side at most `max_side`, 0 for its own size. A browser decodes it
    /// itself, see `browser_decode`. An svg keeps the usual path.
    #[cfg(all(wasm, feature = "net"))]
    pub(crate) async fn decode_download(name: &str, data: Vec<u8>, max_side: u32) -> Result<Weak<Image>> {
        if Svg::is_svg(&data) {
            // `load` stops the app on an svg that does not parse, and this
            // one came from the network.
            Svg::parse(&data)?;
            return Ok(Self::load(&data, name));
        }
        match crate::image::browser_decode::decode(&data, max_side).await {
            Ok(raw) => Ok(Self::store_raw(name, raw)),
            Err(err) => {
                log::warn!("The browser did not decode {name}, the engine does: {err}");
                Self::decode_here(name, &data, max_side)
            }
        }
    }

    /// `decode_download` with the decoder of the engine, the only one there is
    /// outside a browser.
    #[cfg(all(not_wasm, feature = "net"))]
    pub(crate) fn decode_download(name: &str, data: Vec<u8>, max_side: u32) -> Result<Weak<Image>> {
        if Svg::is_svg(&data) {
            // `load` stops the app on an svg that does not parse, and this
            // one came from the network.
            Svg::parse(&data)?;
            return Ok(Self::load(&data, name));
        }
        Self::decode_here(name, &data, max_side)
    }

    #[cfg(feature = "net")]
    fn decode_here(name: &str, data: &[u8], max_side: u32) -> Result<Weak<Image>> {
        let raw = TextureRawData::from_file_bytes(data)?.fit_in(max_side)?;
        Ok(Self::store_raw(name, raw))
    }

    /// The mip chain is built on the calling thread, the texture on main.
    #[cfg(feature = "net")]
    fn store_raw(name: &str, raw: TextureRawData) -> Weak<Image> {
        let levels = raw.into_levels();
        let name = name.to_string();
        let stored = from_main(move || {
            Self::store_with_name(&name, || {
                Ok::<_, Infallible>(Self::with_levels(levels, &name, None))
            })
        });
        match stored {
            Ok(image) => image,
            Err(never) => match never {},
        }
    }

    /// How the image samples when drawn at another size than its bitmap,
    /// for every view and sprite that draws it. Set it once after the
    /// load, `Nearest` keeps pixel art sharp.
    pub fn set_filter(&mut self, filter: ImageFilter) {
        if self.filter == filter {
            return;
        }
        self.filter = filter;
        #[cfg(feature = "gpu")]
        self.gpu_filter(filter);
        #[cfg(all(wasm, feature = "gpu"))]
        if let Some(svg) = &mut self.svg {
            svg.set_filter(filter);
        }
    }

    pub fn filter(&self) -> ImageFilter {
        self.filter
    }

    pub fn from_raw_data(
        data: Vec<u8>,
        name: impl Into<String>,
        size: Size<u32>,
        channels: u8,
    ) -> Weak<Image> {
        let name = name.into();
        let image = Self::with_levels(TextureRawData { data, size, channels }.into_levels(), &name, None);
        Image::store_with_name::<Infallible>(&name, || Ok(image)).unwrap()
    }

    /// An image of RGBA `data` for a material cut out at `cutout`, see
    /// `Material::cutout`. Its smaller mip levels keep the share of texels
    /// over the threshold, a plain box filter thins far blades of grass
    /// or leaves to nothing, see `hilen_pixels::keep_coverage`.
    #[cfg(feature = "scene")]
    pub fn from_cutout_data(data: Vec<u8>, name: &str, size: Size<u32>, cutout: f32) -> Weak<Image> {
        let threshold = crate::gm::LossyConvert::lossy_convert((cutout.clamp(0.0, 1.0) * 255.0).round());
        let raw = TextureRawData {
            data,
            size,
            channels: 4,
        };
        Self::from_levels(raw.into_cutout_levels(threshold), name)
    }

    /// An image of `levels`, every mip level given, stored under `name` in
    /// place of an image of that name.
    #[cfg(feature = "scene")]
    pub(crate) fn from_levels(levels: TextureLevels, name: &str) -> Weak<Image> {
        Image::free_with_name(name);
        let image = Self::with_levels(levels, name, None);
        Image::store_with_name::<Infallible>(name, || Ok(image)).unwrap()
    }

    pub(crate) fn from_file_data(data: &[u8], name: &str) -> Weak<Image> {
        Image::store_with_name(name, || Self::load_to_wgpu(name, data))
            .expect("Failed to load image from data")
    }

    pub fn is_monochrome(&self) -> bool {
        self.channels == 1
    }
}

#[cfg(feature = "gpu")]
impl Drop for Image {
    fn drop(&mut self) {
        crate::image::ImageGpu::forget(self.id);
    }
}

managed!(Image);

pub(crate) static DEFAULT_IMAGE_DATA: &[u8] = include_bytes!("delete.png");

impl ResourceLoader for Image {
    fn load_path(path: &Path) -> Self {
        let data = read(path);

        let data = data
            .as_ref()
            .map(Vec::as_slice)
            .inspect_err(|err| {
                error!(
                    "Failed to read image file: {}. Error: {err} Returning default image",
                    path.display()
                );
            })
            .unwrap_or(DEFAULT_IMAGE_DATA);

        Self::load_data(data, path.display())
    }

    fn load_data(data: &[u8], name: impl ToString) -> Self {
        let name = name.to_string();

        #[cfg(wasm)]
        crate::image::svg_sources::store(&name, data);

        #[cfg(wasm)]
        if Svg::is_svg(data) {
            let svg = Svg::parse(data).unwrap_or_else(|err| panic!("Failed to parse svg {name}. Err: {err}"));
            return from_main(move || Image::from_svg(svg));
        }
        #[cfg(wasm)]
        if crate::image::image_header::is_jpeg(data) {
            let data = data.to_vec();
            return from_main(move || {
                Image::from_browser(&name, &data)
                    .unwrap_or_else(|err| panic!("Failed to load image {name}. Err: {err}"))
            });
        }

        let decode_started = web_time::Instant::now();

        let raw_data = TextureRawData::from_file_bytes(data)
            .unwrap_or_else(|err| panic!("Failed to load image {name} to wgpu. Err: {err}"));
        let svg = Svg::is_svg(data)
            .then(|| Svg::parse(data))
            .transpose()
            .unwrap_or_else(|err| panic!("Failed to parse svg {name}. Err: {err}"));

        // On the calling thread. A download decodes on a worker, and the
        // box filter over every pixel must not run on the main thread.
        let levels = raw_data.into_levels();

        let decode_time = decode_started.elapsed();
        if decode_time.as_millis() > 100 {
            log::debug!("Decoding image {name} took {} ms", decode_time.as_millis());
        }

        from_main(move || Image::with_levels(levels, &name, svg))
    }

    #[cfg(wasm)]
    fn pending_data(name: &str) -> Option<Vec<u8>> {
        crate::image::pending::get(name)
    }

    #[cfg(wasm)]
    fn pending_loaded(name: &str) {
        crate::image::pending::forget(name);
    }
}

/// An svg in a browser with no `gpu` feature: the size of its fixed
/// raster, no picture.
#[cfg(all(wasm, not(feature = "gpu")))]
impl Image {
    fn from_svg(svg: Svg) -> Self {
        Self::described(svg.fixed_size(), 4, Some(svg))
    }

    fn from_browser(name: &str, data: &[u8]) -> Result<Self> {
        let (width, height) = crate::image::image_header::dimensions(data)
            .ok_or_else(|| anyhow::anyhow!("No size in the header of {name}"))?;
        Ok(Self::described(Size::new(width, height), 4, None))
    }
}

/// How an image samples between its texels, see `Image::set_filter`.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub enum ImageFilter {
    #[default]
    Linear,
    Nearest,
}

#[cfg(all(test, not_wasm, feature = "net"))]
mod test {
    use super::Image;

    /// A file from the network that starts like an svg and does not parse
    /// stopped the whole app in `load`.
    #[test]
    fn a_downloaded_svg_that_does_not_parse_is_an_error() {
        let broken = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><rect".to_vec();
        assert!(Image::decode_download("broken.svg", broken, 0).is_err());
    }
}
