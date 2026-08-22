use std::{marker::ConstParamTy, ops::Range};

use wgpu::Device;

use crate::{
    gm::{
        checked_usize_to_u32,
        flat::{Point, Vertex2D},
    },
    render::device_helper::DeviceHelper,
    window::BufferUsages,
};

const VERTICES: &[Point] = &[
    Point::new(-1.0, 1.0),
    Point::new(-1.0, -1.0),
    Point::new(1.0, 1.0),
    Point::new(1.0, -1.0),
];

const TEXTURED_VERTICES: &[Vertex2D; 4] = &[
    Vertex2D {
        pos: Point::new(-1.0, 1.0),
        uv:  Point::new(0.0, 0.0),
    },
    Vertex2D {
        pos: Point::new(-1.0, -1.0),
        uv:  Point::new(0.0, 1.0),
    },
    Vertex2D {
        pos: Point::new(1.0, 1.0),
        uv:  Point::new(1.0, 0.0),
    },
    Vertex2D {
        pos: Point::new(1.0, -1.0),
        uv:  Point::new(1.0, 1.0),
    },
];

const VERTEX_RANGE: Range<u32> = 0..checked_usize_to_u32(VERTICES.len());

const TEXTURED_VERTEX_RANGE: Range<u32> = 0..checked_usize_to_u32(TEXTURED_VERTICES.len());

#[derive(ConstParamTy, PartialEq, Eq)]
pub enum PipelineType {
    Color,
    Image,
}

impl PipelineType {
    pub(crate) const fn color(&self) -> bool {
        matches!(self, PipelineType::Color)
    }

    pub(crate) const fn image(&self) -> bool {
        matches!(self, Self::Image)
    }

    pub(crate) const fn vertex_range(&self) -> Range<u32> {
        match self {
            Self::Color => VERTEX_RANGE,
            Self::Image => TEXTURED_VERTEX_RANGE,
        }
    }

    pub(crate) fn vertex_buffer(&self, device: &Device) -> wgpu::Buffer {
        if self.image() {
            device.buffer(TEXTURED_VERTICES, BufferUsages::VERTEX)
        } else {
            device.buffer(VERTICES, BufferUsages::VERTEX)
        }
    }
}
