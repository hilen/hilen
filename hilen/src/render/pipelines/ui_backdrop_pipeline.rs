use std::num::NonZeroU64;

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindingResource, Buffer, BufferBinding,
    PipelineLayoutDescriptor, PrimitiveTopology, RenderPass, RenderPipeline, ShaderModuleDescriptor,
    ShaderSource, ShaderStages,
};

use crate::{
    gm::flat::Point,
    render::{
        data::{RectView, UIRectInstance},
        device_helper::DeviceHelper,
        pipelines::pipeline_type::PipelineType,
        uniform::{UniformBind, make_storage_layout, make_uniform_layout},
        vec_buffer::VecBuffer,
        vertex_layout::VertexLayout,
    },
    window::{PolygonMode, Window, image::Image},
};

const UI_BACKDROP_CODE: &str = include_str!("shaders/ui_backdrop.wgsl");

/// Draws one rounded rect that shows the blurred scene behind it,
/// tinted by the instance color. Unlike the batched pipelines it draws
/// immediately, mid frame, right after a blur barrier reopened the
/// pass. The instance buffer is bump allocated, so several barriers in
/// one frame do not overwrite each other.
pub struct UIBackdropPipeline {
    pipeline: RenderPipeline,

    vertex_buffer: Buffer,

    view: UniformBind<RectView>,

    /// Binds the instance buffer for the fragment stage. The bind group names
    /// the byte range of one flush, and the buffer bump allocates a new range
    /// for every flush, so it cannot be cached here.
    instances_layout: BindGroupLayout,

    instances: VecBuffer<UIRectInstance>,
}

impl Default for UIBackdropPipeline {
    fn default() -> Self {
        let device = Window::device();

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label:  Some("ui_backdrop.wgsl"),
            source: ShaderSource::Wgsl(UI_BACKDROP_CODE.into()),
        });

        let view_layout = make_uniform_layout("ui_backdrop_uniform_layout", ShaderStages::VERTEX_FRAGMENT);

        let instances_layout = make_storage_layout("ui_backdrop_instances_layout", ShaderStages::FRAGMENT);

        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label:              Some("ui_backdrop_pipeline_layout"),
            bind_group_layouts: &[
                Some(&view_layout),
                Some(Image::uniform_layout()),
                Some(&instances_layout),
            ],
            immediate_size:     0,
        });

        let pipeline = device.pipeline(
            "ui_backdrop_pipeline",
            &layout,
            &shader,
            PolygonMode::Fill,
            PrimitiveTopology::TriangleStrip,
            &[Point::VERTEX_LAYOUT, UIRectInstance::VERTEX_LAYOUT],
        );

        Self {
            pipeline,
            vertex_buffer: PipelineType::Color.vertex_buffer(device),
            view: view_layout.into(),
            instances_layout,
            instances: VecBuffer::default(),
        }
    }
}

impl UIBackdropPipeline {
    pub fn draw(
        &mut self,
        render_pass: &mut RenderPass,
        view: RectView,
        instance: UIRectInstance,
        blurred: &BindGroup,
    ) {
        self.view.update(view);
        self.instances.push(instance);
        self.instances.load();

        let range = self.instances.range();

        let instances_bind = Window::device().create_bind_group(&BindGroupDescriptor {
            label:   Some("ui_backdrop_instances_bind"),
            layout:  &self.instances_layout,
            entries: &[BindGroupEntry {
                binding:  0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: self.instances.buffer(),
                    offset: range.start,
                    size:   NonZeroU64::new(range.end - range.start),
                }),
            }],
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, self.view.bind(), &[]);
        render_pass.set_bind_group(1, blurred, &[]);
        render_pass.set_bind_group(2, &instances_bind, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instances.slice());

        render_pass.draw(PipelineType::Color.vertex_range(), 0..1);
    }
}
