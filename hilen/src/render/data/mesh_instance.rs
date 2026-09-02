use bytemuck::{Pod, Zeroable};
use wgpu::{BufferAddress, VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode};

use crate::{
    gm::{
        color::Color,
        volume::{Mat3, Mat4, Vec4},
    },
    render::vertex_layout::VertexLayout,
    scene::{LightPick, Material},
};

/// One drawn node. The vertex stage reads the matrices and `index` as
/// attributes, the fragment stage reads the material and the light list
/// through the storage binding at that index, so only the uv, the
/// normal and the index cross the stage boundary. Every row is 16
/// bytes, so the std430 layout the storage binding needs is the
/// `repr(C)` one.
#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod)]
pub(crate) struct MeshInstance {
    pub model:       Mat4,
    /// The inverse transpose of the model's rotation and scale, what a
    /// normal transforms by when the scale is not uniform. Three vec4
    /// rows, the fourth component is padding.
    pub normal:      [Vec4; 3],
    pub color:       Color,
    pub metallic:    f32,
    pub roughness:   f32,
    pub light_count: u32,
    /// Where this instance sits in the storage buffer of its draw. The
    /// vertex stage carries it instead of `instance_index`, so a
    /// translucent node drawn alone from the middle of the buffer needs
    /// no base instance, which an A7 cannot draw.
    pub index:       u32,
    /// Indices into the light buffer, two to a word, see `LightPick`.
    pub lights:      [u32; 4],
    /// The normal scale, the rest is padding.
    pub params:      [f32; 4],
}

impl MeshInstance {
    pub(crate) fn new(model: Mat4, material: Material, lights: LightPick) -> Self {
        let normal = Mat3::from_mat4(model).inverse().transpose();
        Self {
            model,
            normal: [
                normal.x_axis.extend(0.0),
                normal.y_axis.extend(0.0),
                normal.z_axis.extend(0.0),
            ],
            color: material.color,
            metallic: material.metallic,
            roughness: material.roughness,
            light_count: lights.count,
            index: 0,
            lights: lights.packed,
            params: [material.normal_scale, 0.0, 0.0, 0.0],
        }
    }
}

const fn row(location: u32, offset: u64) -> VertexAttribute {
    VertexAttribute {
        format: VertexFormat::Float32x4,
        offset,
        shader_location: location,
    }
}

impl VertexLayout for MeshInstance {
    const ATTRIBS: &'static [VertexAttribute] = &[
        row(3, 0),
        row(4, 16),
        row(5, 32),
        row(6, 48),
        row(7, 64),
        row(8, 80),
        row(9, 96),
        VertexAttribute {
            format:          VertexFormat::Uint32,
            offset:          140,
            shader_location: 10,
        },
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
    // offsets are its std430 layout, and the vertex attributes above
    // name the same offsets by hand.
    #[test]
    fn layout_matches_the_shader() {
        assert_eq!(offset_of!(MeshInstance, model), 0);
        assert_eq!(offset_of!(MeshInstance, normal), 64);
        assert_eq!(offset_of!(MeshInstance, color), 112);
        assert_eq!(offset_of!(MeshInstance, metallic), 128);
        assert_eq!(offset_of!(MeshInstance, roughness), 132);
        assert_eq!(offset_of!(MeshInstance, light_count), 136);
        assert_eq!(offset_of!(MeshInstance, index), 140);
        assert_eq!(offset_of!(MeshInstance, lights), 144);
        assert_eq!(offset_of!(MeshInstance, params), 160);
        assert_eq!(size_of::<MeshInstance>(), 176);
        assert_eq!(MeshInstance::ATTRIBS[7].offset, 140);
    }
}
