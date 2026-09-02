use bytemuck::{Pod, Zeroable};
use wgpu::{BufferAddress, VertexBufferLayout, VertexStepMode};

use crate::{
    gm::{
        color::Color,
        volume::{Mat3, Mat4, Vec4},
    },
    render::vertex_layout::VertexLayout,
};

/// One drawn node. The vertex stage reads the matrices as attributes,
/// the fragment stage reads the color through the storage binding by
/// instance index, so only the normal and the index cross the stage
/// boundary. Every field is a vec4 row, so the std430 layout the
/// storage binding needs is the `repr(C)` one.
#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod)]
pub(crate) struct MeshInstance {
    pub model:  Mat4,
    /// The inverse transpose of the model's rotation and scale, what a
    /// normal transforms by when the scale is not uniform. Three vec4
    /// rows, the fourth component is padding.
    pub normal: [Vec4; 3],
    pub color:  Color,
}

impl MeshInstance {
    pub(crate) fn new(model: Mat4, color: Color) -> Self {
        let normal = Mat3::from_mat4(model).inverse().transpose();
        Self {
            model,
            normal: [
                normal.x_axis.extend(0.0),
                normal.y_axis.extend(0.0),
                normal.z_axis.extend(0.0),
            ],
            color,
        }
    }
}

impl VertexLayout for MeshInstance {
    const ATTRIBS: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
        3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4,
        7 => Float32x4, 8 => Float32x4, 9 => Float32x4,
    ];
    const VERTEX_LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: size_of::<Self>() as BufferAddress,
        step_mode:    VertexStepMode::Instance,
        attributes:   Self::ATTRIBS,
    };
}

#[cfg(test)]
mod test {
    use std::mem::offset_of;

    use super::*;

    // The shader reads this struct through a storage buffer, so the
    // offsets are its std430 layout.
    #[test]
    fn layout_matches_the_shader() {
        assert_eq!(offset_of!(MeshInstance, model), 0);
        assert_eq!(offset_of!(MeshInstance, normal), 64);
        assert_eq!(offset_of!(MeshInstance, color), 112);
        assert_eq!(size_of::<MeshInstance>(), 128);
    }
}
