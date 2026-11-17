use std::f32::consts::PI;

use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Quat, Shape3, Vec3},
    refs::Weak,
    scene::{
        Body, Camera, CoefficientCombineRule, ColliderShape, Node, NodeTemplates, SceneCreation, SceneSetup,
        SceneTest, Sky, Sun, Wall, WorldMove, scene, step_scene,
    },
    ui::Color,
    ui_test::{check_colors, checkpoint, set_record_probe_count},
};

const BALL: f32 = 0.4;
const BALL_START: Vec3 = Vec3::new(-3.0, BALL, 2.0);
const ROLL: Vec3 = Vec3::new(2.0, 0.0, -0.5);
const PLAYER_START: Vec3 = Vec3::new(3.0, 0.95, 3.0);
/// Frames of rolling before the move and after it.
const ROLL_FRAMES: u32 = 20;

/// A turn about up and a shift of 360 m, the move a game makes when its
/// player walked far from the origin. A turn about up leaves the gradient
/// sky as it was, so the picture must come back exactly.
fn the_move() -> WorldMove {
    WorldMove {
        turn:  Quat::from_rotation_y(0.6),
        shift: Vec3::new(300.0, 0.0, -200.0),
    }
}

/// A floor with a turned crate whose collider box sits a little to its
/// side, a ball rolling across the floor, a lamp, a sun and a player that
/// stands and looks at them. The whole scene moves and turns in 1 call,
/// and the picture through the moved camera is the same as before: the
/// crate's green collider box turned with it, a wall's offset turns with
/// the wall. Then the physics runs on, the ball rolls on along its turned
/// path and the player still stands.
#[scene]
#[derive(Default)]
struct MovedWorld {
    ball: Weak<Body>,
}

impl SceneSetup for MovedWorld {
    fn needs_physics(&self) -> bool {
        true
    }

    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 5.0, 9.0),
            target: Vec3::new(0.0, 0.5, 0.0),
            ..Camera::default()
        };
        self.sky = Some(Sky::gradient(
            Color::hex("#3a7bd5"),
            Color::hex("#d9e4f0"),
            Color::hex("#5a4a3a"),
        ));
        self.sun = Sun {
            direction: Vec3::new(-0.5, -1.0, -0.3),
            intensity: 0.8,
            ..Sun::default()
        };
        self.show_colliders = true;

        self.make_node::<Wall>(Shape3::Plane(40.0), Vec3::ZERO)
            .set_color(Color::hex("#8d9aa5"))
            .set_roughness(0.9)
            .set_restitution(0.0, CoefficientCombineRule::Min);

        self.make_node::<Wall>(Shape3::cuboid(2.0, 1.0, 1.0), Vec3::new(0.5, 0.5, -1.0))
            .set_color(Color::hex("#c0392b"))
            .set_roughness(0.7)
            .set_collider(
                ColliderShape::Box(Vec3::new(1.0, 1.0, 1.0)),
                Vec3::new(0.5, 0.0, 0.0),
            )
            .set_rotation(Quat::from_rotation_y(PI / 5.0));

        self.lights.push(
            hilen::scene::Light::point(Vec3::new(-2.0, 2.0, 1.0))
                .color(Color::hex("#ffb060"))
                .intensity(2.0)
                .range(6.0),
        );

        let mut ball = self.make_node::<Body>(Shape3::Ball(BALL), BALL_START);
        ball.set_color(Color::hex("#3498db"))
            .set_roughness(0.4)
            .set_restitution(0.0, CoefficientCombineRule::Min)
            .set_velocity(ROLL);
        self.ball = ball;

        let player = self.add_player(PLAYER_START);
        player.keyboard = false;
        player.mouse = false;
        player.look(-0.6, -0.35);
    }
}

impl SceneTest for MovedWorld {
    fn stepped() -> bool {
        true
    }

