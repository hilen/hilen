use wgpu::{
    AddressMode, Device, Extent3d, FilterMode, MipmapFilterMode, Origin3d, Sampler, SamplerDescriptor,
    TexelCopyBufferLayout, TexelCopyTextureInfo, TextureAspect, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
};

#[cfg(any(feature = "level", feature = "scene"))]
use crate::window::surface_texture_format;
use crate::{
    gm::flat::Size,
    image::{ImageFilter, TextureLevels, TextureRawData},
    window::Window,
};

#[derive(Debug)]
pub struct Texture {
    pub texture:  wgpu::Texture,
    pub view:     TextureView,
    pub sampler:  Sampler,
    pub size:     Size<u32>,
    pub channels: u8,
}

impl Texture {
    pub(crate) const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth24PlusStencil8;

    pub fn from_raw_data(data: TextureRawData, label: &str) -> Self {
        Self::from_levels(data.into_levels(), label)
    }

    /// Only the calls that need the device, the pixel work is done in
    /// `TextureRawData::into_levels`.
    pub(crate) fn from_levels(
        TextureLevels {
            levels,
            size,
            channels,
        }: TextureLevels,
        label: &str,
    ) -> Self {
        let extend_size = Extent3d {
            width:                 size.width,
            height:                size.height,
            depth_or_array_layers: 1,
        };

        // Plain Unorm, the whole pipeline works on encoded sRGB values.
        // Image bytes are already encoded, sampling must return them
        // unchanged. An sRGB format here would decode on sample and the
        // image would render one decode too dark.
        let format = if channels == 1 {
            TextureFormat::R8Unorm
        } else {
            TextureFormat::Rgba8Unorm
        };

        let device = Window::device();

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

        Self {
            texture,
            view,
            sampler: Self::image_sampler(ImageFilter::Linear),
            size,
            channels,
        }
    }

    /// The sampler of an image texture. Linear blends the texels a pixel
    /// falls between, nearest keeps each texel a hard square, what pixel
    /// art drawn bigger than its bitmap needs.
    pub(crate) fn image_sampler(filter: ImageFilter) -> Sampler {
        let (filter, mipmap_filter) = match filter {
            ImageFilter::Linear => (FilterMode::Linear, MipmapFilterMode::Linear),
            ImageFilter::Nearest => (FilterMode::Nearest, MipmapFilterMode::Nearest),
        };
        Window::device().create_sampler(&SamplerDescriptor {
            label: "texture_sampler".into(),
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: filter,
            min_filter: filter,
            mipmap_filter,
            ..Default::default()
        })
    }

    /// An RGBA texture of 1 level that `write_level_0` fills again and
    /// again, the live picture of a camera. Clamped edges.
    #[cfg(feature = "camera")]
    pub(crate) fn stream(size: Size<u32>, label: &str) -> Self {
        let device = Window::device();

        let texture = device.create_texture(&TextureDescriptor {
            label:           label.into(),
            size:            Extent3d {
                width:                 size.width,
                height:                size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count:    1,
            dimension:       TextureDimension::D2,
            format:          TextureFormat::Rgba8Unorm,
            usage:           TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats:    &[],
        });

        let view = texture.create_view(&TextureViewDescriptor::default());

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: "stream_sampler".into(),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Nearest,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
            size,
            channels: 4,
        }
    }

    /// Writes RGBA pixels of `size` into the first level of `texture`.
    #[cfg(feature = "camera")]
    pub(crate) fn write_level_0(texture: &wgpu::Texture, size: Size<u32>, rgba: &[u8]) {
        Window::queue().write_texture(
            TexelCopyTextureInfo {
                aspect: TextureAspect::All,
                texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
            },
            rgba,
            TexelCopyBufferLayout {
                offset:         0,
                bytes_per_row:  Some(4 * size.width),
                rows_per_image: Some(size.height),
            },
            Extent3d {
                width:                 size.width,
                height:                size.height,
                depth_or_array_layers: 1,
            },
        );
    }

    /// A blank RGBA texture a pass can draw into, the image pipeline can
    /// sample and a scene photo reads back, one mip level, clamped edges.
    #[cfg(any(all(feature = "video", ffmpeg), feature = "scene"))]
    pub(crate) fn render_target(size: Size<u32>, label: &str) -> Self {
        let device = Window::device();

        let texture = device.create_texture(&TextureDescriptor {
            label:           label.into(),
            size:            Extent3d {
                width:                 size.width,
                height:                size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count:    1,
            dimension:       TextureDimension::D2,
            format:          TextureFormat::Rgba8Unorm,
            usage:           TextureUsages::TEXTURE_BINDING
                | TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::COPY_SRC,
            view_formats:    &[],
        });

        let view = texture.create_view(&TextureViewDescriptor::default());

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: "render_target_sampler".into(),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
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
            channels: 4,
        }
    }

    /// A texture a pixel art level draws into at art resolution, in the
    /// surface format so the level pipelines can draw into it, one sample,
    /// read back with the nearest filter so each texel stays a hard square.
    #[cfg(any(feature = "level", feature = "scene"))]
    pub(crate) fn pixel_target(size: Size<u32>) -> Self {
        let device = Window::device();

        let texture = device.create_texture(&TextureDescriptor {
            label:           "pixel_art_target".into(),
            size:            Extent3d {
                width:                 size.width,
                height:                size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count:    1,
            dimension:       TextureDimension::D2,
            format:          surface_texture_format(),
            usage:           TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            view_formats:    &[],
        });

        let view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: "pixel_art_sampler".into(),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: MipmapFilterMode::Nearest,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
            size,
            channels: 4,
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
            // Attachment only. Nothing samples the frame depth, and WebGL2
            // can multisample a renderbuffer but not a texture, so any
            // other usage on a multisampled depth dies at creation there.
            usage: TextureUsages::RENDER_ATTACHMENT,
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
