use std::{convert::Infallible, path::Path};

use anyhow::Result;
use log::error;
use wgpu::{
    BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, Sampler, SamplerBindingType, ShaderStages, TextureSampleType, TextureView,
    TextureViewDimension,
};

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
    pub size:     Size<u32>,
    pub channels: u8,
    bind:         ImageBind,
    /// Present for an svg, so an `ImageView` can rasterize it at the
    /// exact size it draws. `bind` then holds the old fixed raster that
    /// sprites and levels still draw.
    pub svg:      Option<Svg>,
}

impl Image {
    fn load_to_wgpu(name: &str, data: &[u8]) -> Result<Self> {
        let texture = Texture::from_file_bytes(data, name)?;
        let svg = Svg::is_svg(data).then(|| Svg::parse(data)).transpose()?;
        Ok(Self::from_texture(&texture, svg))
    }

    pub(crate) fn bind_texture(texture: &Texture) -> ImageBind {
        let bind = Window::device().create_bind_group(&wgpu::BindGroupDescriptor {
            label:   "image_bind_group".into(),
            layout:  Self::uniform_layout(),
            entries: &[
                BindGroupEntry {
                    binding:  0,
                    resource: BindingResource::TextureView(&texture.view),
                },
                BindGroupEntry {
                    binding:  1,
                    resource: BindingResource::Sampler(&texture.sampler),
                },
            ],
        });
        ImageBind::new(bind, texture.view.clone(), texture.sampler.clone())
    }

    fn from_texture(texture: &Texture, svg: Option<Svg>) -> Self {
        Self {
            size: texture.size,
            channels: texture.channels,
            bind: Self::bind_texture(texture),
            svg,
        }
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

    /// A blank image whose texture is also a render target, for a video frame
    /// converted on the GPU. Keyed like any managed image, so the same key
    /// gives the same image back.
    #[cfg(feature = "video")]
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
        self.bind.get()
    }

    pub(crate) fn view(&self) -> &TextureView {
        self.bind.view()
    }

    pub(crate) fn sampler(&self) -> &Sampler {
        self.bind.sampler()
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

        let decode_started = web_time::Instant::now();

        let raw_data = Texture::parse_file_from_bytes(data)
            .unwrap_or_else(|err| panic!("Failed to load image {name} to wgpu. Err: {err}"));
        let svg = Svg::is_svg(data)
            .then(|| Svg::parse(data))
            .transpose()
            .unwrap_or_else(|err| panic!("Failed to parse svg {name}. Err: {err}"));

        let decode_time = decode_started.elapsed();
        if decode_time.as_millis() > 100 {
            log::debug!("Decoding image {name} took {} ms", decode_time.as_millis());
        }

        from_main(move || Image::from_texture(&Texture::from_raw_data(raw_data, &name), svg))
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
