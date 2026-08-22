use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, Zeroable, Pod)]
pub struct Point3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Point3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}
