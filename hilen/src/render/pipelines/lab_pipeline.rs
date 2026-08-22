use std::num::NonZeroU64;

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindingResource, Buffer, BufferBinding,
    PipelineLayoutDescriptor, PrimitiveTopology, RenderPass, RenderPipeline, ShaderModuleDescriptor,
    ShaderSource, ShaderStages,
};

use crate::{
    gm::flat::{Point, Vertex2D},
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

/// A `UIRectPipeline` whose shader comes from a string at runtime instead of a
/// const generic. The shader lab builds one of these per variant, which a const
/// generic cannot express because the sources are only known once the caller
/// hands them over.
///
/// The pipeline shape is a variable too. A variant can take the `Vertex2D`
/// model buffer and a texture the way `ui_image.wgsl` does, or the bare `Point`
/// buffer with no texture the way `ui_rect.wgsl` does. That is the whole point:
/// when one of those shaders draws on a device and the other does not, the
/// difference has to be isolated with nothing else moving.
pub(crate) struct LabPipeline {
    pipeline:         RenderPipeline,
    vertex_buffer:    Buffer,
    view:             UniformBind<RectView>,
    instances_layout: BindGroupLayout,
    instances:        VecBuffer<UIRectInstance>,
    textured:         bool,
    vertex_uv:        bool,
}

impl LabPipeline {
    /// The source must be a drop in replacement for `ui_rect.wgsl`: a `v_main`
    /// taking the model vertex plus a `UIRectInstance`, a `f_main`, a
    /// `RectView` uniform at group 0 binding 0 and an instance storage array at
    /// group 1 binding 0. A textured variant also gets a texture and a sampler
    /// at group 2. Anything that holds to that contract can be compared against
    /// anything else.
    pub(crate) fn new(name: &str, source: &str, vertex_uv: bool, textured: bool) -> Self {
        let device = Window::device();

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label:  Some(name),
            source: ShaderSource::Wgsl(source.into()),
        });

        let view_layout = make_uniform_layout(name, ShaderStages::VERTEX_FRAGMENT);
        let instances_layout = make_storage_layout(name, ShaderStages::FRAGMENT);

        let mut bind_group_layouts = vec![Some(&view_layout), Some(&instances_layout)];

        if textured {
            bind_group_layouts.push(Some(Image::uniform_layout()));
        }

        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label:              Some(name),
            bind_group_layouts: &bind_group_layouts,
            immediate_size:     0,
        });

        // The layout slice has to be built from consts in place. Routing it
        // through a local kills the promotion to `'static` that `pipeline`
        // needs.
        let pipeline = if vertex_uv {
            device.pipeline(
                name,
                &layout,
                &shader,
                PolygonMode::Fill,
                PrimitiveTopology::TriangleStrip,
                &[Vertex2D::VERTEX_LAYOUT, UIRectInstance::VERTEX_LAYOUT],
            )
        } else {
            device.pipeline(
                name,
                &layout,
                &shader,
                PolygonMode::Fill,
                PrimitiveTopology::TriangleStrip,
                &[Point::VERTEX_LAYOUT, UIRectInstance::VERTEX_LAYOUT],
            )
        };

        let vertex_type = if vertex_uv {
            PipelineType::Image
        } else {
            PipelineType::Color
        };

        Self {
            pipeline,
            vertex_buffer: vertex_type.vertex_buffer(device),
            view: view_layout.into(),
            instances_layout,
            instances: VecBuffer::default(),
            textured,
            vertex_uv,
        }
    }

    pub(crate) fn add(&mut self, instance: UIRectInstance) {
        self.instances.push(instance);
    }

    pub(crate) fn draw(&mut self, pass: &mut RenderPass, view: RectView, image: Option<&BindGroup>) {
        if self.instances.is_empty() {
            return;
        }

        // A textured variant cannot draw without its texture, and drawing it
        // anyway would read as a shader failure rather than a missing image.
        if self.textured && image.is_none() {
            return;
        }

        self.view.update(view);
        self.instances.load();

        let range = self.instances.range();

        let instances_bind = Window::device().create_bind_group(&BindGroupDescriptor {
            label:   Some("lab_instances_bind"),
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

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, self.view.bind(), &[]);
        pass.set_bind_group(1, &instances_bind, &[]);

        if let Some(image) = image {
            pass.set_bind_group(2, image, &[]);
        }

        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, self.instances.slice());

        let vertex_type = if self.vertex_uv {
            PipelineType::Image
        } else {
            PipelineType::Color
        };

        pass.draw(vertex_type.vertex_range(), 0..self.instances.len());
    }
}
