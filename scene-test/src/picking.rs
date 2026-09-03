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
     452  240 - #d79b61
     472  240 - #dda063
     432  244 - #ad7c4d
     136  252 - #90685a
     168  252 - #90685a
     292  252 - #3179ac
     196  260 - #a56346
     316  260 - #3784bb
     404  260 - #bec2c5
     408  260 - #bec2c5
     412  260 - #bec2c5
     456  260 - #e8b182
     488  260 - #e6b082
     512  260 - #c8905a
     104  264 - #c7541e
     272  264 - #2d6e9c
     328  264 - #3480b6
     428  264 - #d99f69
     484  264 - #e6b082
     300  276 - #4ecf7e
     312  276 - #4dd27e
     332  276 - #3580b5
     396  276 - #8c653f
     440  276 - #7b5838
     264  280 - #29628c
     304  280 - #6ad88f
     308  280 - #6ada8f
     420  280 - #7b5838
     308  284 - #6ad78e
     340  284 - #3077a9
     452  284 - #c08b58
     468  284 - #976d44
     184  288 - #9d431c
     492  288 - #ba8653
     256  292 - #214865
      96  296 - #ac491c
     256  296 - #214865
     432  296 - #7b5838
     448  300 - #ab7b4d
     260  304 - #214865
     288  304 - #2e9f5b
     308  304 - #33b064
     464  304 - #b28050
     144  308 - #ac491c
     324  312 - #2c9a58
     200  316 - #b6a29b
     336  316 - #285c82
     200  320 - #ae8371
     284  320 - #237040
     268  324 - #214865
     300  328 - #237040
     320  328 - #244f70
     472  332 - #bc8754
     288  336 - #214865
     444  336 - #7e5a39
     188  344 - #9d431c
     104  352 - #baa49b
     136  352 - #baa49b
       4  592 - #bec1c5
     332  592 - #bec1c5
     592  592 - #bec1c5
";

const CRATE_LIT: &str = r"
       4    4 - #597c95
     340    4 - #597c95
     592    4 - #597c95
     456  240 - #d79b61
     432  244 - #ad7c4d
     476  244 - #dda063
     292  252 - #3179ac
     136  256 - #ececec
     196  256 - #8b9da9
     316  260 - #3784bb
     404  260 - #bec2c5
     408  260 - #bec2c5
     412  260 - #bec2c5
     488  260 - #e6b082
     112  264 - #ececec
     272  264 - #2d6e9c
     328  264 - #3480b6
     484  264 - #e6b082
     508  264 - #7e5b39
     156  268 - #ececec
     428  268 - #d99f69
     388  272 - #a6774a
     300  276 - #4ecf7e
     312  276 - #4dd27e
     332  276 - #3580b5
     448  276 - #d39b66
     468  276 - #dd9f64
     496  276 - #7b5838
     264  280 - #29628c
     304  280 - #6ad88f
     308  280 - #6ada8f
     308  284 - #6ad78e
     340  284 - #3077a9
     420  284 - #7b5838
     404  288 - #7b5838
     256  292 - #214865
     488  292 - #a97a4c
     256  296 - #214865
     276  296 - #2c9957
     432  296 - #7b5838
     336  300 - #2e71a1
     448  300 - #ab7b4d
     260  304 - #214865
     288  304 - #2e9f5b
     308  304 - #33b064
     464  304 - #b28050
     324  312 - #2c9a58
     144  316 - #d0d0d0
     336  316 - #285c82
     284  320 - #237040
     328  320 - #285d84
     452  320 - #976d44
     268  324 - #214865
     300  328 - #237040
     320  328 - #244f70
     476  332 - #be8955
     288  336 - #214865
     308  336 - #214865
     444  340 - #7b5838
     104  348 - #d0d0d0
     184  348 - #d0d0d0
       4  592 - #bec1c5
     332  592 - #bec1c5
     592  592 - #bec1c5
";

