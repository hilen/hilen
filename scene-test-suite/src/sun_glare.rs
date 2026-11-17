use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Shape3, Vec3},
    refs::Weak,
    scene::{
        Camera, NodeTemplates, Occluder, Prop, SceneCreation, SceneSetup, SceneTest, SpaceSky, Sun, scene,
    },
    ui::Color,
    ui_test::check_colors,
};

/// The sun's angular radius, the real sun's.
const SUN: f32 = 0.0047;
const BALL: f32 = 1000.0;

/// The sun in black space, a little off the middle of the view, with the
/// glare of `SpaceSky::sun_glare` round it: a bright core and a wide dim
/// halo, far bigger than the small disc. Then a ball moves in front of the
/// sun and hides it, and the glare goes with it, the space round the ball
/// is black again.
#[scene]
#[derive(Default)]
struct SunGlare {
    ball: Option<Weak<Prop>>,
}

impl SceneSetup for SunGlare {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::ZERO,
            target: Vec3::new(0.15, 0.0, -1.0),
            far: f32::INFINITY,
            ..Camera::default()
        };
        self.sun = Sun {
            direction: Vec3::Z,
            angular_radius: SUN,
            ..Sun::default()
        };
        self.space_sky = Some(SpaceSky {
            star_brightness: 0.0,
            sun_glare: 8.0,
            ..SpaceSky::default()
        });
        // Out of the way of the sun until the second check.
        let mut ball = self.make_node::<Prop>(Shape3::Ball(BALL), Vec3::new(0.0, 0.0, 50_000.0));
        ball.set_color(Color::hex("#202020"));
        self.ball = Some(ball);
        self.occluders = vec![Occluder::new(Vec3::new(0.0, 0.0, 50_000.0), BALL)];
    }
}

impl SceneTest for SunGlare {
    fn perform_test(mut scene: Weak<Self>) -> Result<()> {
        wait_for_next_frame();
        check_colors(GLARE)?;
        from_main(move || {
            let hide = Vec3::new(0.0, 0.0, -20_000.0);
            if let Some(ball) = &mut scene.ball {
                ball.set_position(hide);
            }
            scene.occluders = vec![Occluder::new(hide, BALL)];
        });
        wait_for_next_frame();
        check_colors(HIDDEN)
    }
}

/// The glare round the sun, bright at the disc and fading out far past it.
const GLARE: &str = r"
       4    4 - #000000
     592    4 - #000000
     148  212 - #151515
     288  212 - #151515
     212  232 - #2a2a2a
     108  272 - #151515
     188  276 - #3f3f3f
     212  280 - #555555
     224  288 - #6f6f6f
     288  288 - #2a2a2a
     212  292 - #6e6e6e
     220  292 - #8c8c8c
     224  292 - #898989
     232  292 - #6b6b6b
     244  296 - #545454
     212  300 - #7c7c7c
     220  300 - #fefefe
     228  300 - #9c9c9c
     580  300 - #000000
     216  308 - #787878
     224  308 - #818181
     260  308 - #3f3f3f
     204  312 - #565656
     240  312 - #545454
     184  316 - #3f3f3f
     236  336 - #3f3f3f
     272  344 - #2a2a2a
     220  364 - #2c2c2c
     140  380 - #151515
     268  400 - #151515
       4  592 - #000000
     592  592 - #000000
";

/// The sun behind the ball, no glare round it.
const HIDDEN: &str = r"
       4    4 - #000000
     308    4 - #000000
     592    4 - #000000
     156   76 - #000000
     296  148 - #000000
     448  152 - #000000
      36  192 - #000000
     228  276 - #1f1f1f
     208  280 - #1f1f1f
     220  288 - #1f1f1f
     244  288 - #1f1f1f
     196  296 - #1f1f1f
     232  296 - #1f1f1f
     208  300 - #1f1f1f
     220  300 - #1f1f1f
     240  300 - #1f1f1f
     300  300 - #000000
     592  300 - #000000
     232  308 - #1f1f1f
     244  308 - #1f1f1f
     204  312 - #1f1f1f
     220  312 - #1f1f1f
     240  316 - #1f1f1f
     232  320 - #1f1f1f
     216  324 - #1f1f1f
       4  380 - #000000
     448  444 - #000000
     300  448 - #000000
     156  496 - #000000
       4  592 - #000000
     308  592 - #000000
     592  592 - #000000
";
