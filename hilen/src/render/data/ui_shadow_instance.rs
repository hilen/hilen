use bytemuck::{Pod, Zeroable};
use wgpu::{BufferAddress, VertexBufferLayout, VertexStepMode};

use crate::{
    gm::{
        color::Color,
        flat::{CornerRadii, Point, Size},
    },
    render::vertex_layout::VertexLayout,
};

/// A drop shadow under a rounded rect. `position` and `size` are the
/// casting rect, the quad is expanded by `blur` in the shader.
///
/// The fields already sit at `std430` offsets. Only the tail needs padding,
/// because a struct holding a `vec4` has its size rounded up to a multiple of
/// 16, and the fragment stage reads these as a storage array. See
/// `UIRectInstance` for the full reasoning.
#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod)]
pub(crate) struct UIShadowInstance {
    pub position:     Point,
    pub size:         Size,
    pub color:        Color,
    pub corner_radii: CornerRadii,
    pub blur:         f32,
    pub z_position:   f32,
    pub scale:        f32,
    pub padding:      f32,
}

impl VertexLayout for UIShadowInstance {
    const ATTRIBS: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![2 => Float32x2, 3 => Float32x2, 4 => Float32x4, 5 => Float32x4, 6 => Float32, 7 => Float32, 8 => Float32];
    const VERTEX_LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: size_of::<Self>() as BufferAddress,
        step_mode:    VertexStepMode::Instance,
        attributes:   Self::ATTRIBS,
    };
}

#[cfg(test)]
mod test {
    use std::mem::offset_of;

    use super::UIShadowInstance;

    /// `ui_shadow.wgsl` names these offsets in its `std430` storage struct.
    #[test]
    fn std430_layout() {
        assert_eq!(offset_of!(UIShadowInstance, position), 0);
        assert_eq!(offset_of!(UIShadowInstance, size), 8);
        assert_eq!(offset_of!(UIShadowInstance, color), 16);
        assert_eq!(offset_of!(UIShadowInstance, corner_radii), 32);
        assert_eq!(offset_of!(UIShadowInstance, blur), 48);
        assert_eq!(offset_of!(UIShadowInstance, z_position), 52);
        assert_eq!(offset_of!(UIShadowInstance, scale), 56);
        assert_eq!(size_of::<UIShadowInstance>(), 64);
    }
}
