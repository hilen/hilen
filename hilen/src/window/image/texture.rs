use std::path::Path;

use anyhow::{Result, anyhow};
use image::{GenericImageView, ImageBuffer, Rgba};
use wgpu::{
    AddressMode, Device, Extent3d, FilterMode, MipmapFilterMode, Origin3d, Sampler, SamplerDescriptor,
    TexelCopyBufferLayout, TexelCopyTextureInfo, TextureAspect, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
};

use crate::{
    gm::flat::Size,
    window::{Window, image::Svg},
};

#[derive(Debug)]
pub struct Texture {
    pub texture:  wgpu::Texture,
    pub view:     TextureView,
    pub sampler:  Sampler,
    pub size:     Size<u32>,
    pub channels: u8,
}

pub struct TextureRawData {
    pub data:     Vec<u8>,
    pub size:     Size<u32>,
    pub channels: u8,
}

impl Texture {
    pub(crate) const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth24PlusStencil8;

    pub(crate) fn from_file_bytes(bytes: &[u8], label: &str) -> Result<Self> {
        let data = Self::parse_file_from_bytes(bytes)?;
        Ok(Self::from_raw_data(data, label))
    }

    pub(crate) fn parse_file_from_bytes(bytes: &[u8]) -> Result<TextureRawData> {
        if Svg::is_svg(bytes) {
            return Svg::fixed_raster(bytes);
        }

        Self::parse_dynamic_image(bytes)
    }

    fn parse_dynamic_image(bytes: &[u8]) -> Result<TextureRawData> {
        let image = image::load_from_memory(bytes)?;

        let dimensions = image.dimensions();

        Ok(TextureRawData {
            data:     image.to_rgba8().to_vec(),
            size:     (dimensions.0, dimensions.1).into(),
            channels: image.color().channel_count(),
        })
    }

    pub fn from_raw_data(TextureRawData { data, size, channels }: TextureRawData, label: &str) -> Self {
        let extend_size = Extent3d {
            width:                 size.width,
            height:                size.height,
            depth_or_array_layers: 1,
        };

        // Plain Unorm, the whole pipeline works on encoded sRGB values.
        // Image bytes are already encoded, sampling must return them
        // unchanged. An sRGB format here would decode on sample and the
        // image would render one decode too dark.
        let (channels, format) = match channels {
            1 => (1, TextureFormat::R8Unorm),
            3 | 4 => (4, TextureFormat::Rgba8Unorm),
            ch => panic!("Invalid number of channels: {ch}"),
        };

        let device = Window::device();

        // The whole mip chain, so an image drawn smaller than its bitmap
        // is box filtered instead of skipping texels. Svgs rasterize at
        // eight times their size and icons draw at a fraction of that.
        let levels = mip_chain(&data, size, channels);

        let texture = device.create_texture(&TextureDescriptor {
            label: label.into(),
            size: extend_size,
            mip_level_count: u32::try_from(levels.len()).expect("mip count fits u32"),
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        for (level, (pixels, level_size)) in levels.iter().enumerate() {
            let extent = Extent3d {
                width:                 level_size.width,
                height:                level_size.height,
                depth_or_array_layers: 1,
            };
            Window::queue().write_texture(
                TexelCopyTextureInfo {
                    aspect:    TextureAspect::All,
                    texture:   &texture,
                    mip_level: u32::try_from(level).expect("mip level fits u32"),
                    origin:    Origin3d::ZERO,
                },
                pixels,
                TexelCopyBufferLayout {
                    offset:         0,
                    bytes_per_row:  Some(u32::from(channels) * extent.width),
                    rows_per_image: Some(extent.height),
                },
                extent,
            );
        }

        let view = texture.create_view(&TextureViewDescriptor::default());

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: "texture_sampler".into(),
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Linear,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
            size,
            channels,
        }
    }

    pub(crate) fn create_depth_texture(
        device: &Device,
        size: Size<u32>,
        sample_count: u32,
        label: &str,
    ) -> Self {
        let extend = Extent3d {
            width:                 size.width,
            height:                size.height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&TextureDescriptor {
            label: label.into(),
            size: extend,
            mip_level_count: 1,
            sample_count,
            dimension: TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&TextureViewDescriptor::default());

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: "depth_texture_sampler".into(),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: MipmapFilterMode::Nearest,
            compare: None, // doesn't work on iOS 12 Some(wgpu::CompareFunction::LessEqual), // 5.
            // compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
            size,
            channels: 1,
        }
    }
}

fn _save_rgba_image(buffer: &[u8], width: u32, height: u32, path: &str) -> Result<()> {
    let img: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_raw(width, height, buffer.to_vec())
        .ok_or(anyhow!("Failed to create image buffer"))?;

    img.save(Path::new(path))?;
    Ok(())
}

/// The mip levels with their sizes, see `hilen_pixels::mip_chain`.
fn mip_chain(data: &[u8], size: Size<u32>, channels: u8) -> Vec<(Vec<u8>, Size<u32>)> {
    hilen_pixels::mip_chain(data, size.width, size.height, channels)
        .into_iter()
        .map(|(pixels, (width, height))| (pixels, Size::new(width, height)))
        .collect()
}
