use std::{convert::Infallible, path::Path, sync::atomic::AtomicU64};

use anyhow::Result;
use log::error;
use wgpu::{
    BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, Sampler, SamplerBindingType, ShaderStages, TextureSampleType, TextureView,
    TextureViewDimension,
};

#[cfg(feature = "scene")]
use crate::window::image::TextureLevels;
use crate::{
    deps::{
        hreads::from_main,
        refs::{
            Weak,
            main_lock::MainLock,
            manage::{DataManager, ResourceLoader},
        },
    },
    filesystem::read_bytes as read,
    gm::flat::Size,
    managed,
    window::{
        Window,
        image::{ImageBind, Svg, Texture, TextureRawData},
    },
};

#[derive(Debug)]
pub struct Image {
    pub size:            Size<u32>,
    pub channels:        u8,
    bind:                ImageBind,
    filter:              ImageFilter,
    /// Present for an svg, so an `ImageView` can rasterize it at the
    /// exact size it draws. `bind` then holds the old fixed raster that
    /// sprites and levels still draw.
    pub svg:             Option<Svg>,
    /// The render frame an `ImageView` last drew it on, 0 for never. The
    /// memory bound of downloaded images frees the ones drawn longest ago.
    pub(crate) drawn_at: AtomicU64,
    /// A bundled jpeg in a browser, whose pixels the page decodes later.
    /// `bind` is transparent until then.
    #[cfg(wasm)]
    late:                Option<crate::window::image::late::Late>,
}

impl Image {
    fn load_to_wgpu(name: &str, data: &[u8]) -> Result<Self> {
        #[cfg(wasm)]
        if Svg::is_svg(data) {
            return Ok(Self::from_svg(Svg::parse(data)?));
        }
        #[cfg(wasm)]
        if crate::window::image::image_header::is_jpeg(data) {
            return Self::from_browser(name, data);
        }
        let texture = Texture::from_file_bytes(data, name)?;
        let svg = Svg::is_svg(data).then(|| Svg::parse(data)).transpose()?;
        Ok(Self::from_texture(&texture, svg))
    }

    /// Decodes a downloaded file and stores it under `name`, its longest
    /// side at most `max_side`, 0 for its own size. A browser decodes it
    /// itself, see `browser_decode`. An svg keeps the usual path.
    #[cfg(wasm)]
    pub(crate) async fn decode_download(name: &str, data: Vec<u8>, max_side: u32) -> Result<Weak<Image>> {
        if Svg::is_svg(&data) {
            return Ok(Self::load(&data, name));
        }
        match crate::window::image::browser_decode::decode(&data, max_side).await {
            Ok(raw) => Ok(Self::store_raw(name, raw)),
            Err(err) => {
                log::warn!("The browser did not decode {name}, the engine does: {err}");
                Self::decode_here(name, &data, max_side)
            }
        }
    }

    /// `decode_download` with the decoder of the engine, the only one there is
    /// outside a browser.
    #[cfg(not_wasm)]
    pub(crate) fn decode_download(name: &str, data: Vec<u8>, max_side: u32) -> Result<Weak<Image>> {
        if Svg::is_svg(&data) {
            return Ok(Self::load(&data, name));
        }
        Self::decode_here(name, &data, max_side)
    }

    fn decode_here(name: &str, data: &[u8], max_side: u32) -> Result<Weak<Image>> {
        let raw = Texture::parse_file_from_bytes(data)?.fit_in(max_side)?;
        Ok(Self::store_raw(name, raw))
    }

    /// The mip chain is built on the calling thread, the texture on main.
    fn store_raw(name: &str, raw: TextureRawData) -> Weak<Image> {
        let levels = raw.into_levels();
        let name = name.to_string();
        let stored = from_main(move || {
            Self::store_with_name(&name, || {
                Ok::<_, Infallible>(Self::from_texture(&Texture::from_levels(levels, &name), None))
            })
        });
        match stored {
            Ok(image) => image,
            Err(never) => match never {},
        }
    }

    pub(crate) fn bind_texture(texture: &Texture) -> ImageBind {
        Self::bind_view(&texture.view, &texture.sampler)
    }

