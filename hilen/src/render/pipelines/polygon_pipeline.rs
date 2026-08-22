use bytemuck::{Pod, Zeroable, cast_slice};
use wgpu::{
    BindGroup, BindGroupLayout, Buffer, BufferUsages, IndexFormat, PipelineLayoutDescriptor, PolygonMode,
    PrimitiveTopology, RenderPass, RenderPipeline, ShaderStages, include_wgsl,
};

use crate::{
    gm::{checked_usize_to_u32, color::Color, flat::Point},
    render::{
        device_helper::DeviceHelper,
        shader_data::SpriteView,
        uniform::{UniformBind, make_bind, make_uniform_layout},
        vertex_layout::VertexLayout,
    },
    window::{VertexBuffer, Window},
};

#[repr(C)]
#[derive(Default, Debug, Copy, Clone, Zeroable, Pod, PartialEq)]
struct PolygonView {
    color:    Color,
    pos:      Point,
    rot:      f32,
    _padding: u32,
}

#[derive(Debug)]
pub struct PolygonPipeline {
    pipeline: RenderPipeline,

    view: UniformBind<SpriteView>,

    polygon_view_layout: BindGroupLayout,

    // TODO:
    #[allow(clippy::type_complexity)]
    polygons: Vec<(Buffer, usize, Option<Buffer>, Option<usize>, BindGroup)>,
}

impl Default for PolygonPipeline {
    fn default() -> Self {
        let device = Window::device();

        let shader = device.create_shader_module(include_wgsl!("shaders/polygon.wgsl"));

        let view_layout = make_uniform_layout("polygon_sprite_view_layout", ShaderStages::VERTEX);
        let polygon_view_layout = make_uniform_layout("polygon_view_layout", ShaderStages::VERTEX_FRAGMENT);

        let uniform_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label:              "polygon_pipeline_layout".into(),
            bind_group_layouts: &[Some(&view_layout), Some(&polygon_view_layout)],
            immediate_size:     0,
        });

        let pipeline = device.pipeline(
            "polygon_pipeline",
            &uniform_layout,
            &shader,
            PolygonMode::Fill,
            PrimitiveTopology::TriangleList,
            &[Point::VERTEX_LAYOUT],
        );

        Self {
            pipeline,
            view: view_layout.into(),
            polygon_view_layout,
            polygons: vec![],
        }
    }
}

impl PolygonPipeline {
    pub fn add(&mut self, buffer: &VertexBuffer, pos: Point, color: Color, rot: f32) {
        assert!(!buffer.vertices.is_empty(), "Adding polygon with 0 vertices");

        self.polygons.push((
            Window::device().buffer_from_bytes(cast_slice(&buffer.vertices), BufferUsages::VERTEX),
            buffer.vertices.len(),
            buffer
                .indices
                .as_ref()
                .map(|a| Window::device().buffer_from_bytes(cast_slice(a), BufferUsages::INDEX)),
            buffer.indices.as_ref().map(Vec::len),
            make_bind(
                &PolygonView {
                    color,
                    pos,
                    rot,
                    _padding: 0,
                },
                &self.polygon_view_layout,
            ),
        ));
    }

    pub fn draw(&mut self, render_pass: &mut RenderPass, view: SpriteView) {
        if self.polygons.is_empty() {
            return;
        }

        render_pass.set_pipeline(&self.pipeline);

        self.view.update(view);

        render_pass.set_bind_group(0, self.view.bind(), &[]);

        for (buffer, points_len, index_buffer, index_len, view) in &self.polygons {
            render_pass.set_bind_group(1, view, &[]);
            render_pass.set_vertex_buffer(0, buffer.slice(..));

            if let (Some(index_buffer), Some(index_len)) = (index_buffer, index_len) {
                render_pass.set_index_buffer(index_buffer.slice(..), IndexFormat::Uint16);
                render_pass.draw_indexed(0..checked_usize_to_u32(*index_len), 0, 0..1);
            } else {
                render_pass.draw(0..checked_usize_to_u32(*points_len), 0..1);
            }
        }
    }

    pub fn clear(&mut self) {
        self.polygons.clear();
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        // Web requirements
        assert_eq!(size_of::<PolygonView>() % 16, 0);
    }
}
