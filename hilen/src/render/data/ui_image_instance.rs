use bit_ops::BitOps;
use bytemuck::{Pod, Zeroable};
use wgpu::{BufferAddress, VertexBufferLayout, VertexStepMode};

use crate::{
    gm::{
        color::Color,
        flat::{CornerRadii, Point, Rect, Size},
    },
    render::vertex_layout::VertexLayout,
};

/// Field order is not free. The fragment stage reads this through a storage
/// buffer, where a `vec4` must start at a multiple of 16, so every `vec4` comes
/// first. See `UIRectInstance` for the same reasoning and a layout test.
#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod)]
pub struct UIImageInstance {
    pub border_color: Color,
    pub corner_radii: CornerRadii,
    pub position:     Point,
    pub size:         Size,
    pub uv_position:  Point,
    pub uv_size:      Size,
    pub border_width: f32,
    pub z_position:   f32,
    pub flags:        u32,
    pub scale:        f32,
}

impl UIImageInstance {
    const FLIP_X_FLAG: u32 = 0;
    const FLIP_Y_FLAG: u32 = 1;

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        rect: Rect,
        uv: Rect,
        border_color: Color,
        border_width: f32,
        corner_radii: CornerRadii,
        z_position: f32,
        flip_x: bool,
        flip_y: bool,
        scale: f32,
    ) -> Self {
        let mut result = Self {
            position: rect.origin,
            size: rect.size,
            corner_radii,
            z_position,
            flags: 0,
            scale,
            border_color,
            border_width,
            uv_position: uv.origin,
            uv_size: uv.size,
        };

        result.set_flip_x(flip_x);
        result.set_flip_y(flip_y);

        result
    }

    fn set_flag(&mut self, bit: u32, value: bool) {
        if value {
            self.flags = self.flags.set_bit(bit);
        } else {
            self.flags = self.flags.clear_bit(bit);
        }
    }

    fn set_flip_x(&mut self, flip_x: bool) {
        self.set_flag(Self::FLIP_X_FLAG, flip_x);
    }

    fn set_flip_y(&mut self, flip_x: bool) {
        self.set_flag(Self::FLIP_Y_FLAG, flip_x);
    }
}

impl VertexLayout for UIImageInstance {
    const ATTRIBS: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![2 => Float32x4, 3 => Float32x4, 4 => Float32x2, 5 => Float32x2, 6 => Float32x2, 7 => Float32x2, 8 => Float32, 9 => Float32, 10 => Uint32, 11 => Float32];
    const VERTEX_LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: size_of::<Self>() as BufferAddress,
        step_mode:    VertexStepMode::Instance,
        attributes:   Self::ATTRIBS,
    };
}

#[cfg(test)]
mod test {
    use std::mem::offset_of;

    use super::UIImageInstance;

    /// `ui_image.wgsl` names these offsets in its `std430` storage struct. A
    /// reordered field would still compile and would feed the shader whatever
    /// happened to land at the offset it expected.
    #[test]
    fn std430_layout() {
        assert_eq!(offset_of!(UIImageInstance, border_color), 0);
        assert_eq!(offset_of!(UIImageInstance, corner_radii), 16);
        assert_eq!(offset_of!(UIImageInstance, position), 32);
        assert_eq!(offset_of!(UIImageInstance, size), 40);
        assert_eq!(offset_of!(UIImageInstance, uv_position), 48);
        assert_eq!(offset_of!(UIImageInstance, uv_size), 56);
        assert_eq!(offset_of!(UIImageInstance, border_width), 64);
        assert_eq!(offset_of!(UIImageInstance, z_position), 68);
        assert_eq!(offset_of!(UIImageInstance, flags), 72);
        assert_eq!(offset_of!(UIImageInstance, scale), 76);
        assert_eq!(size_of::<UIImageInstance>(), 80);
    }
}
