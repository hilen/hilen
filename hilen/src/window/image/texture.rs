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

/// Level 0 is the image itself, every next level halves it with a 2 by 2
/// box filter, an odd edge keeps its last row or column, down to 1 by 1.
/// Alpha weighted, so a transparent neighbor does not bleed its color
/// into a covered texel.
fn mip_chain(data: &[u8], size: Size<u32>, channels: u8) -> Vec<(Vec<u8>, Size<u32>)> {
    let channels = usize::from(channels);
    let mut levels = vec![(data.to_vec(), size)];
    while let Some((pixels, size)) = levels.last()
        && (size.width > 1 || size.height > 1)
    {
        let next = Size::new((size.width / 2).max(1), (size.height / 2).max(1));
        let mut out = Vec::with_capacity((next.width * next.height) as usize * channels);
        for y in 0..next.height {
            for x in 0..next.width {
                let x0 = (x * 2).min(size.width - 1);
                let x1 = (x * 2 + 1).min(size.width - 1);
                let y0 = (y * 2).min(size.height - 1);
                let y1 = (y * 2 + 1).min(size.height - 1);
                let texel = |px: u32, py: u32| {
                    let at = (py * size.width + px) as usize * channels;
                    &pixels[at..at + channels]
                };
                let four = [texel(x0, y0), texel(x1, y0), texel(x0, y1), texel(x1, y1)];
                if channels == 4 {
                    let alpha: u32 = four.iter().map(|t| u32::from(t[3])).sum();
                    for channel in 0..3 {
                        let weighted: u32 =
                            four.iter().map(|t| u32::from(t[channel]) * u32::from(t[3])).sum();
                        let value = (weighted + alpha / 2).checked_div(alpha).unwrap_or(0);
                        out.push(u8::try_from(value).expect("weighted mean fits u8"));
                    }
                    out.push(u8::try_from((alpha + 2) / 4).expect("mean alpha fits u8"));
                } else {
                    for channel in 0..channels {
                        let sum: u32 = four.iter().map(|t| u32::from(t[channel])).sum();
                        out.push(u8::try_from((sum + 2) / 4).expect("mean fits u8"));
                    }
                }
            }
        }
        levels.push((out, next));
    }
    levels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mip_chain_halves_down_to_one_texel() {
        let data = vec![255; 4 * 4 * 4];
        let levels = mip_chain(&data, Size::new(4, 4), 4);
        let sizes: Vec<(u32, u32)> = levels.iter().map(|(_, s)| (s.width, s.height)).collect();
        assert_eq!(sizes, [(4, 4), (2, 2), (1, 1)]);
        assert_eq!(levels[2].0, [255, 255, 255, 255]);
    }

    #[test]
    fn mip_chain_keeps_odd_edges_and_weights_by_alpha() {
        // A 3 by 1 row: opaque red, transparent green, opaque red.
        let data = vec![255, 0, 0, 255, 0, 255, 0, 0, 255, 0, 0, 255];
        let levels = mip_chain(&data, Size::new(3, 1), 4);
        assert_eq!(levels[1].1, Size::new(1, 1));
        // The transparent green texel contributes no color, only alpha.
        assert_eq!(levels[1].0, [255, 0, 0, 128]);
    }

    #[test]
    fn mip_chain_averages_single_channel() {
        let data = vec![0, 100, 200, 100];
        let levels = mip_chain(&data, Size::new(2, 2), 1);
        assert_eq!(levels[1].0, [100]);
    }
}
