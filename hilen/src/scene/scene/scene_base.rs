use std::ops::Deref;

use educe::Educe;

use crate::{
    deps::refs::{Own, Weak},
    gm::volume::Vec3,
    scene::{Camera, Node, Scene, scene::scene_physics::ScenePhysics},
};

#[derive(Educe)]
#[educe(Default)]
pub struct SceneBase {
    pub(crate) nodes: Vec<Own<dyn Node>>,

    pub camera: Camera,

    pub(crate) physics: Option<ScenePhysics>,
}

impl SceneBase {
    /// Physics steps per frame, the same reasoning as `LevelBase`.
    pub const PHYSICS_SUBSTEPS: usize = 4;

    pub fn has_physics(&self) -> bool {
        self.physics.is_some()
    }

    pub fn init_physics(&mut self) {
        assert!(self.physics.is_none(), "Double init_physics");
        self.physics = ScenePhysics::default().into();
    }

    pub fn update_physics(&mut self, frame_time: f32) {
        if let Some(physics) = self.physics.as_mut() {
            physics.update_physics(&self.nodes, frame_time);
        }
    }

    pub fn remove(&mut self, node: Weak<dyn Node>) {
        let index = self.nodes.iter().position(|a| a.raw() == node.raw()).unwrap();

        let node = self.nodes[index].deref();

        if let Some(physics) = self.physics.as_mut() {
            physics.remove(node);
        }
        self.nodes.remove(index);
    }

    pub fn remove_all_nodes(&mut self) {
        if let Some(physics) = &mut self.physics {
            for node in self.nodes.drain(..) {
                physics.remove(node.deref());
            }
        } else {
            self.nodes.clear();
        }
    }
}

pub trait SceneTemplates {
    fn set_gravity(&mut self, g: impl Into<Vec3>);
}

impl<T: ?Sized + Scene> SceneTemplates for T {
    fn set_gravity(&mut self, g: impl Into<Vec3>) {
        if let Some(physics) = self.physics.as_mut() {
            physics.gravity = g.into();
        }
    }
}
