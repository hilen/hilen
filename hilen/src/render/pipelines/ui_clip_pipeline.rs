use std::num::NonZeroU64;

use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindingResource, Buffer, BufferBinding,
    PipelineLayoutDescriptor, RenderPass, RenderPipeline, ShaderStages, StencilOperation, include_wgsl,
};

use crate::{
    gm::flat::{Point, Size},
    render::{
        data::{ClipView, UIRectInstance},
        device_helper::DeviceHelper,
        pipelines::pipeline_type::PipelineType,
        uniform::{UniformBind, make_storage_layout, make_uniform_layout},
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

        let shader = device.create_shader_module(include_wgsl!("shaders/ui_clip.wgsl"));

        let view_layout = make_uniform_layout("ui_clip_view_layout", ShaderStages::VERTEX_FRAGMENT);
        let instances_layout = make_storage_layout("ui_clip_instances_layout", ShaderStages::FRAGMENT);

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

        let range = self.instances.range();

        let instances_bind = Window::device().create_bind_group(&BindGroupDescriptor {
            label:   Some("ui_clip_instances_bind"),
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

        pass.set_pipeline(if enter { &self.enter } else { &self.leave });
        pass.set_bind_group(0, self.view.bind(), &[]);
        pass.set_bind_group(1, &instances_bind, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, self.instances.slice());
        pass.draw(PipelineType::Color.vertex_range(), 0..1);
    }
}
