use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    gm::{
        LossyConvert,
        volume::{Quat, Shape3, Vec3},
    },
    refs::Weak,
    scene::{
        Motion, Node, NodeTemplates, Player, Prop, SceneBase, SceneCreation, SceneSetup, SceneTest, Sky,
        ThirdPerson, WorldMove, scene, step_scene,
    },
    ui::Color,
};

/// A jet in space, the speed where a lag shows.
const SPEED: f32 = 6000.0;
/// Every step the scene moves back by this much once the player got
/// farther, the floating origin of a space game.
const BUBBLE: f32 = 1000.0;
const STEPS: u32 = 60;

/// A player flying at 6 km/s with the camera behind it, and 2 boxes on
/// it. 1 is placed in `after_physics`, it keeps its place from the camera
/// every step, also when the scene moves back to the player. The other
/// is placed in `update`, before the physics step, and lags 1 step of
/// travel behind, so the test tells the 2 apart.
#[scene]
#[derive(Default)]
struct RideWithPlayer {
    after:  Option<Weak<Prop>>,
    before: Option<Weak<Prop>>,
}

impl SceneSetup for RideWithPlayer {
    fn needs_physics(&self) -> bool {
        true
    }

    fn setup(&mut self) {
        self.sky = Some(Sky::gradient(
            Color::hex("#0a0a20"),
            Color::hex("#101030"),
            Color::hex("#050510"),
        ));
        let mut after = self.make_node::<Prop>(Shape3::cuboid(1.0, 1.0, 1.0), Vec3::ZERO);
        after.set_color(Color::hex("#e0a030"));
        let mut before = self.make_node::<Prop>(Shape3::cuboid(1.0, 1.0, 1.0), Vec3::ZERO);
        before.set_color(Color::hex("#3090e0"));
        self.after = Some(after);
        self.before = Some(before);
        let player = self.add_player(Vec3::ZERO);
        player.mouse = false;
        player.keyboard = false;
        player.motion = Motion::Fly;
        player.speed = SPEED;
        player.steer = Some(Vec3::NEG_Z);
        player.third_person = Some(ThirdPerson::default());
    }

    fn update(&mut self, _: f32) {
        let at = self.player.as_ref().map_or(Vec3::ZERO, Player::position);
        if let Some(node) = &mut self.before {
            node.set_position(at);
        }
    }

    fn after_physics(&mut self, _: f32) {
        let at = self.player.as_ref().map_or(Vec3::ZERO, Player::position);
        if let Some(node) = &mut self.after {
            node.set_position(at);
        }
        if at.length() > BUBBLE {
            self.move_world(WorldMove {
                turn:  Quat::IDENTITY,
                shift: -at,
            });
        }
    }
}

impl SceneTest for RideWithPlayer {
    fn stepped() -> bool {
        true
    }

    fn perform_test(scene: Weak<Self>) -> Result<()> {
        step_scene(5);
        let read = move || {
            from_main(move || {
                let camera = scene.camera.position;
                let after = scene.after.map_or(Vec3::ZERO, |node| node.position());
                let before = scene.before.map_or(Vec3::ZERO, |node| node.position());
                (after - camera, before - after)
            })
        };
        let (first, _) = read();
        let substep = SPEED / 60.0 / SceneBase::PHYSICS_SUBSTEPS.lossy_convert();
        for step in 0..STEPS {
            step_scene(1);
            let (offset, lag) = read();
            ensure!(
                offset.distance(first) < 0.01,
                "step {step}: the box rides {offset} from the camera, it started at {first}"
            );
            ensure!(
                (lag.length() - substep).abs() < 0.5,
                "step {step}: the box of update lags {} m, 1 substep is {substep} m",
                lag.length()
            );
        }
        Ok(())
    }
}
