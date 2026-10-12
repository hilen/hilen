//! The textures a frame draws into besides the surface. Each is made when
//! it is first needed and again when the size of the frame changes.

use super::{SCENE_TEXTURE_USAGE, State, surface_texture_format};
use crate::{
    gm::{LossyConvert, flat::Size},
    window::{Window, image::Texture, msaa::msaa_sample_count},
};

impl State {
    /// The textures made for the old sample count go, each is made again
    /// by the next frame.
    pub(crate) fn sample_count_changed(&mut self) {
        self.depth_texture = None;
        self.msaa_texture = None;
        self.kept = None;
    }

    /// A view of the multisampled texture, none with 1 sample.
    pub(super) fn msaa_view(&self) -> Option<wgpu::TextureView> {
        self.msaa_texture
            .as_ref()
            .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()))
    }

    pub(super) fn ensure_depth_texture(&mut self, size: Size<u32>) {
        let up_to_date = self.depth_texture.as_ref().is_some_and(|texture| texture.size == size);

        if up_to_date {
            return;
        }

        self.depth_texture = Some(Texture::create_depth_texture(
            Window::device(),
            size,
            msaa_sample_count(),
            "depth_texture",
        ));
    }

    /// The multisampled color target the frame renders into before
    /// resolving to the presentable texture. Not created when
    /// multisampling is off, then the frame renders straight into the
    /// resolve target.
    pub(super) fn ensure_msaa_texture(&mut self, size: Size<u32>) {
        if msaa_sample_count() == 1 {
            return;
        }

        let up_to_date = self.msaa_texture.as_ref().is_some_and(|texture| {
            texture.size().width == size.width && texture.size().height == size.height
        });

        if up_to_date {
            return;
        }

        // The frame before was in the old texture.
        self.kept = None;

        self.msaa_texture = Some(Window::device().create_texture(&wgpu::TextureDescriptor {
            label:           Some("MSAA Render Texture"),
            size:            wgpu::Extent3d {
                width:                 size.width,
                height:                size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count:    msaa_sample_count(),
            dimension:       wgpu::TextureDimension::D2,
            format:          surface_texture_format(),
            usage:           wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats:    &[],
        }));
    }

    pub(super) fn ensure_offscreen_texture(&mut self) {
        let size = Window::render_size();
        let width: u32 = size.width.lossy_convert();
        let height: u32 = size.height.lossy_convert();

        let up_to_date = self
            .offscreen_texture
            .as_ref()
            .is_some_and(|texture| texture.size().width == width && texture.size().height == height);

        if up_to_date {
            return;
        }

        // The frame before was in the old texture.
        self.kept = None;

        self.offscreen_texture = Some(Window::device().create_texture(&wgpu::TextureDescriptor {
            label:           Some("Offscreen Render Texture"),
            size:            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count:    1,
            dimension:       wgpu::TextureDimension::D2,
            format:          surface_texture_format(),
            usage:           wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats:    &[],
        }));
    }

    /// Intermediate render target for frames that sample themselves,
    /// for example for backdrop blur. Windowed only, the headless
    /// offscreen texture is sampleable directly.
    pub(super) fn ensure_scene_texture(&mut self, size: Size<u32>) {
        let up_to_date = self.scene_texture.as_ref().is_some_and(|texture| {
            texture.size().width == size.width && texture.size().height == size.height
        });

        if up_to_date {
            return;
        }

        // The frame before was in the old texture.
        self.kept = None;

        self.scene_texture = Some(Window::device().create_texture(&wgpu::TextureDescriptor {
            label:           Some("Scene Render Texture"),
            size:            wgpu::Extent3d {
                width:                 size.width,
                height:                size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count:    1,
            dimension:       wgpu::TextureDimension::D2,
            format:          surface_texture_format(),
            usage:           SCENE_TEXTURE_USAGE,
            view_formats:    &[],
        }));
    }
}
