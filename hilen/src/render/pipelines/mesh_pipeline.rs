use std::num::NonZeroU64;

use indexmap::IndexMap;
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindingResource, BufferBinding, IndexFormat,
    PipelineLayoutDescriptor, RenderPass, RenderPipeline, ShaderStages, include_wgsl,
};

use crate::{
    deps::refs::Weak,
    gm::volume::Vertex3D,
    render::{
        SceneView,
        data::MeshInstance,
        device_helper::DeviceHelper,
        uniform::{UniformBind, make_storage_layout, make_uniform_layout},
        vec_buffer::VecBuffer,
        vertex_layout::VertexLayout,
    },
    scene::Mesh,
    window::Window,
};

/// Draws every node of a frame, one instanced indexed draw per mesh.
pub struct MeshPipeline {
    pipeline: RenderPipeline,

    view: UniformBind<SceneView>,

    /// Binds the instance buffer for the fragment stage, see
    /// `RectPipeline` for why the group cannot be cached.
    instances_layout: BindGroupLayout,

    // A unit mesh lives for the whole process, so a key cannot die.
    instances: IndexMap<Weak<Mesh>, VecBuffer<MeshInstance>>,
}

impl Default for MeshPipeline {
    fn default() -> Self {
        let device = Window::device();

        let shader = device.create_shader_module(include_wgsl!("shaders/mesh.wgsl"));

        let view_layout = make_uniform_layout("mesh_view_layout", ShaderStages::VERTEX_FRAGMENT);
        let instances_layout = make_storage_layout("mesh_instances_layout", ShaderStages::FRAGMENT);

        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label:              "mesh_pipeline_layout".into(),
            bind_group_layouts: &[Some(&view_layout), Some(&instances_layout)],
            immediate_size:     0,
        });

        let pipeline = device.mesh_pipeline(
            "mesh_pipeline",
            &layout,
            &shader,
            &[Vertex3D::VERTEX_LAYOUT, MeshInstance::VERTEX_LAYOUT],
        );

        Self {
            pipeline,
            view: view_layout.into(),
            instances_layout,
            instances: IndexMap::default(),
        }
    }
}

impl MeshPipeline {
    pub(crate) fn add(&mut self, mesh: Weak<Mesh>, instance: MeshInstance) {
        self.instances.entry(mesh).or_default().push(instance);
    }

    pub(crate) fn draw(&mut self, render_pass: &mut RenderPass, view: SceneView) {
        if self.instances.values().all(VecBuffer::is_empty) {
            return;
        }

        render_pass.set_pipeline(&self.pipeline);

        self.view.update(view);
        render_pass.set_bind_group(0, self.view.bind(), &[]);

        for (mesh, instances) in &mut self.instances {
            if instances.is_empty() {
                continue;
            }

            instances.load();

            let range = instances.range();

            let instances_bind = Window::device().create_bind_group(&BindGroupDescriptor {
                label:   Some("mesh_instances_bind"),
                layout:  &self.instances_layout,
                entries: &[BindGroupEntry {
                    binding:  0,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: instances.buffer(),
                        offset: range.start,
                        size:   NonZeroU64::new(range.end - range.start),
                    }),
                }],
            });

            render_pass.set_bind_group(1, &instances_bind, &[]);
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, instances.slice());
            render_pass.set_index_buffer(mesh.index_buffer.slice(..), IndexFormat::Uint16);

            // Base vertex stays zero, an A7 draws nothing otherwise.
            render_pass.draw_indexed(0..mesh.index_count, 0, 0..instances.len());
        }
    }
}