    fn perform_test(scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(48);
        wait_for_next_frame();
        check_colors(BEFORE)?;

        let world = the_move();
        let (ball, speed, player, look) = from_main(move || {
            let mut scene = scene;
            let player = scene.player.as_ref().expect("a player");
            let before = (
                scene.ball.position(),
                scene.ball.velocity(),
                player.position(),
                player.direction(),
            );
            scene.move_world(world);
            before
        });
        wait_for_next_frame();
        check_colors(BEFORE)?;

        let (moved_ball, moved_speed, moved_player, moved_look) = from_main(move || {
            let player = scene.player.as_ref().expect("a player");
            (
                scene.ball.position(),
                scene.ball.velocity(),
                player.position(),
                player.direction(),
            )
        });
        let off = |a: Vec3, b: Vec3| (a - b).length();
        ensure!(
            off(moved_ball, world.point(ball)) < 1e-3,
            "the ball went to {moved_ball}"
        );
        ensure!(
            off(moved_speed, world.direction(speed)) < 1e-4,
            "the ball rolls at {moved_speed}"
        );
        ensure!(
            off(moved_player, world.point(player)) < 1e-3,
            "the player went to {moved_player}"
        );
        ensure!(
            off(moved_look, world.direction(look)) < 1e-4,
            "the player looks along {moved_look}"
        );
        checkpoint("the scene moved and turned, the same picture")?;

        step_scene(ROLL_FRAMES);
        let (rolled, standing, grounded) = from_main(move || {
            let player = scene.player.as_ref().expect("a player");
            (scene.ball.position(), player.position(), player.grounded())
        });
        let back = world.inverse();
        let rolled = back.point(rolled);
        let path = rolled - BALL_START;
        ensure!(
            path.normalize().dot(ROLL.normalize()) > 0.98,
            "the ball rolled off its line, to {rolled} before the move"
        );
        ensure!(
            (rolled.y - BALL).abs() < 0.02,
            "the ball left the floor, its middle at {} before the move",
            rolled.y
        );
        ensure!(grounded, "the player stands on the moved floor");
        ensure!(
            off(back.point(standing), PLAYER_START) < 0.1,
            "the player slid to {}",
            back.point(standing)
        );
        checkpoint("the ball rolled on along its turned path, the player stands")
    }
}

/// The scene before the move, and after it through the moved camera.
const BEFORE: &str = r"
     112    4 - #d4dfec
     256    4 - #d4dfec
     452    4 - #d4dfec
     592    4 - #d4dfec
       4   96 - #d5dfeb
     164  140 - #d2dde8
     592  144 - #8ea4c6
     352  244 - #c15050
     316  248 - #c25050
     276  264 - #a93b36
     336  276 - #b2413b
     372  280 - #b2423c
     296  284 - #a73b35
     272  300 - #9b3936
     292  300 - #a13a35
     324  312 - #b2413b
     296  316 - #9c3935
     120  340 - #5daaf4
     100  344 - #479bf1
     128  344 - #5dabf4
     112  348 - #58a5f5
      96  352 - #3e96ef
      88  356 - #4b91e1
     104  360 - #429bf0
     124  360 - #4ca2f2
     136  360 - #4ca0e9
      88  368 - #437fc3
     100  368 - #3e95e6
     124  368 - #409bea
     580  368 - #8ea3c6
     112  372 - #3d95e4
     128  372 - #378dd7
     132  372 - #3885c9
      92  376 - #3d79b8
     120  376 - #3488d0
     124  376 - #3284ca
     128  376 - #327dbe
     100  380 - #2e71b0
     108  380 - #2d77b8
     120  380 - #2c76b4
     124  380 - #2d6ea8
     104  384 - #356fa6
     112  384 - #2c6ba3
     116  384 - #2e6ba1
     428  460 - #8ea3c5
       4  592 - #8da3c5
     308  592 - #8da3c5
     592  592 - #8da3c5
";
