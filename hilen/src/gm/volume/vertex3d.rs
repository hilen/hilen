use bytemuck::{Pod, Zeroable};

use crate::gm::{flat::Point, volume::Vec3};

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Zeroable, Pod)]
pub struct Vertex3D {
    pub pos:    Vec3,
    pub normal: Vec3,
    pub uv:     Point,
}
