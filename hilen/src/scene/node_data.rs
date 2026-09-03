use educe::Educe;

use crate::{
    deps::{refs::Weak, vents::Event},
    gm::volume::{Quat, Shape3, Vec3},
    scene::{Material, Mesh, Node, Playback},
};

#[derive(Educe)]
#[educe(Default)]
pub struct NodeData {
    pub(crate) position: Vec3,
    #[educe(Default = Quat::IDENTITY)]
    pub(crate) rotation: Quat,
    /// Uniform, on top of the shape's own size, see `NodeTemplates::set_scale`.
    #[educe(Default = 1.0)]
    pub(crate) scale:    f32,
    pub(crate) shape:    Shape3,

    pub(crate) collision_enabled: bool,

    /// The clip of the model playing on this node, see
    /// `NodeTemplates::play`. Without one a model draws at rest.
    pub(crate) playback: Option<Playback>,

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

    /// Moves the playing clip on by `dt` seconds of scene time.
    pub(crate) fn advance_animation(&mut self, dt: f32) {
        let Shape3::Model(model) = self.shape else {
            return;
        };
        if let (Some(playback), true) = (self.playback.as_mut(), model.is_ok()) {
            playback.advance(dt, model.clips()[playback.clip].duration);
        }
    }
}
