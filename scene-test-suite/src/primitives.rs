use std::f32::consts::TAU;

use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::{
        LossyConvert,
        volume::{Quat, Shape3, Vec3},
    },
    refs::Weak,
    scene::{Camera, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, scene},
    ui::Color,
    ui_test::{capture_screenshot, check_colors, set_record_probe_count},
};

/// Frames of one full turn of the camera around the scene.
const TURN_FRAMES: usize = 120;

/// A box, a ball and a plane under the fixed sun. The camera makes one
/// full turn around them, so every primitive, its winding, the depth
/// band and the shade of each face are seen from every side. The tall
/// box behind the ball is the depth test, the ball must cover it.
#[scene]
#[derive(Default)]
struct Primitives {}

impl SceneSetup for Primitives {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 4.0, 9.0),
            target: Vec3::new(0.0, 0.8, 0.0),
            ..Camera::default()
        };

        self.make_node::<Prop>(Shape3::Plane(12.0), Vec3::ZERO)
            .set_color(Color::hex("#8fbc8f"));
        self.make_node::<Prop>(Shape3::cube(2.0), Vec3::new(-2.0, 1.0, 0.0))
            .set_color(Color::hex("#e74c3c"))
            .set_rotation(Quat::from_rotation_y(0.5));
        self.make_node::<Prop>(Shape3::Ball(1.0), Vec3::new(2.0, 1.0, 0.0))
            .set_color(Color::hex("#3498db"));
        self.make_node::<Prop>(Shape3::cuboid(1.0, 3.0, 1.0), Vec3::new(2.0, 1.5, -2.5))
            .set_color(Color::hex("#f1c40f"));
    }
}

impl SceneTest for Primitives {
    fn perform_test(mut scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);

        // One full turn around the target, a fixed step per frame, so the
        // camera lands back where it started and the probes below see
        // the same view on every machine.
        for frame in 0..TURN_FRAMES {
            from_main(move || scene.camera.orbit(TAU / TURN_FRAMES.lossy_convert(), 0.0));
            wait_for_next_frame();
            if frame == TURN_FRAMES / 2 {
                check_colors(HALF_TURN)?;
            }
        }

        capture_screenshot()?;
        check_colors(PRIMITIVES)
    }
}

const HALF_TURN: &str = r"
       4    4 - #597c95
     280    4 - #597c95
     368    4 - #597c95
     504    4 - #597c95
     592    4 - #597c95
     144   52 - #597c95
     436   60 - #597c95
       4  120 - #597c95
     276  128 - #597c95
     372  132 - #597c95
     492  140 - #597c95
     592  148 - #597c95
     108  196 - #eac34e
     120  196 - #eac34d
     140  196 - #eac24b
     152  196 - #e9c249
     172  196 - #e9c147
      96  200 - #ebc34e
     164  200 - #e9c148
      80  204 - #ebc34f
     112  204 - #eac34d
     128  204 - #eac24b
     144  204 - #e9c24a
     156  204 - #e9c148
     400  224 - #7a7282
     428  224 - #db5347
     380  228 - #dc5449
     344  232 - #dc564a
     364  232 - #dc5549
     444  232 - #db5246
     188  236 - #599ad2
     192  236 - #5196ce
     196  236 - #4a92ca
     200  236 - #448fc9
     204  236 - #408ec9
     416  236 - #db5347
     464  236 - #db5246
     148  240 - #846b1b
     184  240 - #827b4b
     188  240 - #6b9fd0
     192  240 - #5a95c8
     196  240 - #4d8ec3
     200  240 - #4489bf
     204  240 - #3f86bd
     208  240 - #3b85bc
     212  240 - #3a85bd
     188  244 - #6b9ac8
     192  244 - #588fbf
     196  244 - #4b87b9
     204  244 - #3c80b4
     208  244 - #397eb2
     212  244 - #377db2
     216  244 - #367db2
     220  244 - #357eb4
     376  244 - #dc5448
     488  244 - #db5145
      84  248 - #846b1b
     212  248 - #3476a8
     216  248 - #3376a8
     220  248 - #3276a8
     224  248 - #3277aa
     392  248 - #db5448
     440  248 - #db5246
     228  252 - #3071a1
     408  252 - #db5347
     424  252 - #db5347
     208  256 - #2f6995
     216  256 - #2d6793
     224  256 - #2c6894
     232  256 - #2d6b98
     196  260 - #2f6690
     212  260 - #2b618a
     220  260 - #2a6089
     228  260 - #2a618a
     124  264 - #846b1b
     188  264 - #2d628a
     204  264 - #295b82
     236  264 - #285c83
     156  276 - #846b1b
     368  276 - #87322a
      92  280 - #846b1b
     188  280 - #255478
     208  280 - #255478
     236  280 - #255478
     400  280 - #803029
     452  280 - #7e2f28
     224  288 - #255478
     336  288 - #87322a
     196  292 - #255478
     244  292 - #255478
     480  296 - #7e2f28
     424  300 - #7e2f28
     220  304 - #255478
     192  308 - #255478
     380  308 - #87322a
     148  312 - #846b1b
     236  312 - #255478
      92  316 - #846b1b
     208  316 - #255478
     344  316 - #87322a
     592  316 - #597c95
     192  324 - #255478
     224  328 - #255478
     444  336 - #7e2f28
     200  340 - #255478
     352  344 - #87312a
     388  344 - #87312a
     484  352 - #7e2f28
     136  364 - #846b1b
     416  368 - #7e2f28
       4  372 - #98bd98
     176  372 - #846b1b
     100  380 - #846b1b
     144  404 - #846b1b
     184  412 - #846b1b
     108  424 - #846b1b
     592  448 - #8ab38a
     304  464 - #91b891
       4  476 - #9dc19d
     220  500 - #98bd98
     456  508 - #8cb48c
      84  524 - #a1c4a1
     164  572 - #a1c4a1
     504  584 - #8cb48c
       4  592 - #a2c5a2
     320  592 - #93ba93
     416  592 - #8eb68e
     592  592 - #8ab38a
