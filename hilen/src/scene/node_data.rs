use educe::Educe;

use crate::{
    deps::{refs::Weak, vents::Event},
    gm::volume::{Quat, Shape3, Vec3},
    scene::{ColliderShape, Material, Mesh, Node, Playback},
};

#[derive(Educe)]
#[educe(Default)]
pub struct NodeData {
    pub(crate) position:       Vec3,
    #[educe(Default = Quat::IDENTITY)]
    pub(crate) rotation:       Quat,
    /// Uniform, on top of the shape's own size, see `NodeTemplates::set_scale`.
    #[educe(Default = 1.0)]
    pub(crate) scale:          f32,
    pub(crate) shape:          Shape3,
    /// A collider put on in place of the shape's own, with where it sits
    /// from the origin, see `NodeTemplates::set_collider`.
    pub(crate) collider:       Option<(ColliderShape, Vec3)>,
    /// The node this one hangs on, null for none. While it lives the
    /// position and rotation are the parent's local ones, see
    /// `NodeTemplates::attach_to`.
    pub(crate) parent:         Weak<dyn Node>,
    /// The world scale the rapier collider was last built at, so a parent
    /// that grows is noticed on the next step.
    #[educe(Default = 1.0)]
    pub(crate) collider_scale: f32,

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

    /// The node's own uniform size, see `NodeTemplates::set_scale`.
    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// The node this one is attached to.
    pub fn parent(&self) -> Option<Weak<dyn Node>> {
        self.parent.is_ok().then_some(self.parent)
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
