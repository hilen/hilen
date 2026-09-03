use std::mem::size_of;

use wgpu::{BufferAddress, VertexAttribute, VertexBufferLayout, VertexStepMode};

use crate::gm::flat::{Point, Vertex2D};
#[cfg(feature = "scene")]
use crate::{
    gm::volume::{SkinVertex, Vertex3D},
    render::data::LineVertex,
};

pub(crate) trait VertexLayout: Sized {
    const ATTRIBS: &'static [VertexAttribute];
    const VERTEX_LAYOUT: VertexBufferLayout<'static>;
}

impl VertexLayout for Point {
    const ATTRIBS: &'static [VertexAttribute] = &wgpu::vertex_attr_array![0 => Float32x2];
    const VERTEX_LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: size_of::<Self>() as BufferAddress,
        step_mode:    VertexStepMode::Vertex,
        attributes:   Self::ATTRIBS,
    };
}

impl VertexLayout for Vertex2D {
    const ATTRIBS: &'static [VertexAttribute] = &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2];
    const VERTEX_LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: size_of::<Self>() as BufferAddress,
        step_mode:    VertexStepMode::Vertex,
        attributes:   Self::ATTRIBS,
    };
}

#[cfg(feature = "scene")]
impl VertexLayout for Vertex3D {
    const ATTRIBS: &'static [VertexAttribute] =
        &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2];
    const VERTEX_LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: size_of::<Self>() as BufferAddress,
        step_mode:    VertexStepMode::Vertex,
        attributes:   Self::ATTRIBS,
    };
}

/// The second buffer of a skinned mesh, locations past the instance.
#[cfg(feature = "scene")]
impl VertexLayout for SkinVertex {
    const ATTRIBS: &'static [VertexAttribute] = &wgpu::vertex_attr_array![12 => Uint16x4, 13 => Float32x4];
    const VERTEX_LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: size_of::<Self>() as BufferAddress,
        step_mode:    VertexStepMode::Vertex,
        attributes:   Self::ATTRIBS,
    };
}

#[cfg(feature = "scene")]
impl VertexLayout for LineVertex {
    const ATTRIBS: &'static [VertexAttribute] = &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4];
    const VERTEX_LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: size_of::<Self>() as BufferAddress,
        step_mode:    VertexStepMode::Vertex,
        attributes:   Self::ATTRIBS,
    };
}
