use wgpu::{
    BindGroup, BindGroupLayout, Buffer, CompareFunction, PipelineLayoutDescriptor, PrimitiveTopology,
    RenderPass, RenderPipeline, ShaderModuleDescriptor, ShaderSource, ShaderStages,
};

use crate::{
    gm::flat::Point,
    render::{
        data::{RectView, UIRectInstance},
        device_helper::DeviceHelper,
        pipelines::pipeline_type::PipelineType,
        uniform::{InstanceBinding, UniformBind, draw_instances, instances_shader, make_uniform_layout},
        vec_buffer::VecBuffer,
        vertex_layout::VertexLayout,
    },
    window::{Window, image::Image},
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
        let binding = InstanceBinding::device();

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label:  Some("ui_backdrop.wgsl"),
            source: ShaderSource::Wgsl(instances_shader(
                UI_BACKDROP_CODE,
                size_of::<UIRectInstance>() as u64,
                binding,
            )),
        });

        let view_layout = make_uniform_layout("ui_backdrop_uniform_layout", ShaderStages::VERTEX_FRAGMENT);

        let instances_layout = binding.layout("ui_backdrop_instances_layout", ShaderStages::FRAGMENT);

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
            CompareFunction::Less,
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

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, self.view.bind(), &[]);
        render_pass.set_bind_group(1, blurred, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

        draw_instances(
            render_pass,
            &self.instances_layout,
            "ui_backdrop_instances_bind",
            2,
            1,
            PipelineType::Color.vertex_range(),
            &self.instances,
        );
    }
}
