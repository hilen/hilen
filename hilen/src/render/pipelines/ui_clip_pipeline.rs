use wgpu::{
    BindGroupLayout, Buffer, PipelineLayoutDescriptor, RenderPass, RenderPipeline, ShaderModuleDescriptor,
    ShaderSource, ShaderStages, StencilOperation,
};

use crate::{
    gm::flat::{Point, Size},
    render::{
        data::{ClipView, UIRectInstance},
        device_helper::DeviceHelper,
        pipelines::pipeline_type::PipelineType,
        uniform::{InstanceBinding, UniformBind, draw_instances, instances_shader, make_uniform_layout},
        vec_buffer::VecBuffer,
        vertex_layout::VertexLayout,
    },
    window::{Window, msaa_sample_count},
};

/// Writes the rounded clip mask into the stencil. A view that clips to
/// its bounds with rounded corners enters by drawing its shape with the
/// increment pipeline, which raises the stencil inside the shape by one,
/// draws its subtree at that raised reference, then leaves by drawing the
/// same shape with the decrement pipeline. Nested rounded clips stack the
/// same way. See `depth_stencil_state`.
pub(crate) struct UIClipPipeline {
    enter:            RenderPipeline,
    leave:            RenderPipeline,
    vertex_buffer:    Buffer,
    view:             UniformBind<ClipView>,
    instances_layout: BindGroupLayout,
    instances:        VecBuffer<UIRectInstance>,
}

impl Default for UIClipPipeline {
    fn default() -> Self {
        let device = Window::device();
        let binding = InstanceBinding::device();

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label:  Some("ui_clip.wgsl"),
            source: ShaderSource::Wgsl(instances_shader(
                include_str!("shaders/ui_clip.wgsl"),
                size_of::<UIRectInstance>() as u64,
                binding,
            )),
        });

        let view_layout = make_uniform_layout("ui_clip_view_layout", ShaderStages::VERTEX_FRAGMENT);
        let instances_layout = binding.layout("ui_clip_instances_layout", ShaderStages::FRAGMENT);

        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label:              Some("ui_clip_pipeline_layout"),
            bind_group_layouts: &[Some(&view_layout), Some(&instances_layout)],
            immediate_size:     0,
        });

        let vertex_layout = &[Point::VERTEX_LAYOUT, UIRectInstance::VERTEX_LAYOUT];

        Self {
            enter: device.mask_pipeline(
                "ui_clip_enter",
                &layout,
                &shader,
                vertex_layout,
                StencilOperation::IncrementClamp,
            ),
            leave: device.mask_pipeline(
                "ui_clip_leave",
                &layout,
                &shader,
                vertex_layout,
                StencilOperation::DecrementClamp,
            ),
            vertex_buffer: PipelineType::Color.vertex_buffer(device),
            view: view_layout.into(),
            instances_layout,
            instances: VecBuffer::default(),
        }
    }
}

impl UIClipPipeline {
    pub(crate) fn enter(&mut self, pass: &mut RenderPass, resolution: Size, shape: UIRectInstance) {
        self.draw(pass, true, resolution, shape);
    }

    pub(crate) fn leave(&mut self, pass: &mut RenderPass, resolution: Size, shape: UIRectInstance) {
        self.draw(pass, false, resolution, shape);
    }

    fn draw(&mut self, pass: &mut RenderPass, enter: bool, resolution: Size, shape: UIRectInstance) {
        self.view.update(ClipView {
            resolution,
            threshold: if msaa_sample_count() > 1 { 0.004 } else { 0.5 },
            _padding: 0,
        });

        self.instances.push(shape);
        self.instances.load();

        pass.set_pipeline(if enter { &self.enter } else { &self.leave });
        pass.set_bind_group(0, self.view.bind(), &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

        draw_instances(
            pass,
            &self.instances_layout,
            "ui_clip_instances_bind",
            1,
            1,
            PipelineType::Color.vertex_range(),
            &self.instances,
        );
    }
}