";

const PRIMITIVES: &str = r"
       4    4 - #597c95
      96    4 - #597c95
     224    4 - #597c95
     388    4 - #597c95
     592    4 - #597c95
     492   72 - #597c95
     124   92 - #597c95
     264  116 - #597c95
       4  152 - #597c95
     504  160 - #597c95
     412  164 - #e3ba25
     372  168 - #94781d
     392  168 - #c4a023
     372  172 - #a4861f
     592  172 - #597c95
     372  176 - #a4861f
     372  180 - #b49321
     416  184 - #c4a023
     368  188 - #846b1b
     396  188 - #c4a023
     368  196 - #846b1b
     384  200 - #c4a022
     368  204 - #846b1b
     400  204 - #c4a022
     416  204 - #c4a022
     368  212 - #846b1b
     392  216 - #c4a022
     364  220 - #6f7458
     412  220 - #c4a022
     216  224 - #da4d40
     140  228 - #797080
     368  228 - #846b1b
     388  232 - #c4a022
     244  236 - #da4d40
     364  236 - #846b1b
     408  236 - #3a8fcd
     368  240 - #846b1b
     424  240 - #3d98d8
     120  244 - #7e2f28
     192  244 - #da4d40
     364  244 - #846b1b
     376  244 - #c4a022
     368  248 - #846b1b
     444  248 - #3e99da
     364  252 - #846b1b
     364  256 - #846b1b
     420  256 - #4e9edd
     432  256 - #4a9fdf
     396  260 - #3b8dc9
     136  264 - #7e2f28
     416  264 - #5ba1dc
     436  264 - #51a1df
     456  264 - #3d96d6
     224  268 - #c6473b
     412  268 - #529bd6
     424  268 - #73afe7
     176  272 - #c6473b
     364  272 - #2b6590
     420  272 - #69a8df
     432  272 - #62a6e1
     440  272 - #4c9ddb
     416  276 - #559bd5
     424  276 - #63a4dd
     124  280 - #7e2f28
     368  280 - #2c6996
     380  280 - #3178ac
     428  280 - #559dd7
     144  284 - #90352d
     256  284 - #c6473b
     356  284 - #255478
     464  284 - #388bc6
       4  292 - #597c95
     368  292 - #295f87
     380  292 - #2e70a0
     400  292 - #3580b7
     448  292 - #3a8eca
     140  296 - #7e2f28
     200  296 - #c6473b
     356  296 - #255478
     416  296 - #3986be
     128  300 - #7e2f28
     172  300 - #c6473b
     432  300 - #3888c1
     372  304 - #26567b
     384  304 - #2c6895
     144  308 - #7e2f28
     360  308 - #255478
     396  308 - #2e6f9f
     408  312 - #2f73a4
     460  312 - #3073a4
     572  312 - #89b289
     128  320 - #7e2f28
     228  320 - #c6463b
     368  320 - #255478
     388  320 - #255579
     432  320 - #2f73a4
     448  320 - #2f71a2
     148  328 - #7e2f28
     184  328 - #c6473b
     400  328 - #255478
     420  328 - #2a638d
     136  332 - #7e2f28
     264  332 - #989776
     380  332 - #255478
     440  332 - #27597f
     264  336 - #989776
     264  340 - #a77c62
     392  340 - #255478
     428  340 - #255478
     264  344 - #a77c62
     408  344 - #578381
     412  344 - #578381
     140  348 - #7e2f28
     236  348 - #c5463b
     208  352 - #c5463b
     152  364 - #7e2f28
     152  368 - #7e2f28
     592  420 - #89b289
       4  432 - #89b289
     224  452 - #89b289
     104  460 - #89b289
     328  460 - #89b289
     456  496 - #89b289
     168  540 - #89b289
       4  592 - #89b289
     332  592 - #89b289
     488  592 - #89b289
     592  592 - #89b289
";
