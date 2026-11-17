use bytemuck::{Pod, Zeroable};
use wgpu::{BufferAddress, VertexBufferLayout, VertexStepMode};

use crate::{
    gm::{
        color::{Color, WHITE},
        flat::{Point, Size},
    },
    render::vertex_layout::VertexLayout,
};

const FLIP_X: u32 = 1;
const FLIP_Y: u32 = 1 << 1;

#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod)]
pub(crate) struct TexturedSpriteInstance {
    pub tint:       Color,
    pub position:   Point,
    pub size:       Size,
    pub scale:      f32,
    pub rotation:   f32,
    pub z_position: f32,
    flags:          u32,
}

impl TexturedSpriteInstance {
    pub(crate) fn new(position: Point, size: Size, rotation: f32, z_position: f32) -> Self {
        Self {
            tint: WHITE,
            position,
            size,
            scale: 1.0,
            rotation,
            z_position,
            flags: 0,
        }
    }

    pub(crate) fn flipped(mut self, flip_x: bool, flip_y: bool) -> Self {
        self.flags = if flip_x { FLIP_X } else { 0 } | if flip_y { FLIP_Y } else { 0 };
        self
    }
}

impl VertexLayout for TexturedSpriteInstance {
    const ATTRIBS: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
        2 => Float32x4,
        3 => Float32x2,
        4 => Float32x2,
        5 => Float32,
        6 => Float32,
        7 => Float32,
        8 => Uint32
    ];
    const VERTEX_LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: size_of::<Self>() as BufferAddress,
        step_mode:    VertexStepMode::Instance,
        attributes:   Self::ATTRIBS,
    };
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn layout() {
        assert_eq!(size_of::<TexturedSpriteInstance>(), 48);
    }
}
