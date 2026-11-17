use bytemuck::{Pod, Zeroable};

use crate::gm::{color::Color, volume::Vec3};

/// One end of a debug line, see `lines.wgsl`. Plain arrays, a SIMD
/// `Vec4` would pad the struct and break `Pod`.
#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod)]
pub(crate) struct LineVertex {
    pub position: [f32; 3],
    pub color:    [f32; 4],
}

impl LineVertex {
    pub(crate) fn new(position: Vec3, color: Color) -> Self {
        Self {
            position: position.to_array(),
            color:    [color.r, color.g, color.b, color.a],
        }
    }
}
