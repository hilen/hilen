use std::ops::Deref;

use educe::Educe;

use crate::{
    deps::{
        refs::{Own, Weak},
        vents::Event,
    },
    gm::{
        color::Color,
        flat::Point,
        volume::{Ray, Vec3},
    },
    scene::{Camera, Fog, Light, Node, Player, Scene, Sky, Sun, scene::scene_physics::ScenePhysics},
    ui::UIManager,
};

#[derive(Educe)]
#[educe(Default)]
pub struct SceneBase {
    pub(crate) nodes: Vec<Own<dyn Node>>,

    pub camera: Camera,

    pub sun:     Sun,
    /// What lights every node from all directions at once, the stand in
    /// for a sky until one exists. Encoded sRGB like every color, the
    /// default is a quarter of full light.
    #[educe(Default = Color::hex("#898989"))]
    pub ambient: Color,
    /// The point and spot lights. A node is drawn with the nearest
    /// eight of those in reach of it.
    pub lights:  Vec<Light>,
    /// Drawn behind everything and reflected by every surface. With a
    /// sky the flat `ambient` is not used, the sky lights the scene.
    pub sky:     Option<Sky>,
    /// Distance fog over every surface, none by default.
    pub fog:     Option<Fog>,
    /// The first person player, see `add_player`. While one exists the
    /// camera follows its eyes.
    pub player:  Option<Player>,

    /// A touch that no view took, as the ray it makes from the camera.
    /// The nearest node under it gets its own `on_touch` first.
    pub on_tap: Event<Ray>,

    /// Draws every collider as a green wireframe over the scene, the
    /// box around a model's bounds, three rings on a ball. A plane's
    /// slab is the floor itself and is not drawn. Off by default.
    pub show_colliders: bool,

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

    /// One step of the scene's time: the playing clips move on, then
    /// the physics, when the scene has any.
    pub fn update_physics(&mut self, frame_time: f32) {
        for node in &mut self.nodes {
            node.advance_animation(frame_time);
        }
        let Some(physics) = self.physics.as_mut() else {
            return;
        };
        if let Some(player) = self.player.as_mut() {
            player.step(physics, frame_time);
        }
        physics.update_physics(&self.nodes, frame_time);
        if let Some(player) = &self.player {
            let eye = player.eye(physics);
            self.camera.position = eye;
            self.camera.target = eye + player.direction();
        }
    }

    /// A first person player standing at `position`, a capsule 1.8
    /// tall and 0.7 wide. Needs physics.
    pub fn add_player(&mut self, position: impl Into<Vec3>) -> &mut Player {
        let physics = self
            .physics
            .as_mut()
            .expect("A player needs physics. Override SceneSetup::needs_physics to enable.");
        self.player = Some(Player::make(physics, position.into(), 0.35, 1.8));
        self.player.as_mut().expect("just set")
    }

    /// The ray from the camera through a pixel of the scene's area.
    pub fn ray(&self, point: Point) -> Ray {
        self.camera.ray(point, UIManager::render_area())
    }

    /// The nearest node under a pixel, by the ray against every node's
    /// solid, a model on its bounds.
    pub fn node_at(&self, point: Point) -> Option<Weak<dyn Node>> {
        self.hit(self.ray(point)).map(|(_, node)| node)
    }

    fn hit(&self, ray: Ray) -> Option<(f32, Weak<dyn Node>)> {
        self.nodes
            .iter()
            .filter_map(|node| node.hit(ray).map(|distance| (distance, node.weak())))
            .min_by(|a, b| a.0.total_cmp(&b.0))
    }

    /// A touch that no view took. The nearest node under it gets
    /// `on_touch` with the hit point, then `on_tap` fires with the ray.
    pub(crate) fn add_touch(&mut self, point: Point) -> bool {
        let ray = self.ray(point);
        if let Some((distance, node)) = self.hit(ray) {
            node.on_touch.trigger(ray.at(distance));
        }
        self.on_tap.trigger(ray);
        true
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
