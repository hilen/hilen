use std::{marker::ConstParamTy, ops::Range};

#[cfg(wasm)]
use wgpu::{BlendComponent, BlendFactor, BlendOperation};
use wgpu::{BlendState, Device};

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

/// Neither the fragment nor what is under it counts, the pixel ends as
/// zeros.
#[cfg(wasm)]
const ERASE: BlendComponent = BlendComponent {
    src_factor: BlendFactor::Zero,
    dst_factor: BlendFactor::Zero,
    operation:  BlendOperation::Add,
};

const VERTEX_RANGE: Range<u32> = 0..checked_usize_to_u32(VERTICES.len());

const TEXTURED_VERTEX_RANGE: Range<u32> = 0..checked_usize_to_u32(TEXTURED_VERTICES.len());

#[derive(ConstParamTy, PartialEq, Eq)]
pub enum PipelineType {
    Color,
    Image,
    /// A color rect that writes nothing but zeros, color and alpha. A
    /// browser shows the page behind the canvas there, see `ui_drawer.rs`.
    #[cfg(wasm)]
    Hole,
}

impl PipelineType {
    pub(crate) const fn color(&self) -> bool {
        !self.image()
    }

    /// How a fragment lands on what is drawn already.
    pub(crate) const fn blend(&self) -> BlendState {
        match self {
            Self::Color | Self::Image => BlendState::ALPHA_BLENDING,
            #[cfg(wasm)]
            Self::Hole => BlendState {
                color: ERASE,
                alpha: ERASE,
            },
        }
    }

    pub(crate) const fn image(&self) -> bool {
        matches!(self, Self::Image)
    }

    pub(crate) const fn vertex_range(&self) -> Range<u32> {
        if self.image() {
            TEXTURED_VERTEX_RANGE
        } else {
            VERTEX_RANGE
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
