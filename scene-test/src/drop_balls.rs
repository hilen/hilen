use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::{
        LossyConvert,
        volume::{Shape3, Vec3},
    },
    refs::Weak,
    scene::{Body, Camera, Node, NodeTemplates, SceneCreation, SceneSetup, SceneTest, Wall, scene},
    ui::Color,
    ui_test::{capture_screenshot, check_colors, set_record_probe_count},
};

const HALF: f32 = 4.0;
const WALL: f32 = 0.5;
const WALL_HEIGHT: f32 = 3.0;
const BALLS: usize = 24;
const FRAMES: usize = 360;
/// A sideways push at the drop, so the balls roll and bounce off the
/// walls instead of settling where they land.
const PUSH: f32 = 3.0;
/// Motion left by this speed counts as rest.
const REST_SPEED: f32 = 0.02;

const FLOOR: Color = Color::hex("#d5dbdb");
const WALLS: Color = Color::hex("#7f8c8d");

/// A floor with four walls and two dozen balls dropped in from a height
/// with a sideways push, seen from above the box. With continuous
/// collision detection and the physics substeps every ball rolls inside
/// the box, none falls through the floor and none is flung over a wall.
/// Friction and damping bring every ball to rest within the run, and
/// the probes pin where they stopped, so the physics has to be the same
/// on every run.
#[scene]
#[derive(Default)]
struct DropBalls {
    balls: Vec<Weak<Body>>,
}

impl SceneSetup for DropBalls {
    fn needs_physics(&self) -> bool {
        true
    }

    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 11.0, 6.0),
            target: Vec3::new(0.0, 0.5, 0.0),
            ..Camera::default()
        };

        let outer = HALF + WALL / 2.0;
        let length = 2.0 * outer + WALL;
        // The side walls fit between the front and back ones, overlapping
        // boxes would z fight at the corners.
        let side = 2.0 * HALF;

        self.make_node::<Wall>(Shape3::Plane(length), Vec3::ZERO).set_color(FLOOR);

        for (center, shape) in [
            (
                Vec3::new(0.0, WALL_HEIGHT / 2.0, -outer),
                Shape3::cuboid(length, WALL_HEIGHT, WALL),
            ),
            (
                Vec3::new(0.0, WALL_HEIGHT / 2.0, outer),
                Shape3::cuboid(length, WALL_HEIGHT, WALL),
            ),
            (
                Vec3::new(-outer, WALL_HEIGHT / 2.0, 0.0),
                Shape3::cuboid(WALL, WALL_HEIGHT, side),
            ),
            (
                Vec3::new(outer, WALL_HEIGHT / 2.0, 0.0),
                Shape3::cuboid(WALL, WALL_HEIGHT, side),
            ),
        ] {
            self.make_node::<Wall>(shape, center).set_color(WALLS);
        }

        for i in 0..BALLS {
            let along = i.lossy_convert() / (BALLS - 1).lossy_convert();
            let x = (along - 0.5) * 2.0 * (HALF - 1.0);
            let z = ((i % 5).lossy_convert() - 2.0) * 1.2;
            // Low enough that a ball meets a wall below its top, a higher
            // drop with this push flies over it and out of the world.
            let y = 1.5 + (i % 3).lossy_convert() * 0.5;
            let mut ball = self.make_node::<Body>(Shape3::Ball(0.4), Vec3::new(x, y, z));
            let angle = i.lossy_convert() * 0.7;
            ball.set_velocity(Vec3::new(angle.cos() * PUSH, 0.0, angle.sin() * PUSH))
                .set_damping(0.6, 0.6)
                .set_friction(1.0)
                .set_color(Color::hex(BALL_COLORS[i % BALL_COLORS.len()]));
            self.balls.push(ball);
        }
    }
}

const BALL_COLORS: [&str; 6] = ["#e74c3c", "#3498db", "#2ecc71", "#f1c40f", "#9b59b6", "#e67e22"];

impl SceneTest for DropBalls {
    fn perform_test(scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);

        for _ in 0..FRAMES {
            wait_for_next_frame();
        }

        let (positions, speeds): (Vec<Vec3>, Vec<f32>) = from_main(move || {
            scene
                .balls
                .iter()
                .map(|ball| (ball.position(), ball.velocity().length()))
                .unzip()
        });

        for (i, pos) in positions.iter().enumerate() {
            anyhow::ensure!(
                pos.x.abs() < HALF && pos.z.abs() < HALF && pos.y > 0.0 && pos.y < WALL_HEIGHT,
                "ball {i} left the box, it is at {pos:?}"
            );
        }

        for (i, speed) in speeds.iter().enumerate() {
            anyhow::ensure!(
                *speed < REST_SPEED,
                "ball {i} is still moving at {speed} after {FRAMES} frames"
            );
        }

        capture_screenshot()?;
        check_colors(DROP_BALLS)
    }
}

const DROP_BALLS: &str = r"
       4    4 - #597c95
     592    4 - #597c95
     124  104 - #5b6565
     476  108 - #414849
     372  176 - #864c9e
     240  180 - #9e3126
     276  180 - #be9a09
     160  184 - #969f9f
     336  184 - #969f9f
     440  184 - #969f9f
     196  188 - #c56b1b
     300  188 - #cea70b
     252  200 - #925049
     368  200 - #7d4793
     256  204 - #436f8b
     228  212 - #b3392c
     508  240 - #414849
     144  252 - #919999
     456  272 - #2cc36c
     236  276 - #c7cdcd
     384  276 - #814998
     428  276 - #15703c
     404  280 - #9656b1
     428  284 - #146c39
     440  292 - #1f9652
      76  296 - #5b6565
     388  296 - #78448d
     312  308 - #9a5213
     348  316 - #967905
     380  316 - #ccb13c
     276  320 - #2674a9
     436  320 - #81440e
     464  340 - #c7cdcd
     204  344 - #992f25
     428  348 - #7b241c
     464  352 - #c98780
     284  356 - #428a61
     328  356 - #6e3d82
     436  364 - #a53428
     308  368 - #512c60
     208  376 - #cf4335
     268  376 - #146e3a
     336  380 - #9756b1
     296  384 - #2bc26b
     168  400 - #1f9551
     352  412 - #8a4910
     428  416 - #826904
     384  420 - #db7820
     468  420 - #19547b
     108  424 - #bf9b09
     196  424 - #2e8ac7
     456  424 - #174f75
     128  428 - #eec10f
     144  428 - #b08f08
     416  428 - #a28306
     452  428 - #174f75
     460  428 - #21699a
     148  436 - #c7cdcd
     448  436 - #737b40
     200  440 - #643776
     244  444 - #c7cdcd
     296  444 - #25ab5e
     412  444 - #866c04
     484  444 - #3396d9
";
