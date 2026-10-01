use wgpu::{
    BindGroup, ColorTargetState, ColorWrites, CommandEncoder, FragmentState, FrontFace, LoadOp,
    MultisampleState, Operations, PipelineCompilationOptions, PipelineLayoutDescriptor, PolygonMode,
    PrimitiveState, PrimitiveTopology, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderSource, StoreOp, TextureFormat, TextureView,
    VertexState,
};

use crate::window::{Window, image::Image};

/// The last step of a scene picture: copies the resolved frame into the
/// image's own texture with the color divided by its alpha. The scene
/// pipelines draw in the surface format with the frame's sample count, an
/// image is plain RGBA with straight alpha, so the frame cannot be the
/// image itself.
pub(crate) struct PicturePipeline {
    pipeline: RenderPipeline,
}

impl Default for PicturePipeline {
    fn default() -> Self {
        let device = Window::device();
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label:  Some("picture.wgsl"),
            source: ShaderSource::Wgsl(include_str!("shaders/picture.wgsl").into()),
        });
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label:              "picture_pipeline_layout".into(),
            bind_group_layouts: &[Some(Image::uniform_layout())],
            immediate_size:     0,
        });
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label:          "picture_pipeline".into(),
            layout:         (&layout).into(),
            vertex:         VertexState {
                module:              &shader,
                entry_point:         "v_main".into(),
                compilation_options: PipelineCompilationOptions::default(),
                buffers:             &[],
            },
            fragment:       FragmentState {
                module:              &shader,
                entry_point:         "f_main".into(),
                compilation_options: PipelineCompilationOptions::default(),
                targets:             &[ColorTargetState {
                    format:     TextureFormat::Rgba8Unorm,
                    blend:      None,
                    write_mask: ColorWrites::ALL,
                }
                .into()],
            }
            .into(),
            primitive:      PrimitiveState {
                topology:           PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face:         FrontFace::Ccw,
                cull_mode:          None,
                polygon_mode:       PolygonMode::Fill,
                unclipped_depth:    false,
                conservative:       false,
            },
            depth_stencil:  None,
            multisample:    MultisampleState::default(),
            cache:          None,
            multiview_mask: None,
        });
        Self { pipeline }
    }
}

impl PicturePipeline {
    /// Draws the frame bound by `frame` over the whole of `target`.
    pub(crate) fn draw(&self, encoder: &mut CommandEncoder, frame: &BindGroup, target: &TextureView) {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label:                    Some("Picture Pass"),
            color_attachments:        &[Some(RenderPassColorAttachment {
                view:           target,
                depth_slice:    None,
                resolve_target: None,
                ops:            Operations {
                    load:  LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set:      None,
            timestamp_writes:         None,
            multiview_mask:           None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, frame, &[]);
        pass.draw(0..3, 0..1);
    }
}
