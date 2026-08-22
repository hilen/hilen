use bytemuck::{Pod, Zeroable};
use wgpu::{BufferAddress, VertexBufferLayout, VertexStepMode};

use crate::{
    gm::{
        color::Color,
        flat::{Point, Size},
    },
    render::vertex_layout::VertexLayout,
};

#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod)]
pub(crate) struct SpriteInstance {
    pub position:   Point,
    pub size:       Size,
    pub color:      Color,
    pub rotation:   f32,
    pub z_position: f32,
}

impl SpriteInstance {}

impl VertexLayout for SpriteInstance {
    const ATTRIBS: &'static [wgpu::VertexAttribute] =
        &wgpu::vertex_attr_array![2 => Float32x2, 3 => Float32x2, 4 => Float32x4, 5 => Float32, 6 => Float32];
    const VERTEX_LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: size_of::<Self>() as BufferAddress,
        step_mode:    VertexStepMode::Instance,
        attributes:   Self::ATTRIBS,
    };
}
