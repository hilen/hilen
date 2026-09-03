use bytemuck::Pod;
use indexmap::IndexMap;
use wgpu::{
    BindGroupLayout, Buffer, CompareFunction, PipelineLayoutDescriptor, PrimitiveTopology, RenderPass,
    RenderPipeline, ShaderModuleDescriptor, ShaderSource, ShaderStages,
};

use crate::{
    deps::refs::Weak,
    gm::flat::{Point, Size, Vertex2D},
    render::{
        device_helper::DeviceHelper,
        pipelines::pipeline_type::PipelineType,
        uniform::{InstanceBinding, UniformBind, draw_instances, instances_shader, make_uniform_layout},
        vec_buffer::VecBuffer,
        vertex_layout::VertexLayout,
    },
    window::{
        Window,
        image::{Image, RASTER_KEEP_FRAMES},
    },
};

/// What one draw of the image pipeline binds. A raster size selects an
/// svg raster, none means the image texture itself.
#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageKey {
    pub image:  Weak<Image>,
    pub raster: Option<(u32, u32)>,
}

impl From<Weak<Image>> for ImageKey {
    fn from(image: Weak<Image>) -> Self {
        Self { image, raster: None }
    }
}

impl ImageKey {
    fn with_bind(&self, draw: impl FnOnce(&wgpu::BindGroup)) {
        match self.raster {
            Some((width, height)) => self
                .image
                .svg
                .as_ref()
                .expect("raster key on an image without an svg")
                .with_bind(Size::new(width, height), draw),
            None => draw(self.image.bind()),
        }
    }
}

pub struct RectPipeline<
    const TYPE: PipelineType,
    const NAME: &'static str,
    const SHADER_CODE: &'static str,
    View,
    Instance,
> {
    pipeline: RenderPipeline,

    vertex_buffer: Buffer,

    view: UniformBind<View>,

    /// Binds the instance buffer for the fragment stage. The bind group itself
    /// cannot be cached here because it names the byte range of one flush, and
    /// the buffer bump allocates a new range for every flush of the frame.
    instances_layout: BindGroupLayout,

    // Managed images live for the whole process, see docs/refs.md, so an
    // image key cannot die. Svg raster keys come and go with the sizes
    // drawn and are dropped with their rasters.
    instances: IndexMap<ImageKey, VecBuffer<Instance>>,
}

impl<
    const TYPE: PipelineType,
    const NAME: &'static str,
    const SHADER_CODE: &'static str,
    View: Default + Pod,
    Instance: VertexLayout,
> Default for RectPipeline<TYPE, NAME, SHADER_CODE, View, Instance>
{
    fn default() -> Self {
        let device = Window::device();
        let binding = InstanceBinding::device();

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label:  Some(&format!("{NAME}.wgsl")),
            source: ShaderSource::Wgsl(instances_shader(
                SHADER_CODE,
                size_of::<Instance>() as u64,
                binding,
            )),
        });

        let sprite_view_layout =
            make_uniform_layout(&format!("{NAME}_uniform_layout"), ShaderStages::VERTEX_FRAGMENT);

        let instances_layout = binding.layout(&format!("{NAME}_instances_layout"), ShaderStages::FRAGMENT);

        let mut bind_group_layouts = vec![Some(&sprite_view_layout), Some(&instances_layout)];

        if TYPE.image() {
            bind_group_layouts.push(Some(Image::uniform_layout()));
        }

        let uniform_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label:              Some(&format!("{NAME}_pipeline_layout")),
            bind_group_layouts: &bind_group_layouts,
            immediate_size:     0,
        });

        let pipeline = if TYPE.image() {
            device.pipeline(
                &format!("{NAME}_pipeline"),
                &uniform_layout,
                &shader,
                CompareFunction::Less,
                PrimitiveTopology::TriangleStrip,
                &[Vertex2D::VERTEX_LAYOUT, Instance::VERTEX_LAYOUT],
            )
        } else {
            device.pipeline(
                &format!("{NAME}_pipeline"),
                &uniform_layout,
                &shader,
                CompareFunction::Less,
                PrimitiveTopology::TriangleStrip,
                &[Point::VERTEX_LAYOUT, Instance::VERTEX_LAYOUT],
            )
        };

        Self {
            pipeline,
            vertex_buffer: TYPE.vertex_buffer(device),
            view: sprite_view_layout.into(),
            instances_layout,
            instances: IndexMap::default(),
        }
    }
}

impl<
    const TYPE: PipelineType,
    const NAME: &'static str,
    const SHADER_CODE: &'static str,
    View: Pod + PartialEq,
    Instance: Pod,
> RectPipeline<TYPE, NAME, SHADER_CODE, View, Instance>
{
    pub fn add(&mut self, instance: Instance) {
        assert!(TYPE.color());
        self.instances.entry(ImageKey::default()).or_default().push(instance);
    }

    pub fn add_with_image(&mut self, instance: Instance, image: impl Into<ImageKey>) {
        assert!(TYPE.image());
        self.instances.entry(image.into()).or_default().push(instance);
    }

    pub fn draw(&mut self, render_pass: &mut RenderPass, view: View) {
        if self.instances.is_empty() {
            return;
        }

        render_pass.set_pipeline(&self.pipeline);

        self.view.update(view);

        for (key, instances) in &mut self.instances {
            if instances.is_empty() {
                continue;
            }

            instances.load();

            render_pass.set_bind_group(0, self.view.bind(), &[]);

            if TYPE.image() {
                key.with_bind(|bind| render_pass.set_bind_group(2, bind, &[]));
            }

            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

            draw_instances(
                render_pass,
                &self.instances_layout,
                &format!("{NAME}_instances_bind"),
                1,
                1,
                TYPE.vertex_range(),
                instances,
            );
        }

        // After the loads, a key added this frame has its frame stamped
        // only by load and would otherwise be dropped before it draws.
        let frame = Window::render_frame();
        self.instances
            .retain(|key, instances| key.raster.is_none() || instances.frame() + RASTER_KEEP_FRAMES >= frame);
    }
}
