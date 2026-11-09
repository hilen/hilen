use std::f32::consts::PI;

use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::{
        LossyConvert,
        volume::{Ray, Shape3, Vec3},
    },
    refs::{Weak, manage::DataManager},
    scene::{
        Camera, Model, Node, NodeTemplates, Prop, SceneCreation, SceneManager, SceneSetup, SceneTest, scene,
    },
    ui::{Color, Point, WHITE},
    ui_test::{check_colors, inject_touches, set_record_probe_count},
};

/// The test canvas, square so the aspect is one.
const CANVAS: f32 = 600.0;
/// Frames of the 70 degree turn to the second angle.
const TURN_FRAMES: usize = 20;
const TURN: f32 = -PI * 70.0 / 180.0;

const BALL_AT: Vec3 = Vec3::new(0.0, 0.9, 0.0);
const BALL_RADIUS: f32 = 0.9;
const MONKEY_AT: Vec3 = Vec3::new(3.0, 1.0, 0.0);

const CRATE: u32 = 1;
const BALL: u32 = 2;
const MONKEY: u32 = 3;
const PEBBLE: u32 = 4;

/// A crate, a ball and the monkey in a row, and a pebble floating on
/// the line from the camera to the ball. A tap that no view takes
/// becomes a ray from the camera, and the nearest node it hits gets
/// `on_touch` with the hit point, so the tap on the ball lands on the
/// pebble in front of it, the monkey is hit on its bounds, and a tap on
/// the sky hits nothing but still fires `on_tap` with its ray. A touched
/// node flips between its color and white, so a tap shows, and a second
/// tap on the crate flips it back. Then the camera turns 70 degrees and
/// the taps land from there, the ball itself now that the pebble is off
/// the line, hit on its surface, and the monkey again, back to its color.
#[scene]
#[derive(Default)]
struct Picking {
    touched: Vec<(u32, Vec3)>,
    taps:    Vec<Ray>,
}

impl Picking {
    /// The pixel a world point is drawn at, the forward path a tap's
    /// ray has to invert.
    fn pixel(&self, world: Vec3) -> Point {
        let clip = self.camera.view_projection(1.0).project_point3(world);
        Point::new(f32::midpoint(clip.x, 1.0) * CANVAS, (1.0 - clip.y) / 2.0 * CANVAS)
    }
}

impl SceneSetup for Picking {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 4.0, 10.0),
            target: Vec3::new(0.0, 0.8, 0.0),
            ..Camera::default()
        };

        self.make_node::<Prop>(Shape3::Plane(14.0), Vec3::ZERO)
            .set_color(Color::hex("#c8ccd0"))
            .set_roughness(0.9);

        let touch = |tag: u32, node: &mut dyn Node| {
            node.tag = tag;
            let mut weak = node.weak_node();
            let own_color = node.material.color;
            let mut lit = false;
            node.on_touch.val(move |point| {
                SceneManager::downcast_scene::<Picking>().touched.push((tag, point));
                lit = !lit;
                weak.set_color(if lit { WHITE } else { own_color });
            });
        };

        let mut crate_box = self.make_node::<Prop>(Shape3::cube(1.6), Vec3::new(-3.0, 0.8, 0.0));
        crate_box.set_color(Color::hex("#d35400"));
        touch(CRATE, &mut *crate_box);

        let mut ball = self.make_node::<Prop>(Shape3::Ball(BALL_RADIUS), BALL_AT);
        ball.set_color(Color::hex("#2980b9"));
        touch(BALL, &mut *ball);

        let mut monkey = self.make_node::<Prop>(Shape3::Model(Model::get("Monkey.glb")), MONKEY_AT);
        monkey.set_color(Color::hex("#e0a060"));
        touch(MONKEY, &mut *monkey);

        // A third of the way from the camera to the ball's center.
        let pebble_at = self.camera.position.lerp(BALL_AT, 0.34);
        let mut pebble = self.make_node::<Prop>(Shape3::Ball(0.25), pebble_at);
        pebble.set_color(Color::hex("#2ecc71"));
        touch(PEBBLE, &mut *pebble);

        self.on_tap.val(|ray| SceneManager::downcast_scene::<Picking>().taps.push(ray));
    }
}

impl SceneTest for Picking {
    fn canvas() -> (u32, u32) {
        (600, 600)
    }