const PEBBLE_LIT: &str = r"
       4    4 - #597c95
     340    4 - #597c95
     592    4 - #597c95
     464  240 - #dda063
     432  244 - #ad7c4d
     292  252 - #3179ac
     196  256 - #8b9da9
     312  256 - #3582b8
     448  256 - #d49c68
     316  260 - #3784bb
     404  260 - #bec2c5
     408  260 - #bec2c5
     412  260 - #bec2c5
     488  260 - #e6b082
     112  264 - #ececec
     272  264 - #2d6e9c
     328  264 - #3480b6
     460  264 - #e8b283
     484  264 - #e6b082
     488  264 - #e6b082
     508  264 - #7e5b39
     156  268 - #ececec
     428  268 - #d99f69
     264  272 - #29628b
     388  272 - #a6774a
     332  276 - #3580b5
     400  276 - #8c653f
     448  276 - #d39b66
     472  276 - #bc8855
     264  280 - #29628c
     268  284 - #b2b2b2
     488  284 - #c68f5a
     504  284 - #ab7b4d
     300  288 - #ededed
     416  288 - #d09761
     256  292 - #214865
     264  292 - #8b8b8b
     340  292 - #2f74a5
     256  296 - #214865
     432  296 - #7b5838
     480  300 - #c38d58
     452  304 - #7b5838
     336  308 - #2b6995
     264  312 - #214865
     468  312 - #8d6640
     144  316 - #d0d0d0
     280  316 - #8b8b8b
     332  316 - #29618a
     292  320 - #929292
     268  324 - #214865
     304  324 - #8c8c8c
     324  324 - #26577c
     456  324 - #a8794b
     272  328 - #214865
     476  332 - #be8955
     288  336 - #214865
     308  336 - #214865
     444  340 - #7b5838
     460  344 - #7b5838
     104  348 - #d0d0d0
     184  348 - #d0d0d0
       4  592 - #bec1c5
     272  592 - #bec1c5
     592  592 - #bec1c5
";

const MONKEY_LIT: &str = r"
       4    4 - #597c95
     256    4 - #597c95
     592    4 - #597c95
     424   68 - #597c95
     132  104 - #597c95
     472  240 - #f0f0f0
     300  252 - #337db2
     440  252 - #d6d6d6
     196  256 - #8b9da9
     280  256 - #2f72a2
     316  260 - #3784bb
     424  260 - #eeeeee
     512  260 - #e3e3e3
     148  264 - #ececec
     272  264 - #2d6e9c
     328  264 - #3480b6
     412  264 - #bec2c5
     452  264 - #d3d3d3
     100  268 - #ececec
     428  268 - #eeeeee
     264  272 - #29628b
     388  272 - #bcbcbc
     468  272 - #d9d9d9
     332  276 - #3580b5
     408  276 - #bebebe
     448  276 - #ebebeb
     480  276 - #afafaf
     264  280 - #29628c
     420  280 - #8b8b8b
     436  280 - #bababa
     492  280 - #f0f0f0
     296  284 - #ededed
     336  284 - #327baf
     404  288 - #8b8b8b
     464  288 - #b0b0b0
     256  292 - #214865
     264  292 - #8b8b8b
     340  292 - #2f74a5
     428  292 - #8b8b8b
     488  292 - #c0c0c0
     256  296 - #214865
     448  300 - #c3c3c3
     336  308 - #2b6995
     472  308 - #ebebeb
     264  312 - #214865
     280  316 - #8b8b8b
     296  316 - #adadad
     332  316 - #29618a
     452  316 - #afafaf
     328  320 - #285d84
     268  324 - #214865
     324  324 - #26577c
     272  328 - #214865
     300  332 - #214865
     288  336 - #214865
     308  336 - #214865
     448  336 - #8f8f8f
     476  340 - #a4a4a4
     460  344 - #8b8b8b
     132  452 - #bec1c5
     428  520 - #bec1c5
       4  592 - #bec1c5
     264  592 - #bec1c5
     592  592 - #bec1c5
";

const SKY_TAPPED: &str = r"
       4    4 - #597c95
     256    4 - #597c95
     592    4 - #597c95
     424   68 - #597c95
     132  104 - #597c95
     472  240 - #f0f0f0
     300  252 - #337db2
     440  252 - #d6d6d6
     196  256 - #8b9da9
     280  256 - #2f72a2
     316  260 - #3784bb
     424  260 - #eeeeee
     512  260 - #e3e3e3
     148  264 - #ececec
     272  264 - #2d6e9c
     328  264 - #3480b6
     412  264 - #bec2c5
     452  264 - #d3d3d3
     100  268 - #ececec
     428  268 - #eeeeee
     264  272 - #29628b
     388  272 - #bcbcbc
     468  272 - #d9d9d9
     332  276 - #3580b5
     408  276 - #bebebe
     448  276 - #ebebeb
     480  276 - #afafaf
     264  280 - #29628c
     420  280 - #8b8b8b
     436  280 - #bababa
     492  280 - #f0f0f0
     296  284 - #ededed
     336  284 - #327baf
     404  288 - #8b8b8b
     464  288 - #b0b0b0
     256  292 - #214865
     264  292 - #8b8b8b
     340  292 - #2f74a5
     428  292 - #8b8b8b
     488  292 - #c0c0c0
     256  296 - #214865
     448  300 - #c3c3c3
     336  308 - #2b6995
     472  308 - #ebebeb
     264  312 - #214865
     280  316 - #8b8b8b
     296  316 - #adadad
     332  316 - #29618a
     452  316 - #afafaf
     328  320 - #285d84
     268  324 - #214865
     324  324 - #26577c
     272  328 - #214865
     300  332 - #214865
     288  336 - #214865
     308  336 - #214865
     448  336 - #8f8f8f
     476  340 - #a4a4a4
     460  344 - #8b8b8b
     132  452 - #bec1c5
     428  520 - #bec1c5
       4  592 - #bec1c5
     264  592 - #bec1c5
     592  592 - #bec1c5
