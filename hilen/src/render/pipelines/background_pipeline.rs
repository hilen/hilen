use std::ops::Range;

use bytemuck::{Pod, Zeroable};
use wgpu::{
    BindGroup, Buffer, BufferUsages, PipelineLayoutDescriptor, PolygonMode, PrimitiveTopology, RenderPass,
    RenderPipeline, ShaderStages,
};

use crate::{
    gm::{
        checked_usize_to_u32,
        flat::{Point, Size, Vertex2D},
    },
    render::{
        buffer_helper::BufferHelper, device_helper::DeviceHelper, uniform::make_uniform_layout,
        vertex_layout::VertexLayout,
    },
    window::{Window, image::Image},
};

const VAL: f32 = 100_000.0;
const UV: f32 = 500.0;

const fn image_vertices() -> [Vertex2D; 4] {
    [
        Vertex2D {
            pos: Point::new(-VAL, VAL),
            uv:  Point::new(-UV, -UV),
        },
        Vertex2D {
            pos: Point::new(-VAL, -VAL),
            uv:  Point::new(-UV, UV),
        },
        Vertex2D {
            pos: Point::new(VAL, VAL),
            uv:  Point::new(UV, -UV),
        },
        Vertex2D {
            pos: Point::new(VAL, -VAL),
            uv:  Point::new(UV, UV),
        },
    ]
}

const VERTICES: [Vertex2D; 4] = image_vertices();

const RANGE: Range<u32> = 0..checked_usize_to_u32(VERTICES.len());

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Zeroable, Pod, PartialEq)]
struct BackgroundView {
    camera_pos:      Point<f32>,
    resolution:      Size<f32>,
    camera_rotation: f32,
    scale:           f32,
    _padding:        u64,
}

#[derive(Debug)]
pub struct BackgroundPipeline {
    render_pipeline: RenderPipeline,
    vertex_buffer:   Buffer,
    view_buffer:     Buffer,
    view_bind:       BindGroup,
    view:            BackgroundView,
}

impl Default for BackgroundPipeline {
    fn default() -> Self {
        let device = Window::device();
        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/background.wgsl"));

        let vertex_layout = make_uniform_layout("background_drawer_vertex_layout", ShaderStages::VERTEX);

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label:              "background_pipeline_layout".into(),
            bind_group_layouts: &[Some(&vertex_layout), Some(Image::uniform_layout())],
            immediate_size:     0,
        });

        let render_pipeline = device.pipeline(
            "background_pipeline",
            &pipeline_layout,
            &shader,
            PolygonMode::Fill,
            PrimitiveTopology::TriangleStrip,
            &[Vertex2D::VERTEX_LAYOUT],
        );

        let view = BackgroundView {
            camera_pos:      Point::default(),
            resolution:      (1000, 1000).into(),
            camera_rotation: 0.0,
            scale:           1.0,
            _padding:        0,
        };

        let vertex_buffer = device.buffer(&VERTICES, BufferUsages::VERTEX);
        let view_buffer = device.buffer(&view, BufferUsages::UNIFORM | BufferUsages::COPY_DST);

        let view_bind = device.bind(&view_buffer, &vertex_layout);

        Self {
            render_pipeline,
            vertex_buffer,
            view_buffer,
            view_bind,
            view,
        }
    }
}

impl BackgroundPipeline {
    pub fn draw(
        &mut self,
        render_pass: &mut RenderPass,
        image: &Image,
        resolution: Size,
        camera_pos: Point,
        camera_rotation: f32,
        scale: f32,
    ) {
        render_pass.set_pipeline(&self.render_pipeline);

        let view = BackgroundView {
            camera_pos,
            resolution,
            camera_rotation,
            scale,
            _padding: 0,
        };

        if view != self.view {
            self.view = view;
            self.view_buffer.update(view);
        }

        render_pass.set_bind_group(0, &self.view_bind, &[]);
        render_pass.set_bind_group(1, image.bind(), &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(RANGE, 0..1);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        // Web requirements
        assert_eq!(size_of::<BackgroundView>() % 16, 0);
    }
}