    fn perform_test(mut scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);
        wait_for_next_frame();

        let (crate_px, ball_px, monkey_px) = from_main(move || {
            (
                scene.pixel(Vec3::new(-3.0, 0.8, 0.0)),
                scene.pixel(BALL_AT),
                scene.pixel(MONKEY_AT),
            )
        });

        check_colors(ROW)?;

        // The crate, the pebble in front of the ball, the monkey, the sky
        // and the crate again, each check pins what the tap lit or put
        // back.
        for (pixel, colors) in [
            (crate_px, CRATE_LIT),
            (ball_px, PEBBLE_LIT),
            (monkey_px, MONKEY_LIT),
            (Point::new(300.0, 20.0), SKY_TAPPED),
            (crate_px, CRATE_BACK),
        ] {
            tap(pixel);
            check_colors(colors)?;
        }

        let frames: f32 = TURN_FRAMES.lossy_convert();
        for _ in 0..TURN_FRAMES {
            from_main(move || scene.camera.orbit(TURN / frames, 0.0));
            wait_for_next_frame();
        }
        check_colors(SIDE)?;

        // The same nodes from the new angle, their pixels from the moved
        // camera.
        let (ball_px, monkey_px) = from_main(move || (scene.pixel(BALL_AT), scene.pixel(MONKEY_AT)));
        tap(ball_px);
        check_colors(SIDE_BALL_LIT)?;
        tap(monkey_px);
        check_colors(SIDE_MONKEY_BACK)?;

        let (touched, taps) = from_main(move || (scene.touched.clone(), scene.taps.clone()));

        let hit_tags: Vec<u32> = touched.iter().map(|(tag, _)| *tag).collect();
        anyhow::ensure!(
            hit_tags == [CRATE, PEBBLE, MONKEY, CRATE, BALL, MONKEY],
            "the taps touched {hit_tags:?}, expected the crate, the pebble in front of the ball, the monkey, the \
             crate again, then from the side the ball and the monkey"
        );
        anyhow::ensure!(
            taps.len() == 7,
            "{} taps reached the scene, expected 7",
            taps.len()
        );

        let (_, on_crate) = touched[0];
        anyhow::ensure!(
            (on_crate.z - 0.8).abs() < 0.05 && (on_crate.x + 3.0).abs() < 0.9,
            "the crate was hit at {on_crate:?}, expected its front face"
        );
        let (_, on_monkey) = touched[2];
        anyhow::ensure!(
            on_monkey.distance(MONKEY_AT) < 2.0,
            "the monkey was hit at {on_monkey:?}, far from it"
        );
        let (_, on_ball) = touched[4];
        anyhow::ensure!(
            (on_ball.distance(BALL_AT) - BALL_RADIUS).abs() < 0.01,
            "the ball was hit at {on_ball:?}, not on its surface"
        );

        let sky = taps[3];
        anyhow::ensure!(sky.direction.y > 0.0, "the sky tap's ray goes down: {sky:?}");
        anyhow::ensure!(
            sky.origin == Vec3::new(0.0, 4.0, 10.0),
            "a ray starts at the camera"
        );

        Ok(())
    }
}

fn tap(pixel: Point) {
    inject_touches(format!("{} {} b\n{} {} e", pixel.x, pixel.y, pixel.x, pixel.y));
    wait_for_next_frame();
}

const ROW: &str = r"
       4    4 - #597c95
     340    4 - #597c95
     592    4 - #597c95
     456  240 - #d79b61
     432  244 - #ad7c4d
     476  244 - #dda063
     292  252 - #3179ac
     148  256 - #c7541e
     316  260 - #3784bb
     404  260 - #bec2c6
     408  260 - #bec2c6
     412  260 - #bec2c6
     488  260 - #e6b082
     196  264 - #9c431c
     272  264 - #2d6d9c
     328  264 - #3480b6
     484  264 - #e6b082
     488  264 - #e6b082
     508  264 - #7e5b39
     428  268 - #d99f69
     388  272 - #a6774a
     332  276 - #3580b5
     448  276 - #d39b66
     468  276 - #dd9f64
     496  276 - #7b5838
     264  280 - #29628b
     304  280 - #6ad88f
     308  280 - #6bda90
     300  284 - #55cc80
     312  284 - #59d284
     340  284 - #3077a9
     420  284 - #7b5838
     280  288 - #30ab61
     308  288 - #56cd80
     404  288 - #7b5838
     256  292 - #214865
     488  292 - #a97a4c
     256  296 - #214865
     432  296 - #7b5838
      96  300 - #ac491c
     336  300 - #2e71a1
     448  300 - #ab7b4d
     260  304 - #214865
     288  304 - #2d9f5a
     464  304 - #b28050
     184  312 - #9c421c
     324  312 - #2c9a57
     308  316 - #2b9354
     336  316 - #285b82
     284  320 - #237040
     328  320 - #285c83
     452  320 - #976d44
     268  324 - #214865
     300  328 - #237040
     320  328 - #244e6f
     476  332 - #be8955
     288  336 - #214865
     308  336 - #214865
     444  340 - #7b5838
     188  344 - #9c421c
     136  348 - #ac491c
       4  592 - #bec2c5
     332  592 - #bec2c5
     592  592 - #bec2c5
