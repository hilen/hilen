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
     592    4 - #597c95
     476  108 - #474e4e
     180  172 - #c8702a
     240  180 - #a03a31
     160  184 - #99a1a2
     284  184 - #ebc133
     376  184 - #9f63b9
     440  184 - #99a1a2
     256  188 - #e65a4f
     300  188 - #d0aa21
     268  192 - #3583bb
     356  192 - #5a3769
     252  200 - #945853
     276  200 - #479dde
     368  200 - #804d96
     256  204 - #4e738e
     272  208 - #4b9cdb
     144  252 - #959c9d
     428  272 - #237040
     448  272 - #42cb77
     376  276 - #563564
     396  276 - #9c60b5
     448  284 - #3dc874
     396  288 - #9d62b6
     440  292 - #2c9957
     364  300 - #b08f1f
     332  312 - #f09257
     332  316 - #e8853d
     436  316 - #7e471f
     376  324 - #e2b822
     456  324 - #f19a67
     280  336 - #71b1eb
     152  344 - #c9cfcf
     204  344 - #9b3830
     464  344 - #cacfcf
     428  348 - #7e2f28
     464  352 - #cb8c86
     284  356 - #4e8d67
     324  356 - #6e4381
     328  356 - #714485
      60  360 - #606969
     188  360 - #ac3d33
     216  372 - #ce493c
     308  372 - #5f3a6f
     268  376 - #247141
     288  376 - #6ddd92
     328  376 - #af7dc6
     164  400 - #2b9756
     356  400 - #7e471f
     216  404 - #2e6e9d
     428  416 - #856c1c
     208  420 - #63a9e5
     372  420 - #e98948
     468  420 - #27587e
     168  424 - #44cf7a
     456  424 - #255478
     144  428 - #b2911f
     124  432 - #fed560
     288  436 - #27814a
     448  436 - #7b7f4e
     188  444 - #754789
     432  444 - #efc437
     484  444 - #3f9adb
     592  468 - #7d8989
";
