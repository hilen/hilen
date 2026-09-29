use bytemuck::{Pod, Zeroable};
use wgpu::{
    BindGroupLayout, CompareFunction, PipelineLayoutDescriptor, PrimitiveTopology, RenderPass,
    RenderPipeline, ShaderModuleDescriptor, ShaderSource, ShaderStages,
};

use crate::{
    gm::flat::Point,
    render::{
        device_helper::DeviceHelper,
        uniform::{UniformBind, make_uniform_layout},
    },
    window::{Window, image::Image},
};

/// Which part of the art resolution texture the level area shows, and at
/// what depth.
#[repr(C)]
#[derive(Default, Debug, Copy, Clone, Zeroable, Pod, PartialEq)]
pub(crate) struct PixelBlit {
    pub(crate) uv_min: Point,
    pub(crate) uv_max: Point,
    pub(crate) z:      f32,
    pad:               [f32; 3],
}

/// Draws the art resolution texture of a pixel art level over the level
/// area in the frame pass, with the frame MSAA, the nearest filter and
/// alpha blending over the clear color.
#[derive(Debug)]
pub(crate) struct PixelBlitPipeline {
    pipeline: RenderPipeline,
    blit:     UniformBind<PixelBlit>,
}

impl Default for PixelBlitPipeline {
    fn default() -> Self {
        let device = Window::device();
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label:  Some("pixel_blit.wgsl"),
            source: ShaderSource::Wgsl(include_str!("shaders/pixel_blit.wgsl").into()),
        });
        let blit_layout: BindGroupLayout = make_uniform_layout("pixel_blit_layout", ShaderStages::VERTEX);
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label:              "pixel_blit_pipeline_layout".into(),
            bind_group_layouts: &[Some(&blit_layout), Some(Image::uniform_layout())],
            immediate_size:     0,
        });
        let pipeline = device.pipeline(
            "pixel_blit_pipeline",
            &layout,
            &shader,
            CompareFunction::Always,
            PrimitiveTopology::TriangleStrip,
            &[],
        );
        Self {
            pipeline,
            blit: blit_layout.into(),
        }
    }
}

impl PixelBlitPipeline {
    /// Shows the part `uv_min` to `uv_max` of the texture bound by
    /// `texture` over the whole viewport, at depth `z`.
    pub(crate) fn draw(
        &mut self,
        pass: &mut RenderPass,
        texture: &wgpu::BindGroup,
        uv_min: Point,
        uv_max: Point,
        z: f32,
    ) {
        self.blit.update(PixelBlit {
            uv_min,
            uv_max,
            z,
            pad: [0.0; 3],
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, self.blit.bind(), &[]);
        pass.set_bind_group(1, texture, &[]);
        pass.draw(0..4, 0..1);
    }
}