";

const CRATE_BACK: &str = r"
       4    4 - #597c95
     256    4 - #597c95
     592    4 - #597c95
     448  240 - #eeeeee
     484  244 - #f0f0f0
     136  252 - #90685a
     152  252 - #90685a
     168  252 - #90685a
     300  252 - #337db2
     196  256 - #7b6059
     280  256 - #2f72a2
     196  260 - #a56346
     316  260 - #3784bb
     424  260 - #eeeeee
     512  260 - #e3e3e3
     104  264 - #c7541e
     272  264 - #2d6e9c
     328  264 - #3480b6
     412  264 - #bec2c5
     452  264 - #d3d3d3
     428  268 - #eeeeee
     264  272 - #29628b
     388  272 - #bcbcbc
     468  272 - #d9d9d9
     332  276 - #3580b5
     408  276 - #bebebe
     264  280 - #29628c
     448  280 - #e5e5e5
     492  280 - #f0f0f0
     432  284 - #ababab
     184  288 - #9d431c
     404  288 - #8b8b8b
     264  292 - #8b8b8b
     340  292 - #2f74a5
     428  292 - #8b8b8b
     472  292 - #bcbcbc
      96  296 - #ac491c
     256  296 - #214865
     300  296 - #e5e5e5
     144  308 - #ac491c
     184  308 - #9d431c
     336  308 - #2b6995
     448  308 - #8b8b8b
     472  308 - #ebebeb
     264  312 - #214865
     200  316 - #b6a29b
     332  316 - #29618a
     200  320 - #ae8371
     292  320 - #929292
     268  324 - #214865
     324  324 - #26577c
     272  328 - #214865
     288  336 - #214865
     308  336 - #214865
     444  340 - #8b8b8b
     476  340 - #a4a4a4
     188  344 - #9d431c
     460  344 - #8b8b8b
     104  352 - #baa49b
     136  352 - #baa49b
     164  352 - #baa49b
       4  592 - #bec1c5
     264  592 - #bec1c5
     592  592 - #bec1c5
";

const SIDE: &str = r"
       4    4 - #597c95
     240    4 - #597c95
     592    4 - #597c95
       4  204 - #597c95
     328  220 - #e2e2e2
     340  220 - #eeeeee
     356  220 - #ededed
     352  224 - #ebebeb
     324  232 - #afafaf
     348  232 - #c0c0c0
     360  232 - #ebebeb
     380  232 - #bdbdbd
     316  236 - #8b8b8b
     336  236 - #d4d4d4
     372  236 - #d7d7d7
     368  240 - #eaeaea
     316  244 - #c8c8c8
     320  244 - #c8c8c8
     352  244 - #d4d4d4
     360  244 - #d2d2d2
     368  244 - #eaeaea
     256  252 - #8c9fae
     316  252 - #cacaca
     320  252 - #b8b8b8
     336  252 - #8b8b8b
     364  252 - #ababab
     284  256 - #2e6d9c
     360  260 - #8b8b8b
     316  264 - #6292bf
     324  264 - #4a88bb
     376  264 - #dddddd
     312  268 - #6590b9
     316  268 - #759cc4
     320  268 - #6e99c3
     344  268 - #8b8b8b
     452  268 - #8c9fae
     316  272 - #6d95bc
     320  272 - #6e97bf
     292  276 - #2b628a
     368  276 - #a4a4a4
     260  280 - #214865
     336  284 - #3477a8
     372  284 - #d0d0d0
     364  288 - #8f8f8f
     276  292 - #214865
     356  292 - #98999a
     296  296 - #755147
     200  300 - #c75520
     328  300 - #2a628b
     236  312 - #c75520
     300  320 - #214865
     320  332 - #214865
     152  340 - #865644
     260  344 - #ac491c
     200  348 - #733219
     156  388 - #ab9e9a
     216  392 - #733219
     296  392 - #ac491c
     592  400 - #bfc2c6
     184  424 - #733219
     256  440 - #733219
       4  592 - #bec1c5
     388  592 - #bec2c5
     592  592 - #bec2c6
