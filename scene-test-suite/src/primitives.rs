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
     268    4 - #597c95
     500    4 - #597c95
     592    4 - #597c95
     356    8 - #597c95
     136   52 - #597c95
     428   60 - #597c95
       4  120 - #597c95
     280  128 - #597c95
     372  136 - #597c95
     492  140 - #597c95
     592  148 - #597c95
     104  196 - #e9c24e
     120  196 - #e8c14d
     132  196 - #e8c14c
     144  196 - #e8c14a
     156  196 - #e8c049
     172  196 - #e8c047
     112  200 - #e9c14d
     164  200 - #e8c048
      80  204 - #e9c24f
      96  204 - #e9c24e
     128  204 - #e8c14b
     140  204 - #e8c14a
     156  204 - #e8c048
     416  224 - #db5347
     380  228 - #dc5449
     440  228 - #db5246
     348  232 - #dc554a
     456  232 - #db5246
     188  236 - #599ad2
     192  236 - #5196ce
     196  236 - #4a92ca
     200  236 - #448fc9
     204  236 - #408ec9
     472  236 - #db5245
     152  240 - #846b1b
     188  240 - #6b9fd0
     192  240 - #5a95c8
     196  240 - #4d8ec3
     200  240 - #4489bf
     204  240 - #3f86bd
     208  240 - #3b85bc
     212  240 - #3a85bd
     396  240 - #db5348
     188  244 - #6b9ac8
     192  244 - #588fbf
     196  244 - #4b87b9
     204  244 - #3c80b4
     208  244 - #397eb2
     212  244 - #377db2
     216  244 - #367db2
     220  244 - #357eb4
     372  244 - #dc5449
     488  244 - #db5145
      84  248 - #846b1b
     212  248 - #3476a8
     216  248 - #3376a8
     220  248 - #3276a8
     224  248 - #3277aa
     440  248 - #db5246
     456  248 - #db5246
     228  252 - #3071a1
     408  252 - #db5347
     424  252 - #db5347
     208  256 - #2f6995
     216  256 - #2d6793
     224  256 - #2c6894
     232  256 - #2d6b98
     124  260 - #846b1b
     196  260 - #2f6690
     212  260 - #2b618a
     220  260 - #2a6089
     228  260 - #2a618a
     236  260 - #2c6590
     188  264 - #2d628a
     204  264 - #295b82
     236  264 - #285c83
     592  264 - #597c95
     356  272 - #87322a
      96  280 - #846b1b
     156  280 - #846b1b
     188  280 - #255478
     208  280 - #255478
     236  280 - #255478
     460  284 - #7e2f28
     224  288 - #255478
     396  288 - #87322a
     196  292 - #255478
     244  292 - #255478
     492  300 - #7e2f28
     220  304 - #255478
     368  304 - #87322a
     192  308 - #255478
     336  308 - #87322a
     144  312 - #846b1b
     236  312 - #255478
     440  312 - #7e2f28
      92  316 - #846b1b
     208  316 - #255478
     192  324 - #255478
     224  328 - #255478
     400  328 - #7e2f28
     200  340 - #255478
     348  340 - #87312a
     484  340 - #7e2f28
     376  360 - #87312a
     452  360 - #7e2f28
     136  364 - #846b1b
     412  368 - #7e2f28
       4  372 - #98bd98
     176  372 - #846b1b
     100  380 - #846b1b
     144  404 - #846b1b
     184  412 - #846b1b
     108  424 - #846b1b
     592  428 - #8ab38a
     388  456 - #8db58d
     300  464 - #91b891
       4  476 - #9dc19d
     216  496 - #98bd98
     548  508 - #8bb38b
     456  516 - #8cb48c
      84  524 - #a1c4a1
     164  572 - #a1c4a1
       4  592 - #a2c5a2
     320  592 - #93ba93
     592  592 - #8ab38a
";

const PRIMITIVES: &str = r"
       4    4 - #597c95
     104    4 - #597c95
     232    4 - #597c95
     428    4 - #597c95
     592    4 - #597c95
     332   48 - #597c95
     508   92 - #597c95
     132  100 - #597c95
     256  120 - #597c95
       4  148 - #597c95
     392  164 - #e3b925
     368  168 - #846b1b
     404  172 - #c4a023
     420  172 - #c4a023
     368  180 - #846b1b
     384  180 - #c4a023
     592  184 - #597c95
     368  188 - #846b1b
     404  188 - #c4a023
     504  188 - #597c95
     380  192 - #c4a023
     368  196 - #846b1b
     368  204 - #846b1b
     388  204 - #c4a022
     416  208 - #c4a022
     368  212 - #846b1b
     380  216 - #c4a022
     400  216 - #c4a022
     368  220 - #846b1b
     192  224 - #da4d40
     224  224 - #da4d40
     416  224 - #c4a022
     156  228 - #da4d40
     384  228 - #c4a022
     368  232 - #846b1b
     120  236 - #7e2f28
     400  236 - #388ac4
     256  240 - #da4d40
     364  240 - #846b1b
     380  240 - #c4a022
     420  240 - #3d97d7
     440  244 - #3d97d8
     132  248 - #7e2f28
     368  248 - #846b1b
     364  252 - #846b1b
     396  252 - #3a8ecb
     120  256 - #7e2f28
     216  256 - #c6473b
     364  256 - #846b1b
     380  256 - #3480b7
     432  256 - #4a9fdf
     452  256 - #3d98d8
     416  260 - #539fdb
     424  260 - #5ba4e1
     140  264 - #7e2f28
     420  264 - #65a8e1
     436  264 - #51a1df
     128  268 - #7e2f28
     180  268 - #c6473b
     244  268 - #c6473b
     364  268 - #2b6691
     412  268 - #529bd6
     424  268 - #72afe6
     392  272 - #3887c0
     420  272 - #69a8df
     432  272 - #62a6e1
     440  272 - #4c9ddb
     416  276 - #559bd5
     424  276 - #63a4dd
     464  276 - #398dca
     124  280 - #7e2f28
     428  280 - #559dd7
     140  284 - #7e2f28
     356  284 - #255478
     376  284 - #2f71a2
     400  288 - #3783bb
     260  292 - #c6473b
     384  292 - #3074a5
     208  296 - #c6473b
     356  296 - #255478
     368  296 - #275a80
     444  296 - #398cc7
     136  300 - #7e2f28
     392  300 - #3074a5
     424  300 - #3886be
     464  300 - #337eb4
     380  304 - #2a638e
       4  308 - #597c95
     360  308 - #255478
     400  308 - #2f72a3
     436  308 - #3581b8
     572  312 - #89b289
     144  316 - #7e2f28
     376  316 - #255478
     412  316 - #2e71a1
     456  316 - #2f71a2
     128  320 - #7e2f28
     180  320 - #c6473b
     236  320 - #c5463b
     388  320 - #255579
     424  320 - #2e71a1
     444  320 - #2f73a4
     404  324 - #295f88
     372  328 - #255478
     416  328 - #29618a
     144  332 - #7e2f28
     384  332 - #255478
     440  332 - #27597f
     408  336 - #255478
     392  340 - #255478
     420  340 - #255478
     148  348 - #7e2f28
     260  348 - #c5463b
     204  352 - #c5463b
     152  364 - #7e2f28
     152  368 - #7e2f28
     592  416 - #89b289
       4  432 - #89b289
     100  456 - #89b289
     224  456 - #89b289
     328  460 - #89b289
     452  500 - #89b289
     552  504 - #89b289
     160  536 - #89b289
       4  592 - #89b289
     316  592 - #89b289
     492  592 - #89b289
     592  592 - #89b289
";
