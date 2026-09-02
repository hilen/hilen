use educe::Educe;

use crate::{
    deps::{refs::Weak, vents::Event},
    gm::{
        color::Color,
        volume::{Quat, Shape3, Vec3},
    },
    scene::{Mesh, Node},
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

    #[educe(Default = Color::random())]
    pub color: Color,

    pub mesh:         Weak<Mesh>,
    pub on_collision: Event<Weak<dyn Node>>,
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
