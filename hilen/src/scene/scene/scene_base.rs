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
    scene::{
        Camera, DayNightSky, Fog, Grass, GravityWell, Light, Line3, Node, Occluder, Player, Scene,
        SceneManager, Sky, SpaceSky, Sun, TransformHandle, Wind, follow_parent,
        scene::scene_physics::ScenePhysics,
    },
    ui::UIManager,
};

#[derive(Educe)]
#[educe(Default)]
pub struct SceneBase {
    pub(crate) nodes: Vec<Own<dyn Node>>,

    pub camera: Camera,

    pub sun:           Sun,
    /// What lights every node from all directions at once, the stand in
    /// for a sky until one exists. Encoded sRGB like every color, the
    /// default is a quarter of full light.
    #[educe(Default = Color::hex("#898989"))]
    pub ambient:       Color,
    /// The point and spot lights. A node is drawn with the nearest
    /// eight of those in reach of it.
    pub lights:        Vec<Light>,
    /// Balls that hide the sun and the directional lights, a planet and
    /// its moon, see `Occluder`. The first 4 count.
    pub occluders:     Vec<Occluder>,
    /// Drawn behind everything and reflected by every surface. With a
    /// sky the flat `ambient` is not used, the sky lights the scene.
    pub sky:           Option<Sky>,
    /// A sky drawn in the shader that a game moves with its clock, see
    /// `DayNightSky`. It is drawn in place of `sky`, which still lights
    /// the scene when both are set.
    pub day_night_sky: Option<DayNightSky>,
    /// The sky of a planet or a moon in space, with air or without, see
    /// `SpaceSky`. Drawn in place of `sky` and `day_night_sky`.
    pub space_sky:     Option<SpaceSky>,
    /// Distance fog over every surface, none by default.
    pub fog:           Option<Fog>,
    /// The first person player, see `add_player`. While one exists the
    /// camera follows its eyes.
    pub player:        Option<Player>,

    /// Fields of grass that grow around the player, see `Grass`.
    pub grass:               Vec<Grass>,
    /// The wind that leans and sways the grass.
    pub wind:                Wind,
    /// The player's feet this frame, what the grass leans away from, and
    /// where they were a moment ago, which leaves a trail in the grass.
    pub(crate) grass_pusher: Option<Vec3>,
    pub(crate) grass_trail:  Option<Vec3>,

    /// Lines in the world, axes or a floor grid, see `Line3`. A node
    /// carries the lines of its own space in its own `lines`.
    pub lines: Vec<Line3>,

    /// A handle to move, turn and scale a node with a drag, see
    /// `TransformHandle`. A `SceneView` hands it the drags that begin on
    /// it.
    pub handle: Option<TransformHandle>,

    /// A touch that no view took, as the ray it makes from the camera.
    /// The nearest node under it gets its own `on_touch` first.
    pub on_tap: Event<Ray>,

    /// Draws every collider as a green wireframe over the scene, the
    /// box around a model's bounds, three rings on a ball. A plane's
    /// slab is the floor itself and is not drawn. Off by default.
    pub show_colliders: bool,

    /// Draws the edges of every triangle of every node as white lines
    /// over the scene, depth tested so a hidden edge stays hidden. Off by
    /// default.
    pub show_wireframe: bool,

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

    /// One step of the scene's time: the playing clips and fades move on, the
    /// colliders of attached nodes go where their parents carry them, then
    /// the physics, when the scene has any.
    pub fn update_physics(&mut self, frame_time: f32) {
        let wind = self.wind;
        for grass in &mut self.grass {
            grass.advance(frame_time, wind);
        }
        let physics = self.physics.is_some();
        for node in &mut self.nodes {
            node.advance_animation(frame_time);
            node.advance_fade(frame_time);
            if physics {
                follow_parent(&mut **node);
            }
        }
        let Some(physics) = self.physics.as_mut() else {
            return;
        };
        if let Some(player) = self.player.as_mut() {
            player.step(physics, frame_time);
        }
        physics.update_physics(&self.nodes, frame_time);
        self.follow_player();
    }

