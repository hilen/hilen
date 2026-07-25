use std::{convert::Infallible, path::Path};

use anyhow::Result;
use hreads::from_main;
use log::error;
use refs::{
    Weak,
    main_lock::MainLock,
    manage::{DataManager, ResourceLoader},
    managed,
};
use wgpu::{
    BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, SamplerBindingType, ShaderStages, TextureSampleType, TextureViewDimension,
};

use crate::{
    filesystem::read_bytes as read,
    gm::flat::Size,
    window::{
        Window,
        image::{ImageBind, Texture, TextureRawData},
    },
};

#[derive(Debug)]
pub struct Image {
    pub size:     Size<u32>,
    pub channels: u8,
    bind:         ImageBind,
}

impl Image {
    fn load_to_wgpu(name: &str, data: &[u8]) -> Result<Self> {
        let texture = Texture::from_file_bytes(data, name)?;
        Ok(Self::from_texture(&texture))
    }

    fn from_texture(texture: &Texture) -> Self {
        let device = Window::device();

        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
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

        Self {
            size:     texture.size,
            channels: texture.channels,
            bind:     bind.into(),
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
        let image = Self::from_texture(&texture);
        Image::store_with_name::<Infallible>(&name, || Ok(image)).unwrap()
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

        let decode_time = decode_started.elapsed();
        if decode_time.as_millis() > 100 {
            log::debug!("Decoding image {name} took {} ms", decode_time.as_millis());
        }

        from_main(move || Image::from_texture(&Texture::from_raw_data(raw_data, &name)))
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