";

const CRATE_LIT: &str = r"
       4    4 - #597c95
     340    4 - #597c95
     592    4 - #597c95
     456  240 - #d79b61
     432  244 - #ad7c4d
     476  244 - #dda063
     292  252 - #3179ac
     180  256 - #ececec
     316  260 - #3784bb
     404  260 - #bec2c6
     408  260 - #bec2c6
     412  260 - #bec2c6
     456  260 - #e8b182
     488  260 - #e6b082
     140  264 - #ececec
     272  264 - #2d6d9c
     328  264 - #3480b6
     484  264 - #e6b082
     488  264 - #e6b082
     508  264 - #7e5b39
     100  268 - #ececec
     428  268 - #d99f69
     388  272 - #a6774a
     332  276 - #3580b5
     448  276 - #d39b66
     468  276 - #dd9f64
     496  276 - #7b5838
     264  280 - #29628b
     304  280 - #6ad88f
     308  280 - #6bda90
     436  280 - #a47649
     300  284 - #55cc80
     312  284 - #59d284
     340  284 - #3077a9
     420  284 - #7b5838
     280  288 - #30ab61
     308  288 - #56cd80
     404  288 - #7b5838
     256  292 - #214865
     488  292 - #a97a4c
     256  296 - #214865
     432  296 - #7b5838
     336  300 - #2e71a1
     448  300 - #ab7b4d
     260  304 - #214865
     288  304 - #2d9f5a
     464  304 - #b28050
     324  312 - #2c9a57
     308  316 - #2b9354
     336  316 - #285b82
     284  320 - #237040
     328  320 - #285c83
     452  320 - #976d44
     268  324 - #214865
     300  328 - #237040
     320  328 - #244e6f
     476  332 - #be8955
     288  336 - #214865
     308  336 - #214865
     444  340 - #7b5838
     460  344 - #7b5838
       4  592 - #bec2c5
     332  592 - #bec2c5
     592  592 - #bec2c5
";

const PEBBLE_LIT: &str = r"
       4    4 - #597c95
     340    4 - #597c95
     592    4 - #597c95
     464  240 - #dda063
     432  244 - #ad7c4d
     292  252 - #3179ac
     184  256 - #ececec
     312  256 - #3582b8
     448  256 - #d49c68
     316  260 - #3784bb
     404  260 - #bec2c6
     408  260 - #bec2c6
     412  260 - #bec2c6
     488  260 - #e6b082
     272  264 - #2d6d9c
     328  264 - #3480b6
     460  264 - #e8b283
     484  264 - #e6b082
     488  264 - #e6b082
     508  264 - #7e5b39
     100  268 - #ececec
     144  268 - #ececec
     428  268 - #d99f69
     264  272 - #29618b
     388  272 - #a6774a
     332  276 - #3580b5
     400  276 - #8c653f
     448  276 - #d39b66
     472  276 - #bc8855
     264  280 - #29628b
     268  284 - #b1b1b1
     488  284 - #c68f5a
     504  284 - #ab7b4d
     416  288 - #d09761
     264  292 - #8b8b8b
     284  292 - #d7d7d7
     340  292 - #2f74a5
     256  296 - #214865
     288  296 - #d7d7d7
     432  296 - #7b5838
     272  300 - #a5a5a5
     480  300 - #c38d58
     452  304 - #7b5838
     280  308 - #a8a8a8
     336  308 - #2b6995
     264  312 - #214865
     468  312 - #8d6640
     332  316 - #296089
     280  320 - #8b8b8b
     292  320 - #919191
     312  320 - #a6a6a6
     268  324 - #214865
     304  324 - #8b8b8b
     324  324 - #26577b
     456  324 - #a8794b
     272  328 - #214865
     476  332 - #be8955
     288  336 - #214865
     308  336 - #214865
     444  340 - #7b5838
     460  344 - #7b5838
       4  592 - #bec2c5
     264  592 - #bec2c5
     592  592 - #bec2c5
