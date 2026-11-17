use bytemuck::{Pod, Zeroable};

use crate::gm::{color::Color, volume::Vec3};

/// One line of a scene drawn as a quad `width` pixels wide, see
/// `wide_lines.wgsl`. One instance per line, plain arrays so the struct
/// has no padding and stays `Pod`.
#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod)]
pub(crate) struct WideLine {
    pub from:  [f32; 3],
    pub width: f32,
    pub to:    [f32; 3],
    pub pad:   f32,
    pub color: [f32; 4],
}

impl WideLine {
    pub(crate) fn new(from: Vec3, to: Vec3, color: Color, width: f32) -> Self {
        Self {
            from: from.to_array(),
            width,
            to: to.to_array(),
            pad: 0.0,
            color: [color.r, color.g, color.b, color.a],
        }
    }
}
