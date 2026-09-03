use bytemuck::{Pod, Zeroable};

use crate::gm::volume::Vec4;

/// One point or spot light as the mesh shader reads it. `position.w` is
/// one over the range squared, `direction.w` and `color.w` the scale
/// and offset of the cone term, and `color.rgb` linear light times the
/// intensity. See `Light::mesh_light`.
#[repr(C)]
#[derive(Debug, Copy, Clone, Default, PartialEq, Zeroable, Pod)]
pub(crate) struct MeshLight {
    pub position:  Vec4,
    pub direction: Vec4,
    pub color:     Vec4,
}

#[cfg(test)]
mod test {
    use std::mem::offset_of;

    use super::*;

    // Read through a storage buffer, so the offsets are its std430 layout.
    #[test]
    fn layout_matches_the_shader() {
        assert_eq!(offset_of!(MeshLight, position), 0);
        assert_eq!(offset_of!(MeshLight, direction), 16);
        assert_eq!(offset_of!(MeshLight, color), 32);
        assert_eq!(size_of::<MeshLight>(), 48);
    }
}