";

const MONKEY_LIT: &str = r"
       4    4 - #597c95
     340    4 - #597c95
     592    4 - #597c95
     172   72 - #597c95
     472  240 - #f0f0f0
     292  252 - #3179ac
     184  256 - #ececec
     304  256 - #3581b8
     284  260 - #3179ac
     316  260 - #3784bb
     416  260 - #8b8b8b
     512  260 - #e3e3e3
     272  264 - #2d6d9c
     328  264 - #3480b6
     396  264 - #8b8b8b
     448  264 - #d3d3d3
     100  268 - #ececec
     144  268 - #ececec
     428  268 - #eeeeee
     488  268 - #9e9e9e
     264  272 - #29618b
     388  272 - #bcbcbc
     400  272 - #cbcbcb
     468  272 - #d9d9d9
     332  276 - #3580b5
     500  276 - #8b8b8b
     264  280 - #29628b
     404  280 - #acacac
     432  280 - #8b8b8b
     468  284 - #acacac
     260  288 - #265579
     416  288 - #e9e9e9
     492  288 - #d4d4d4
     256  292 - #214865
     264  292 - #8b8b8b
     340  292 - #2f74a5
     436  292 - #a7a7a7
     448  292 - #aeaeae
     256  296 - #214865
     464  296 - #e8e8e8
     480  300 - #dedede
     452  304 - #8b8b8b
     336  308 - #2b6995
     260  312 - #214865
     288  312 - #aeaeae
     468  312 - #a0a0a0
     332  316 - #296089
     448  316 - #afafaf
     268  324 - #214865
     296  324 - #8b8b8b
     328  324 - #255376
     272  328 - #214865
     308  328 - #8b8b8b
     448  328 - #a0a0a0
     300  332 - #214865
     320  332 - #214865
     292  336 - #214865
     452  340 - #a0a0a0
     476  340 - #a4a4a4
       4  416 - #bec2c5
     168  492 - #bec2c5
       4  592 - #bec2c5
     332  592 - #bec2c5
     592  592 - #bec2c5
";

const SKY_TAPPED: &str = r"
       4    4 - #597c95
     340    4 - #597c95
     592    4 - #597c95
     172   72 - #597c95
     472  240 - #f0f0f0
     292  252 - #3179ac
     184  256 - #ececec
     304  256 - #3581b8
     284  260 - #3179ac
     316  260 - #3784bb
     416  260 - #8b8b8b
     512  260 - #e3e3e3
     272  264 - #2d6d9c
     328  264 - #3480b6
     396  264 - #8b8b8b
     448  264 - #d3d3d3
     100  268 - #ececec
     144  268 - #ececec
     428  268 - #eeeeee
     488  268 - #9e9e9e
     264  272 - #29618b
     388  272 - #bcbcbc
     400  272 - #cbcbcb
     468  272 - #d9d9d9
     332  276 - #3580b5
     500  276 - #8b8b8b
     264  280 - #29628b
     404  280 - #acacac
     432  280 - #8b8b8b
     468  284 - #acacac
     260  288 - #265579
     416  288 - #e9e9e9
     492  288 - #d4d4d4
     256  292 - #214865
     264  292 - #8b8b8b
     340  292 - #2f74a5
     436  292 - #a7a7a7
     448  292 - #aeaeae
     256  296 - #214865
     464  296 - #e8e8e8
     480  300 - #dedede
     452  304 - #8b8b8b
     336  308 - #2b6995
     260  312 - #214865
     288  312 - #aeaeae
     468  312 - #a0a0a0
     332  316 - #296089
     448  316 - #afafaf
     268  324 - #214865
     296  324 - #8b8b8b
     328  324 - #255376
     272  328 - #214865
     308  328 - #8b8b8b
     448  328 - #a0a0a0
     300  332 - #214865
     320  332 - #214865
     292  336 - #214865
     452  340 - #a0a0a0
     476  340 - #a4a4a4
       4  416 - #bec2c5
     168  492 - #bec2c5
       4  592 - #bec2c5
     332  592 - #bec2c5
     592  592 - #bec2c5
