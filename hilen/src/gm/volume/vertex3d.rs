use bytemuck::{Pod, Zeroable};

use crate::gm::{LossyConvert, color::U8Color, flat::Point, volume::Vec3};

/// The fastest flow of `flow_texel`, the same number in `mesh.wgsl`.
pub const FLOW_RANGE: f32 = 0.5;

/// A texel of `Material::flow_map` for a flow in uv units a second, up to
/// `FLOW_RANGE` each way.
pub fn flow_texel(flow: Point) -> [u8; 4] {
    water_texel(flow, 0.0, 1.0)
}

/// `flow_texel` with a look: `foam` from 0 for none to 1 for white water, and
/// `depth` from 0 for the color of `Material::shallow_color` to 1 for the color
/// of the material. Blue holds the depth and alpha the opposite of the foam, so
/// the flat texel a mesh with no flow map reads is deep water with no foam.
pub fn water_texel(flow: Point, foam: f32, depth: f32) -> [u8; 4] {
    let byte = |speed: f32| -> u8 {
        (((speed / FLOW_RANGE).clamp(-1.0, 1.0) * 0.5 + 0.5) * 255.0)
            .round()
            .lossy_convert()
    };
    let share = |value: f32| -> u8 { (value.clamp(0.0, 1.0) * 255.0).round().lossy_convert() };
    [byte(flow.x), byte(flow.y), share(depth), share(1.0 - foam)]
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct Vertex3D {
    pub pos:    Vec3,
    pub normal: Vec3,
    pub uv:     Point,
    /// Multiplies the node's material color, encoded sRGB like every
    /// color. White leaves the material as it is. A triangle takes one
    /// color, the shader passes it flat from one of its corners, since an
    /// A7 has no room left for another blended value, see docs/ios.md.
    /// `Model::from_smooth_mesh` fades it across the triangle instead.
    pub color:  U8Color,
}

impl Vertex3D {
    /// A white vertex, the material's color as it is.
    pub const fn new(pos: Vec3, normal: Vec3, uv: Point) -> Self {
        Self {
            pos,
            normal,
            uv,
            color: U8Color::const_rgb(255, 255, 255),
        }
    }

    /// A vertex of a mesh from `Model::from_flowing_mesh`, with `map` its
    /// place in the node's `Material::flow_map`, from 0 to 1. The place takes
    /// the room of the color. `uv` and `map` have to grow in step over the
    /// mesh, 1 number of each to the other, the shader reads the ratio.
    pub fn flowing(pos: Vec3, normal: Vec3, uv: Point, map: Point) -> Self {
        let bytes = |at: f32| {
            let number: u32 = (at.clamp(0.0, 1.0) * 65535.0).round().lossy_convert();
            u16::try_from(number).expect("a place from 0 to 1 fits 16 bits").to_le_bytes()
        };
        let ([r, g], [b, a]) = (bytes(map.x), bytes(map.y));
        Self {
            pos,
            normal,
            uv,
            color: U8Color { r, g, b, a },
        }
    }
}

impl Default for Vertex3D {
    fn default() -> Self {
        Self::new(Vec3::ZERO, Vec3::ZERO, Point::new(0.0, 0.0))
    }
}
