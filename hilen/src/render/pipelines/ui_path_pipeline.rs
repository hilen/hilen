use wgpu::{
    IndexFormat, PipelineLayoutDescriptor, PolygonMode, PrimitiveTopology, RenderPass, RenderPipeline,
    include_wgsl,
};

use crate::{
    gm::flat::Point,
    render::{data::PathData, device_helper::DeviceHelper, vertex_layout::VertexLayout},
    window::Window,
};

#[derive(Debug)]
pub struct UIPathPipeline {
    pipeline: RenderPipeline,
}

impl Default for UIPathPipeline {
    fn default() -> Self {
        let device = Window::device();

        let shader = device.create_shader_module(include_wgsl!("shaders/ui_path.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label:              Some("Path Pipeline Layout"),
            bind_group_layouts: &[Some(PathData::uniform_layout())],
            immediate_size:     0,
        });

        let pipeline = device.pipeline(
            "Path Fill Render Pipeline",
            &pipeline_layout,
            &shader,
            PolygonMode::Fill,
            PrimitiveTopology::TriangleList,
            &[Point::VERTEX_LAYOUT],
        );

        Self { pipeline }
    }
}

impl UIPathPipeline {
    pub fn draw(&self, render_pass: &mut RenderPass, path: &PathData) {
        render_pass.set_pipeline(&self.pipeline);

        render_pass.set_bind_group(0, path.uniform_bind(), &[]);
        render_pass.set_vertex_buffer(0, path.vertex_buffer().slice(..));
        render_pass.set_index_buffer(path.index_buffer().slice(..), IndexFormat::Uint32);
        render_pass.draw_indexed(0..path.index_count(), 0, 0..1);
    }
}
