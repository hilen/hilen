use bytemuck::{Pod, Zeroable};
use wgpu::{BufferAddress, VertexBufferLayout, VertexStepMode};

use crate::{
    gm::{
        color::Color,
        flat::{CornerRadii, Point, Size},
    },
    render::vertex_layout::VertexLayout,
};

/// The fields already sit at `std430` offsets. Only the tail needs padding,
/// because a struct holding a `vec4` has its size rounded up to a multiple of
/// 16, and the fragment stage reads these as a storage array. See
/// `UIRectInstance` for the full reasoning.
///
/// The ramp fields are filled by `Gradient::instance`, which turns the CSS
/// description into what the shader needs. They are in uv space, where the
/// box spans -0.5 to 0.5 on both axes.
#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod)]
pub(crate) struct UIGradientInstance {
    pub position:     Point,
    pub size:         Size,
    pub start_color:  Color,
    pub end_color:    Color,
    pub corner_radii: CornerRadii,
    pub z_position:   f32,
    pub scale:        f32,

    /// The gradient line direction divided by its length, with the box size
    /// folded in.
    pub linear_axis:   Point,
    pub radial_center: Point,
    pub radial_radii:  Size,
    /// The ramp value at the center of the box, half of a full length ramp.
    pub linear_bias:   f32,
    pub kind:          u32,

    /// The border rides on top of the ramp, exactly like `UIRectInstance`
    /// draws it on top of a flat fill.
    pub border_width: f32,

    /// A `vec4` needs 16 byte alignment, so `border_color` lands here rather
    /// than next to the width it belongs with.
    pub padding: [f32; 1],

    pub border_color: Color,
}

impl UIGradientInstance {
    /// `ui_gradient.wgsl` matches these against its `kind` field.
    pub const LINEAR: u32 = 0;
    pub const RADIAL: u32 = 1;
}

impl VertexLayout for UIGradientInstance {
    const ATTRIBS: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![2 => Float32x2, 3 => Float32x2, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32, 8 => Float32];
    const VERTEX_LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: size_of::<Self>() as BufferAddress,
        step_mode:    VertexStepMode::Instance,
        attributes:   Self::ATTRIBS,
    };
}

#[cfg(test)]
mod test {
    use std::mem::offset_of;

    use super::UIGradientInstance;

    /// `ui_gradient.wgsl` names these offsets in its `std430` storage struct.
    #[test]
    fn std430_layout() {
        assert_eq!(offset_of!(UIGradientInstance, position), 0);
        assert_eq!(offset_of!(UIGradientInstance, size), 8);
        assert_eq!(offset_of!(UIGradientInstance, start_color), 16);
        assert_eq!(offset_of!(UIGradientInstance, end_color), 32);
        assert_eq!(offset_of!(UIGradientInstance, corner_radii), 48);
        assert_eq!(offset_of!(UIGradientInstance, z_position), 64);
        assert_eq!(offset_of!(UIGradientInstance, scale), 68);
        assert_eq!(offset_of!(UIGradientInstance, linear_axis), 72);
        assert_eq!(offset_of!(UIGradientInstance, radial_center), 80);
        assert_eq!(offset_of!(UIGradientInstance, radial_radii), 88);
        assert_eq!(offset_of!(UIGradientInstance, linear_bias), 96);
        assert_eq!(offset_of!(UIGradientInstance, kind), 100);
        assert_eq!(offset_of!(UIGradientInstance, border_width), 104);
        assert_eq!(offset_of!(UIGradientInstance, border_color), 112);
        assert_eq!(size_of::<UIGradientInstance>(), 128);
    }
}