";

const CRATE_BACK: &str = r"
       4    4 - #597c95
     264    4 - #597c95
     592    4 - #597c95
     428   64 - #597c95
     444  240 - #ebebeb
     484  244 - #f0f0f0
     304  252 - #337eb3
     140  256 - #c7541e
     280  256 - #2f71a1
     292  260 - #e8e8e8
     316  260 - #3784bb
     416  260 - #8b8b8b
     512  260 - #e3e3e3
     272  264 - #2d6d9c
     328  264 - #3480b6
     396  264 - #8b8b8b
     448  264 - #d3d3d3
     100  268 - #c7541e
     268  268 - #2b6995
     488  268 - #9e9e9e
     180  272 - #9c421c
     388  272 - #bcbcbc
     468  272 - #d9d9d9
     332  276 - #3580b5
     500  276 - #8b8b8b
     264  280 - #29628b
     404  280 - #acacac
     432  280 - #8b8b8b
     196  288 - #9c431c
     304  288 - #efefef
     416  288 - #e9e9e9
     492  288 - #d4d4d4
     256  292 - #214865
     264  292 - #8b8b8b
     340  292 - #2f74a5
     436  292 - #a7a7a7
     448  292 - #aeaeae
     256  296 - #214865
     464  296 - #e8e8e8
     480  300 - #dedede
     196  308 - #9c431c
     336  308 - #2b6995
     144  312 - #ac491c
     260  312 - #214865
     288  312 - #aeaeae
     468  312 - #a0a0a0
     332  316 - #296089
     448  316 - #afafaf
     268  324 - #214865
     284  324 - #8b8b8b
     328  324 - #255376
     272  328 - #214865
     308  328 - #8b8b8b
     320  328 - #244e6f
     300  332 - #214865
     292  336 - #214865
     452  340 - #a0a0a0
     476  340 - #a4a4a4
     188  344 - #9c421c
     104  348 - #ac491c
     168  524 - #bec2c5
       4  592 - #bec2c5
     332  592 - #bec2c5
     592  592 - #bec2c5
";

const SIDE: &str = r"
       4    4 - #597c95
     240    4 - #597c95
     592    4 - #597c95
     416   48 - #597c95
       4  196 - #597c95
     344  216 - #eeeeee
     324  220 - #8f8f8f
     332  220 - #e2e2e2
     340  220 - #eeeeee
     352  224 - #ebebeb
     356  224 - #ebebeb
     328  232 - #afafaf
     344  232 - #c0c0c0
     352  232 - #c0c0c0
     368  232 - #b5b5b5
     316  236 - #8b8b8b
     336  236 - #d4d4d4
     360  236 - #ebebeb
     376  236 - #8b8b8b
     368  240 - #eaeaea
     316  244 - #c8c8c8
     320  244 - #c8c8c8
     352  244 - #d4d4d4
     360  244 - #d2d2d2
     368  244 - #eaeaea
     336  248 - #8b8b8b
     316  252 - #cacaca
     320  252 - #b8b8b8
     368  252 - #8b8b8b
     276  260 - #2a638d
     316  260 - #4b88b9
     324  264 - #4a88bb
     376  264 - #dddddd
     312  268 - #6590b9
     316  268 - #759dc4
     320  268 - #6f9ac3
     344  268 - #8b8b8b
     316  272 - #6c94bb
     320  272 - #6e96bf
     360  272 - #8b8b8b
     292  276 - #2b618a
     260  284 - #214865
     372  284 - #d0d0d0
     216  288 - #c75520
     336  288 - #3173a3
     364  288 - #8f8f8f
     316  292 - #2d638c
     360  292 - #8b8b8b
     300  304 - #214865
     328  308 - #26587c
     256  312 - #c75520
     156  316 - #c7551f
     204  328 - #733219
     304  336 - #214865
     592  360 - #bfc3c6
     172  368 - #733219
     236  376 - #733219
     288  400 - #ac491c
     160  416 - #733219
     228  432 - #733219
       4  592 - #bec2c5
     176  592 - #bec2c5
     380  592 - #bec2c6
     592  592 - #bfc2c6
