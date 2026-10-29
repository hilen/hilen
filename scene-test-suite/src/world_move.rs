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
       4    4 - #d4dfec
     260    4 - #d4dfec
     592    4 - #d4dfec
     424   56 - #d5e0ec
     132  140 - #d3dde8
     540  184 - #8ea3c6
     328  248 - #c25050
     356  248 - #c15050
     276  260 - #c95050
     376  268 - #47d193
     376  272 - #6bbaac
     280  276 - #a63b36
     324  276 - #b2413b
     296  280 - #a93b35
     272  292 - #9e3a36
     288  300 - #a03a35
     340  300 - #b2413b
     296  316 - #9c3935
     120  340 - #5caaf4
      96  344 - #509bef
     128  344 - #5dabf4
     116  348 - #60a8f5
     104  352 - #23cea9
      96  356 - #3e97ee
     120  356 - #5aa6f4
      88  360 - #468cdb
     136  360 - #4ca0e9
     592  360 - #8da3c5
      96  364 - #3e95e8
     104  364 - #419aee
     124  364 - #479fef
      88  368 - #437fc4
      92  372 - #397dc3
     112  372 - #3d96e4
     128  372 - #378ed7
     100  376 - #1ac094
     120  376 - #3489d1
     124  376 - #3285ca
     128  376 - #327ebf
     108  380 - #2d77b9
     116  380 - #2d79ba
     124  380 - #2d6fa9
     104  384 - #356fa6
     112  384 - #2c6ba3
     116  384 - #2e6ba1
       4  592 - #8da3c5
     312  592 - #8da3c5
     592  592 - #8da3c5
";