";

const SIDE_BALL_LIT: &str = r"
       4    4 - #597c95
     248    4 - #597c95
     592    4 - #597c95
     420   52 - #597c95
     592  172 - #597c95
       4  204 - #597c95
     324  220 - #8f8f8f
     356  220 - #ededed
     352  224 - #ebebeb
     356  224 - #ebebeb
     328  232 - #afafaf
     344  232 - #c0c0c0
     380  232 - #bdbdbd
     316  236 - #8b8b8b
     372  236 - #d7d7d7
     368  240 - #eaeaea
     316  244 - #c8c8c8
     320  244 - #c8c8c8
     352  244 - #d4d4d4
     368  244 - #eaeaea
     256  252 - #8c9fae
     300  252 - #ececec
     316  252 - #cacaca
     320  252 - #b8b8b8
     372  252 - #b5b5b5
     284  256 - #d6d6d6
     344  256 - #8b8b8b
     364  256 - #a7a7a7
     376  264 - #dddddd
     304  268 - #dfdfdf
     328  268 - #efefef
     452  268 - #8c9fae
     368  276 - #a4a4a4
     340  280 - #ececec
     288  284 - #a5a5a5
     316  284 - #d1d1d1
     372  284 - #d0d0d0
     216  288 - #c75520
     260  288 - #8b8b8b
     364  288 - #8f8f8f
     296  296 - #ae755b
     304  296 - #a4a4a4
     336  296 - #d0d0d0
     176  308 - #c75520
     324  308 - #a6a6a6
     256  316 - #c75520
     336  316 - #a0a0a0
     320  332 - #8b8b8b
     204  336 - #733219
     300  336 - #8b8b8b
     152  340 - #865644
     248  360 - #733219
     208  384 - #733219
     156  388 - #ab9e9a
     296  392 - #ac491c
     260  400 - #ac491c
     592  400 - #bfc2c6
     200  428 - #733219
       4  432 - #bec2c5
     256  440 - #733219
       4  592 - #bec1c5
     172  592 - #bec2c5
     388  592 - #bec2c5
     592  592 - #bec2c6
";

const SIDE_MONKEY_BACK: &str = r"
       4    4 - #597c95
     236    4 - #597c95
     592    4 - #597c95
     416   48 - #597c95
       4  204 - #597c95
     344  216 - #d89d64
     324  220 - #7d5a39
     332  220 - #c78f59
     356  220 - #d79d65
     336  224 - #ae7e50
     352  224 - #d39b66
     356  224 - #d39b65
     320  232 - #7b5838
     360  232 - #d39a65
     316  236 - #7b5838
     336  236 - #bb8857
     348  236 - #a97b4f
     372  236 - #be8d60
     368  240 - #d5a57d
     320  244 - #b07f50
     356  244 - #ba8654
     380  244 - #a08163
     312  248 - #916841
     336  248 - #7b5838
     256  252 - #8c9fae
     292  252 - #e6e6e6
     372  252 - #9f7247
     320  256 - #f1f1f1
     348  260 - #7b5838
     300  264 - #dfdfdf
     336  264 - #7b5838
     268  268 - #a3a3a3
     452  268 - #8c9fae
     356  272 - #7b5838
     308  276 - #d7d7d7
     372  276 - #aa7b4c
     284  280 - #a7a7a7
     216  288 - #c75520
     328  288 - #d7d7d7
     272  292 - #8b8b8b
     360  292 - #7b5838
     296  296 - #ae755b
     316  304 - #a3a3a3
     336  312 - #acacac
     240  320 - #c75520
     300  320 - #8b8b8b
     200  332 - #733219
     320  332 - #8b8b8b
     304  336 - #8b8b8b
     152  340 - #865644
     276  340 - #ac491c
     152  344 - #ab9e9a
     236  364 - #733219
     196  376 - #733219
     156  388 - #ab9e9a
     296  392 - #ac491c
     260  400 - #ac491c
     592  400 - #bfc2c6
     200  428 - #733219
     256  440 - #733219
       4  592 - #bec1c5
     172  592 - #bec2c5
     388  592 - #bec2c5
     592  592 - #bec2c6
";
