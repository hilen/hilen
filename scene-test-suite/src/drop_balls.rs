use anyhow::Result;
use hilen::{
    dispatch::from_main,
    gm::{
        LossyConvert,
        volume::{Shape3, Vec3},
    },
    refs::Weak,
    scene::{
        Body, Camera, Node, NodeTemplates, SceneCreation, SceneSetup, SceneTest, Wall, scene, step_scene,
    },
    ui::Color,
    ui_test::{capture_screenshot, check_colors, set_record_probe_count},
};

const HALF: f32 = 4.0;
const WALL: f32 = 0.5;
const WALL_HEIGHT: f32 = 3.0;
const BALLS: usize = 24;
const FRAMES: u32 = 360;
/// A sideways push at the drop, so the balls roll and bounce off the
/// walls instead of settling where they land. Every ball is pushed at
/// speed 3, turned 0.7 radians on from the one before. Written out,
/// since `cos` and `sin` differ in the last bit between an arm64 Mac,
/// the x86_64 simulator and wasm, and 24 colliding balls turn that bit
/// into another rest.
const PUSHES: [(f32, f32); BALLS] = [
    (3.0, 0.0),
    (2.294_526_6, 1.932_653),
    (0.509_901_46, 2.956_349_1),
    (-1.514_538, 2.589_628_2),
    (-2.826_666_8, 1.004_964_6),
    (-2.809_37, -1.052_349_7),
    (-1.470_783, -2.614_727),
    (0.559_537_4, -2.947_357_7),
    (2.326_697_3, -1.893_800_1),
    (2.999_575_9, 0.050_440_848),
    (2.261_706_8, 1.970_959_8),
    (0.460_122_14, 2.964_504_5),
    (-1.557_864_9, 2.563_797_5),
    (-2.843_164_2, 0.957_296_7),
    (-2.791_278_6, -1.099_438),
    (-1.426_610_7, -2.639_087_2),
    (0.609_014_03, -2.937_533_4),
    (2.358_21, -1.854_412_3),
    (2.998_304, 0.100_867_435),
    (2.228_247_2, 2.008_709_7),
    (0.410_211_62, 2.971_822_3),
    (-1.600_752_7, 2.537_240_7),
    (-2.858_858_3, 0.909_356_24),
    (-2.772_398, -1.146_215_3),
];
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
            let (push_x, push_z) = PUSHES[i];
            ball.set_velocity(Vec3::new(push_x, 0.0, push_z))
                .set_damping(0.6, 0.6)
                .set_friction(1.0)
                .set_color(Color::hex(BALL_COLORS[i % BALL_COLORS.len()]));
            self.balls.push(ball);
        }
    }
}

const BALL_COLORS: [&str; 6] = ["#e74c3c", "#3498db", "#2ecc71", "#f1c40f", "#9b59b6", "#e67e22"];

impl SceneTest for DropBalls {
    fn stepped() -> bool {
        true
    }

    fn perform_test(scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);

        step_scene(FRAMES);

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
     180  172 - #c86f2a
     240  180 - #9f3930
     292  180 - #edc337
     160  184 - #99a1a1
     260  184 - #e7574b
     356  184 - #563564
     440  184 - #99a1a1
     368  188 - #9b62b3
     380  188 - #9b5db5
     268  204 - #4598d7
     228  208 - #b34035
     284  208 - #3789c4
     256  212 - #ba8681
     144  252 - #959c9c
     448  272 - #42cb77
     380  280 - #78488c
     428  280 - #247342
     400  284 - #a164bb
     440  292 - #2c9857
     388  296 - #7b4a90
     364  300 - #b08f1f
     332  312 - #eb9056
     368  316 - #f0ca5e
     436  320 - #814820
     460  320 - #e88742
      72  324 - #60696a
     376  324 - #e2b823
     184  332 - #c9cfcf
     276  336 - #509fdd
     464  344 - #c9cfcf
     428  348 - #7e2f28
     452  352 - #eb6e65
     220  356 - #c14439
     324  356 - #6e4281
     328  356 - #714484
     192  364 - #cd493c
     308  368 - #563564
     288  372 - #54d282
     268  376 - #237040
     292  376 - #54d583
     328  376 - #af7dc7
     212  404 - #3076a8
     124  412 - #b3921f
     372  416 - #ed9868
     428  416 - #856c1c
     164  420 - #69dc90
     468  420 - #27597e
     456  424 - #255478
     144  428 - #af8f1f
     212  428 - #479ede
     284  428 - #257744
     124  432 - #f1cb64
     412  432 - #8d731c
     164  436 - #7bb393
     448  436 - #7b7f4e
     196  440 - #69407b
     436  440 - #f0c856
     272  444 - #2c9957
     300  444 - #37c36f
     452  444 - #285e86
     476  444 - #64aae5
     592  476 - #7d8989
";