    /// Grows the grass around the player, or around the camera with no
    /// player, once a frame after its steps, see `Grass`.
    pub fn update_grass(&mut self) {
        let player = self.player.as_ref().zip(self.physics.as_ref()).map(|(player, physics)| {
            (
                player.eye(physics) - player.up() * player.eye_height,
                player.eye_height,
                player.up(),
            )
        });
        // The eye sits 0.15 under the top of the capsule, so the feet are
        // that far and the eye height again under the center.
        self.grass_pusher = player.map(|(center, eye, up)| center - up * (eye + 0.15));
        // The trail moves a 20th of the way to the feet a frame.
        self.grass_trail = match (self.grass_trail, self.grass_pusher) {
            (Some(trail), Some(feet)) => Some(trail.lerp(feet, 0.05)),
            (_, feet) => feet,
        };
        let center = player.map_or(self.camera.position, |(center, ..)| center);
        for grass in &mut self.grass {
            grass.update(center);
        }
    }

    /// Puts the camera where the player's view wants it: out of its eyes,
    /// or behind it over the shoulder and pulled in front of any wall on
    /// the way, see `ThirdPerson`.
    fn follow_player(&mut self) {
        let (Some(player), Some(physics)) = (&self.player, &self.physics) else {
            return;
        };
        let (direction, up) = (player.direction(), player.up());
        let position = match &player.third_person {
            None => player.eye(physics),
            Some(rig) => {
                let pivot = player.eye(physics) + up * (rig.pivot_height - player.eye_height);
                let reach = rig.reach(direction, player.right());
                let ray = Ray {
                    origin:    pivot,
                    direction: reach,
                };
                let hit = self.cast_ray(ray, reach.length(), &rig.skip).map(|hit| hit.distance);
                rig.place(pivot, direction, player.right(), hit)
            }
        };
        self.camera.position = position;
        self.camera.target = position + direction;
        self.camera.up = up;
    }

    /// A first person player standing at `position`, a capsule 1.8
    /// tall and 0.7 wide. Needs physics.
    pub fn add_player(&mut self, position: impl Into<Vec3>) -> &mut Player {
        self.add_player_sized(position, 0.35, 1.8)
    }

    /// A player whose capsule is `radius` wide and `height` tall from tip
    /// to tip, for a hero smaller or bigger than a person. The eyes sit
    /// near the top of it. Needs physics.
    pub fn add_player_sized(&mut self, position: impl Into<Vec3>, radius: f32, height: f32) -> &mut Player {
        let physics = self
            .physics
            .as_mut()
            .expect("A player needs physics. Override SceneSetup::needs_physics to enable.");
        self.player = Some(Player::make(physics, position.into(), radius, height));
        self.player.as_mut().expect("just set")
    }

    /// The ray from the camera through a pixel of the scene's area, the
    /// root area or the frame of its `SceneView`.
    pub fn ray(&self, point: Point) -> Ray {
        self.camera.ray(point, SceneManager::area())
    }

    /// Where a world point lands on the screen, in points of the root view,
    /// so a view centered there sits over it, a damage number or a name
    /// over a node. None while the point is behind the camera. A scene in a
    /// `SceneView` lands inside the frame of that view.
    pub fn view_point(&self, world: impl Into<Vec3>) -> Option<Point> {
        let point = self.camera.screen_point(world.into(), SceneManager::area())?;
        let scale = UIManager::scale();
        Some(SceneManager::origin() + Point::new(point.x / scale, point.y / scale))
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
    /// The same pull everywhere, 9.81 down by default.
    fn set_gravity(&mut self, g: impl Into<Vec3>);
    /// Masses that pull every body and the player toward their centers, a
    /// planet and its moon, on top of the uniform gravity. A round world
    /// sets the uniform gravity to zero.
    fn set_wells(&mut self, wells: Vec<GravityWell>);
    /// The wells, moved with the scene by `move_world`.
    fn wells(&self) -> &[GravityWell];
    /// The pull of gravity at `point`, the uniform part and every well.
    /// Zero for a scene with no physics.
    fn gravity_at(&self, point: impl Into<Vec3>) -> Vec3;
}

impl<T: ?Sized + Scene> SceneTemplates for T {
    fn set_gravity(&mut self, g: impl Into<Vec3>) {
        if let Some(physics) = self.physics.as_mut() {
            physics.gravity = g.into();
        }
    }

    fn set_wells(&mut self, wells: Vec<GravityWell>) {
        if let Some(physics) = self.physics.as_mut() {
            physics.wells = wells;
        }
    }

    fn wells(&self) -> &[GravityWell] {
        self.physics.as_ref().map_or(&[], |physics| &physics.wells)
    }

    fn gravity_at(&self, point: impl Into<Vec3>) -> Vec3 {
        self.physics
            .as_ref()
            .map_or(Vec3::ZERO, |physics| physics.gravity_at(point.into()))
    }
}
