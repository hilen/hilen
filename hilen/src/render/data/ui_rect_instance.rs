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
/// first and the tail is padded out to a multiple of 16. Rust packs a `repr(C)`
/// struct with 4 byte alignment and would otherwise land `corner_radii` at 52,
/// which no `std430` layout can name.
#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod)]
pub struct UIRectInstance {
    pub color:        Color,
    pub border_color: Color,
    pub corner_radii: CornerRadii,
    pub position:     Point,
    pub size:         Size,
    pub border_width: f32,
    pub z_position:   f32,
    pub scale:        f32,
    pub padding:      f32,
}

impl UIRectInstance {
    pub fn new(
        rect: Rect,
        color: Color,
        border_color: Color,
        border_width: f32,
        corner_radii: CornerRadii,
        z_position: f32,
        scale: f32,
    ) -> Self {
        Self {
            color,
            border_color,
            corner_radii,
            position: rect.origin,
            size: rect.size,
            border_width,
            z_position,
            scale,
            padding: 0.0,
        }
    }
}

impl VertexLayout for UIRectInstance {
    const ATTRIBS: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x2, 6 => Float32x2, 7 => Float32, 8 => Float32, 9 => Float32];
    const VERTEX_LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: size_of::<Self>() as BufferAddress,
        step_mode:    VertexStepMode::Instance,
        attributes:   Self::ATTRIBS,
    };
}

#[cfg(test)]
mod test {
    use std::mem::offset_of;

    use super::UIRectInstance;

    /// `ui_rect.wgsl` names these offsets in its `std430` storage struct. A
    /// reordered field would still compile and would feed the shader whatever
    /// happened to land at the offset it expected.
    #[test]
    fn std430_layout() {
        assert_eq!(offset_of!(UIRectInstance, color), 0);
        assert_eq!(offset_of!(UIRectInstance, border_color), 16);
        assert_eq!(offset_of!(UIRectInstance, corner_radii), 32);
        assert_eq!(offset_of!(UIRectInstance, position), 48);
        assert_eq!(offset_of!(UIRectInstance, size), 56);
        assert_eq!(offset_of!(UIRectInstance, border_width), 64);
        assert_eq!(offset_of!(UIRectInstance, z_position), 68);
        assert_eq!(offset_of!(UIRectInstance, scale), 72);
        assert_eq!(size_of::<UIRectInstance>(), 80);
    }
}
