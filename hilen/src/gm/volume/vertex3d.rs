use bytemuck::{Pod, Zeroable};

use crate::gm::{color::U8Color, flat::Point, volume::Vec3};

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
}

impl Default for Vertex3D {
    fn default() -> Self {
        Self::new(Vec3::ZERO, Vec3::ZERO, Point::new(0.0, 0.0))
    }
}
