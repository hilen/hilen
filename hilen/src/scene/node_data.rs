use educe::Educe;

use crate::{
    deps::{refs::Weak, vents::Event},
    gm::volume::{Quat, Shape3, Vec3},
    scene::{Material, Mesh, Node},
};

#[derive(Educe)]
#[educe(Default)]
pub struct NodeData {
    pub(crate) position: Vec3,
    #[educe(Default = Quat::IDENTITY)]
    pub(crate) rotation: Quat,
    pub(crate) shape:    Shape3,

    pub(crate) collision_enabled: bool,

    pub tag: u32,

    pub material: Material,

    /// The unit mesh of a primitive shape. A model draws its own
    /// meshes, see `Shape3::Model`.
    pub mesh:         Option<Weak<Mesh>>,
    pub on_collision: Event<Weak<dyn Node>>,
    /// A touch that no view took landed on this node, the nearest one
    /// under the finger. Carries the point hit in the world.
    pub on_touch:     Event<Vec3>,
}

impl NodeData {
    pub(crate) fn make(shape: Shape3, position: Vec3) -> Self {
        Self {
            position,
            shape,
            mesh: Mesh::of_shape(shape),
            ..Default::default()
        }
    }
}