    pub(crate) fn bind_view(view: &TextureView, sampler: &Sampler) -> ImageBind {
        let bind = Window::device().create_bind_group(&wgpu::BindGroupDescriptor {
            label:   "image_bind_group".into(),
            layout:  Self::uniform_layout(),
            entries: &[
                BindGroupEntry {
                    binding:  0,
                    resource: BindingResource::TextureView(view),
                },
                BindGroupEntry {
                    binding:  1,
                    resource: BindingResource::Sampler(sampler),
                },
            ],
        });
        ImageBind {
            bind,
            view: view.clone(),
            #[cfg(feature = "scene")]
            sampler: sampler.clone(),
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
        let view = self.bind.view.clone();
        self.bind = Self::bind_view(&view, &Texture::image_sampler(filter));
        #[cfg(wasm)]
        {
            if let Some(late) = &mut self.late {
                late.set_filter(filter);
            }
            if let Some(svg) = &mut self.svg {
                svg.set_filter(filter);
            }
        }
    }

    pub fn filter(&self) -> ImageFilter {
        self.filter
    }

    /// An svg in a browser, whose rasters the page draws later. Its own
    /// texture is never drawn, `bind` hands out the fixed raster.
    #[cfg(wasm)]
    fn from_svg(svg: Svg) -> Self {
        Self {
            size:     svg.fixed_size(),
            channels: 4,
            bind:     Self::bind_texture(&crate::window::image::late::transparent_texture("svg_image")),
            filter:   ImageFilter::Linear,
            svg:      Some(svg),
            drawn_at: AtomicU64::new(0),
            late:     None,
        }
    }

    /// A bundled jpeg in a browser. The wasm has no jpeg decoder, the page
    /// decodes it, see `browser_decode`. The size comes from the header.
    #[cfg(wasm)]
    fn from_browser(name: &str, data: &[u8]) -> Result<Self> {
        use crate::window::image::{browser_decode, image_header, late};

        let (width, height) = image_header::dimensions(data)
            .ok_or_else(|| anyhow::anyhow!("No size in the header of {name}"))?;
        let late = late::Late::default();
        let bytes = data.to_vec();
        late.start(name.to_string(), async move {
            browser_decode::decode(&bytes, 0).await
        });
        Ok(Self {
            size:     Size::new(width, height),
            channels: 4,
            bind:     Self::bind_texture(&late::transparent_texture(name)),
            filter:   ImageFilter::Linear,
            svg:      None,
            drawn_at: AtomicU64::new(0),
            late:     Some(late),
        })
    }

    /// Pictures of this image drawn on `frame` or later that the browser
    /// still makes, see `late::pictures_pending`.
    #[cfg(wasm)]
    pub(crate) fn pending_since(&self, frame: u64) -> usize {
        let svg = self.svg.as_ref().map_or(0, |svg| svg.pending_since(frame));
        let late = self.late.as_ref().is_some_and(|late| {
            late.is_pending() && self.drawn_at.load(std::sync::atomic::Ordering::Relaxed) >= frame
        });
        svg + usize::from(late)
    }

    /// Whether the pixels are here, false while a browser still makes a
    /// bundled jpeg. The damage of a view changes when they land.
    #[cfg_attr(not_wasm, allow(clippy::unused_self))]
    pub(crate) fn pixels_ready(&self) -> bool {
        #[cfg(wasm)]
        return self.late.as_ref().is_none_or(|late| late.get().is_some());
        #[cfg(not_wasm)]
        true
    }

    fn from_texture(texture: &Texture, svg: Option<Svg>) -> Self {
        Self {
            size: texture.size,
            channels: texture.channels,
            bind: Self::bind_texture(texture),
            filter: ImageFilter::Linear,
            svg,
            drawn_at: AtomicU64::new(0),
            #[cfg(wasm)]
            late: None,
        }
    }

    /// A picture whose pixels change in place, like the live picture of a
    /// camera, RGBA with 1 level. `Texture::write_level_0` on the texture
    /// it gives puts new pixels in. Keyed like any managed image, free it
    /// by its name when it is done. The texture stays out of the image, a
    /// texture of WebGPU may not leave the main thread and an image may.
    #[cfg(feature = "camera")]
    pub(crate) fn stream(name: &str, size: Size<u32>) -> (Weak<Image>, wgpu::Texture) {
        let texture = Texture::stream(size, name);
        let image =
            Image::store_with_name::<Infallible>(name, || Ok(Self::from_texture(&texture, None))).unwrap();
        (image, texture.texture)
    }

    pub fn from_raw_data(
        data: Vec<u8>,
        name: impl Into<String>,
        size: Size<u32>,
        channels: u8,
    ) -> Weak<Image> {
        let name = name.into();
        let texture = Texture::from_raw_data(TextureRawData { data, size, channels }, &name);
        let image = Self::from_texture(&texture, None);
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
        let texture = Texture::from_levels(levels, name);
        let image = Self::from_texture(&texture, None);
        Image::store_with_name::<Infallible>(name, || Ok(image)).unwrap()
    }

    /// A blank image whose texture is also a render target, for a video frame
    /// converted on the GPU or a scene drawn into a picture. Keyed like any
    /// managed image, so the same key gives the same image back.
    #[cfg(any(all(feature = "video", ffmpeg), feature = "scene"))]
    pub(crate) fn render_target(name: &str, size: Size<u32>) -> Weak<Image> {
        Image::store_with_name::<Infallible>(name, || {
            Ok(Self::from_texture(&Texture::render_target(size, name), None))
        })
        .unwrap()
    }

    pub(crate) fn from_file_data(data: &[u8], name: &str) -> Weak<Image> {
        Image::store_with_name(name, || Self::load_to_wgpu(name, data))
            .expect("Failed to load image from data")
    }

    pub fn is_monochrome(&self) -> bool {
        self.channels == 1
    }

    pub(crate) fn bind(&self) -> &BindGroup {
        #[cfg(wasm)]
        if let Some(svg) = &self.svg {
            return svg.fixed_bind();
        }
        &self.current_bind().bind
    }

    /// The bind of the pixels, a late one once it landed.
    fn current_bind(&self) -> &ImageBind {
        #[cfg(wasm)]
        if let Some(bind) = self.late.as_ref().and_then(crate::window::image::late::Late::get) {
            return bind;
        }
        &self.bind
    }

    #[cfg(any(feature = "scene", all(feature = "video", ffmpeg)))]
    pub(crate) fn view(&self) -> &TextureView {
        &self.current_bind().view
    }

    #[cfg(feature = "scene")]
    pub(crate) fn sampler(&self) -> &Sampler {
        &self.current_bind().sampler
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
        crate::window::image::svg_sources::store(&name, data);

        #[cfg(wasm)]
        if Svg::is_svg(data) {
            let svg = Svg::parse(data).unwrap_or_else(|err| panic!("Failed to parse svg {name}. Err: {err}"));
            return from_main(move || Image::from_svg(svg));
        }
        #[cfg(wasm)]
        if crate::window::image::image_header::is_jpeg(data) {
            let data = data.to_vec();
            return from_main(move || {
                Image::from_browser(&name, &data)
                    .unwrap_or_else(|err| panic!("Failed to load image {name}. Err: {err}"))
            });
        }

        let decode_started = web_time::Instant::now();

        let raw_data = Texture::parse_file_from_bytes(data)
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

        from_main(move || Image::from_texture(&Texture::from_levels(levels, &name), svg))
    }

    #[cfg(wasm)]
    fn pending_data(name: &str) -> Option<Vec<u8>> {
        crate::window::image::pending::get(name)
    }

    #[cfg(wasm)]
    fn pending_loaded(name: &str) {
        crate::window::image::pending::forget(name);
    }
}

impl Image {
    pub(crate) fn uniform_layout() -> &'static BindGroupLayout {
        static LAYOUT: MainLock<BindGroupLayout> = MainLock::new();
        LAYOUT.get_or_init(|| {
            Window::device().create_bind_group_layout(&BindGroupLayoutDescriptor {
                label:   "image_bind_group_layout".into(),
                entries: &[
                    BindGroupLayoutEntry {
                        binding:    0,
                        visibility: ShaderStages::FRAGMENT,
                        ty:         BindingType::Texture {
                            multisampled:   false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type:    TextureSampleType::Float { filterable: true },
                        },
                        count:      None,
                    },
                    BindGroupLayoutEntry {
                        binding:    1,
                        visibility: ShaderStages::FRAGMENT,
                        ty:         BindingType::Sampler(SamplerBindingType::Filtering),
                        count:      None,
                    },
                ],
            })
        })
    }
}

/// How an image samples between its texels, see `Image::set_filter`.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub enum ImageFilter {
    #[default]
    Linear,
    Nearest,
}
