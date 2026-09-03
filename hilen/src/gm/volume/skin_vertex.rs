use bytemuck::{Pod, Zeroable};

/// What a skinned vertex adds to its `Vertex3D`: the four joints that
/// move it and how much of it each one moves, summing to one. A second
/// vertex buffer next to the geometry, so a static mesh carries none of
/// it.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct SkinVertex {
    /// Indices into the joints of the skin, not into the model's nodes.
    pub joints:  [u16; 4],
    pub weights: [f32; 4],
}
