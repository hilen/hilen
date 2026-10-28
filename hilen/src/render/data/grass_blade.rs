use bytemuck::{Pod, Zeroable};
use wgpu::{BufferAddress, VertexBufferLayout, VertexStepMode, vertex_attr_array};

use crate::{gm::color::U8Color, render::vertex_layout::VertexLayout};

/// One blade of a grass patch, an instance of the blade mesh, see
/// `scene::Grass`. A patch keeps its blades in 1 buffer from when it is
/// grown until it goes, the frame only sends 1 `MeshInstance` and 1
/// matrix per patch, see `v_grass` in mesh.wgsl.
#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod, PartialEq)]
pub(crate) struct GrassBlade {
    /// The root from the patch's origin, and the turn about up in radians.
    pub place:  [f32; 4],
    /// The scale of the blade mesh.
    pub size:   f32,
    /// Multiplies the vertex color, encoded sRGB.
    pub tint:   U8Color,
    /// The patch's slot in the frame's grass buffers. An A7 cannot draw
    /// from a base instance, so each blade names its patch itself.
    pub slot:   u32,
    /// The x and z of the ground's normal as 2 snorm16, x in the low half,
    /// what the blade stands along. Its y is the rest of a unit length.
    pub normal: u32,
}

impl GrassBlade {
    pub(crate) fn pack_normal(normal: crate::gm::volume::Vec3) -> u32 {
        let snorm = |value: f32| {
            let scaled: i32 =
                crate::gm::LossyConvert::lossy_convert((value.clamp(-1.0, 1.0) * 32767.0).round());
            u32::from(i16::try_from(scaled).expect("a snorm16 fits i16").cast_unsigned())
        };
        let normal = normal.normalize_or(crate::gm::volume::Vec3::Y);
        snorm(normal.x) | snorm(normal.z) << 16
    }
}

impl VertexLayout for GrassBlade {
    const ATTRIBS: &'static [wgpu::VertexAttribute] =
        &vertex_attr_array![3 => Float32x4, 4 => Float32, 5 => Uint32, 6 => Uint32, 7 => Uint32];
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

    #[test]
    fn layout_matches_the_attributes() {
        assert_eq!(offset_of!(GrassBlade, place), 0);
        assert_eq!(offset_of!(GrassBlade, size), 16);
        assert_eq!(offset_of!(GrassBlade, tint), 20);
        assert_eq!(offset_of!(GrassBlade, slot), 24);
        assert_eq!(size_of::<GrassBlade>(), 32);
        assert_eq!(GrassBlade::ATTRIBS[2].offset, 20);
        assert_eq!(GrassBlade::ATTRIBS[3].offset, 24);
        assert_eq!(GrassBlade::ATTRIBS[4].offset, 28);
    }

    #[test]
    fn a_normal_packs_into_2_snorms() {
        use crate::gm::volume::Vec3;
        assert_eq!(GrassBlade::pack_normal(Vec3::Y), 0);
        let [x0, x1, z0, z1] = GrassBlade::pack_normal(Vec3::new(1.0, 1.0, -1.0)).to_le_bytes();
        let x = f32::from(i16::from_le_bytes([x0, x1])) / 32767.0;
        let z = f32::from(i16::from_le_bytes([z0, z1])) / 32767.0;
        let expected = 1.0 / 3f32.sqrt();
        assert!(
            (x - expected).abs() < 1e-4 && (z + expected).abs() < 1e-4,
            "{x} {z}"
        );
    }
}