";

const SIDE_BALL_LIT: &str = r"
       4    4 - #597c95
     236    4 - #597c95
     592    4 - #597c95
     416   52 - #597c95
     548  180 - #597c95
       4  196 - #597c95
     324  220 - #8f8f8f
     352  224 - #ebebeb
     356  224 - #ebebeb
     320  232 - #8b8b8b
     328  232 - #afafaf
     344  232 - #c0c0c0
     352  232 - #c0c0c0
     316  236 - #8b8b8b
     372  236 - #d7d7d7
     376  236 - #8b8b8b
     368  240 - #eaeaea
     316  244 - #c8c8c8
     320  244 - #c8c8c8
     352  244 - #d4d4d4
     368  244 - #eaeaea
     316  252 - #cacaca
     320  252 - #b8b8b8
     364  252 - #ababab
     368  252 - #8b8b8b
     372  252 - #b5b5b5
     284  256 - #d5d5d5
     300  256 - #e9e9e9
     292  264 - #d3d3d3
     312  264 - #ededed
     328  264 - #f1f1f1
     376  264 - #dddddd
     344  268 - #8b8b8b
     300  272 - #d1d1d1
     320  276 - #e8e8e8
     280  284 - #949494
     288  284 - #a4a4a4
     256  288 - #8b8b8b
     324  288 - #d2d2d2
     364  288 - #8f8f8f
     200  292 - #c75520
     300  292 - #a6a6a6
     340  292 - #dbdbdb
     312  300 - #a6a6a6
     324  308 - #a5a5a5
     332  312 - #a6a6a6
     300  316 - #8b8b8b
     152  320 - #733219
     228  324 - #c75520
     312  336 - #8b8b8b
     264  344 - #ac491c
     592  356 - #bfc3c6
     220  364 - #733219
     168  368 - #733219
     296  388 - #ac491c
     252  392 - #733219
     212  412 - #733219
     164  420 - #733219
     260  436 - #ac491c
     452  440 - #bfc2c6
       4  592 - #bec2c5
     184  592 - #bec2c5
     388  592 - #bec2c6
     592  592 - #bfc2c6
";

const SIDE_MONKEY_BACK: &str = r"
       4    4 - #597c95
     240    4 - #597c95
     592    4 - #597c95
     416   52 - #597c95
       4  196 - #597c95
     348  216 - #dea166
     324  220 - #7d5a39
     328  220 - #c78f59
     340  220 - #d89d64
     352  224 - #d39b66
     356  224 - #d39b65
     320  232 - #7b5838
     324  232 - #9a6f45
     360  232 - #d39a65
     316  236 - #7b5838
     336  236 - #bb8857
     372  236 - #be8d60
     376  236 - #7b5838
     356  240 - #be8956
     368  240 - #d5a57d
     320  244 - #b07f50
     312  248 - #916841
     316  252 - #b28151
     372  252 - #9f7247
     288  256 - #dbdbdb
     344  256 - #7b5838
     364  256 - #936a42
     332  260 - #7b5838
     356  260 - #7b5838
     268  268 - #a2a2a2
     320  268 - #f0f0f0
     368  268 - #8d6640
     304  272 - #d7d7d7
     352  272 - #7b5838
     372  276 - #aa7b4c
     280  280 - #9f9f9f
     364  280 - #8d6540
     324  284 - #dadada
     372  284 - #b78352
     256  288 - #8b8b8b
     200  292 - #c75520
     272  292 - #8b8b8b
     292  292 - #979797
     340  292 - #dbdbdb
     360  292 - #7b5838
     308  296 - #aaaaaa
     308  312 - #8b8b8b
     336  316 - #9f9f9f
     152  320 - #733219
     232  324 - #c75520
     320  332 - #8b8b8b
     300  336 - #8b8b8b
     592  352 - #bfc3c6
     264  360 - #ac491c
     220  364 - #733219
     168  368 - #733219
     296  388 - #ac491c
     212  412 - #733219
     164  420 - #733219
     260  436 - #ac491c
       4  592 - #bec2c5
     184  592 - #bec2c5
     388  592 - #bec2c6
     592  592 - #bfc2c6
";
