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
     284    4 - #597c95
     372    4 - #597c95
     592    4 - #597c95
     504    8 - #597c95
     144   48 - #597c95
     436   64 - #597c95
       4  120 - #597c95
     276  128 - #597c95
     372  132 - #597c95
     496  140 - #597c95
     592  148 - #597c95
     108  196 - #e1b70d
     120  196 - #e1b70d
     140  196 - #e1b70d
     152  196 - #e1b70d
     172  196 - #e1b70d
      96  200 - #e1b70d
     164  200 - #e1b70d
      80  204 - #e1b70d
     112  204 - #e1b70d
     128  204 - #e1b70d
     136  204 - #e1b70d
     144  204 - #e1b70d
     156  204 - #e1b70d
     400  224 - #796f7e
     428  224 - #d84637
     380  228 - #d84637
     356  232 - #d84637
     444  232 - #d84637
     192  236 - #2e89c5
     200  236 - #2e88c4
     336  236 - #793d3c
     416  236 - #d84637
     464  236 - #d84637
     148  240 - #816804
     184  240 - #6c6f33
     204  240 - #2a7fb9
     208  240 - #2a80b9
     212  240 - #2b80ba
     404  240 - #d84637
      84  244 - #816804
     188  244 - #297cb3
     196  244 - #287ab1
     204  244 - #2879af
     208  244 - #2779af
     212  244 - #2779af
     216  244 - #2879af
     220  244 - #287ab1
     372  244 - #d84637
     392  244 - #d84637
     488  244 - #d84637
     192  248 - #2674a9
     224  248 - #2573a8
     440  248 - #d84637
     456  248 - #d84637
     188  252 - #246fa2
     204  252 - #226b9c
     216  252 - #226a9b
     228  252 - #236d9e
     408  252 - #d84637
     424  252 - #d84637
     200  256 - #206694
     212  256 - #1f6391
     220  256 - #1f6391
     232  256 - #206695
     120  260 - #816804
     188  260 - #1f6290
     196  260 - #1e608c
     204  260 - #1d5e89
     236  260 - #1e618e
     348  264 - #84281e
     224  268 - #174f75
       4  272 - #597c95
     496  272 - #7b241c
     200  276 - #174f75
     152  280 - #816804
     220  280 - #174f75
     240  280 - #174f75
      88  284 - #816804
     440  284 - #7b241c
     188  288 - #325559
     208  292 - #174f75
     384  292 - #84281e
     244  296 - #174f75
     340  296 - #84281e
     476  296 - #7b241c
     196  300 - #174f75
     224  300 - #174f75
     592  308 - #597c95
     200  312 - #174f75
     216  312 - #174f75
     236  312 - #174f75
     144  316 - #816804
     364  316 - #84281e
     424  316 - #7b241c
      92  324 - #816804
     192  324 - #174f75
     224  324 - #174f75
     208  332 - #174f75
     336  332 - #84281e
     392  332 - #84281e
     192  340 - #325559
     160  344 - #816804
     448  344 - #7b241c
     364  352 - #84281e
     484  352 - #7b241c
     136  368 - #816804
     416  368 - #7b241c
       4  372 - #85b085
     172  376 - #816804
     100  380 - #816804
     144  404 - #816804
     180  412 - #816804
     108  424 - #816804
     592  448 - #85b085
     388  452 - #85b085
     300  464 - #85b085
       4  476 - #85b085
     216  496 - #85b085
     456  508 - #85b085
      84  524 - #85b085
     164  572 - #85b085
     504  584 - #85b085
       4  592 - #85b085
     320  592 - #85b085
     416  592 - #85b085
     592  592 - #85b085
";

const PRIMITIVES: &str = r"
       4    4 - #597c95
      96    4 - #597c95
     224    4 - #597c95
     388    4 - #597c95
     592    4 - #597c95
     492   72 - #597c95
     344   80 - #597c95
     124   92 - #597c95
     264  116 - #597c95
       4  152 - #597c95
     412  164 - #e1b70d
     372  168 - #917506
     592  168 - #597c95
     372  172 - #a28307
     392  172 - #c29d0a
     372  176 - #a28307
     504  176 - #597c95
     372  180 - #b29009
     384  184 - #c29d0a
     416  184 - #c29d0a
     368  188 - #816804
     396  192 - #c29d0a
     380  196 - #c29d0a
     368  200 - #816804
     384  208 - #c29d0a
     400  208 - #c29d0a
     416  208 - #c29d0a
     368  212 - #816804
     364  220 - #6d724d
     380  220 - #c29d0a
     216  224 - #d84637
     392  224 - #c29d0a
     140  228 - #796f7e
     368  228 - #816804
     412  228 - #c29d0a
     364  232 - #816804
     244  236 - #d84637
     384  236 - #c29d0a
     364  240 - #816804
     368  240 - #816804
     120  244 - #7b241c
     192  244 - #d84637
     364  244 - #816804
     432  244 - #3397d9
     364  248 - #816804
     368  248 - #816804
     404  248 - #318fcf
     132  252 - #7b241c
     364  252 - #816804
     384  252 - #2b81bb
     364  256 - #816804
     452  256 - #3395d6
     136  264 - #7b241c
     124  268 - #7b241c
     224  268 - #c43f31
     368  268 - #226a9b
     396  268 - #2d87c3
     424  268 - #3293d4
     176  272 - #c43f31
     444  272 - #3394d6
     360  276 - #19547b
     124  280 - #7b241c
     376  280 - #2470a3
     464  280 - #2e89c6
     144  284 - #8d2b21
     256  284 - #c43f31
     356  284 - #174f75
     408  288 - #2c84be
     424  288 - #2e8ac7
     440  288 - #308dcb
       4  292 - #597c95
     368  292 - #1c5a85
     380  292 - #236c9d
     140  296 - #7b241c
     200  296 - #c43f31
     356  296 - #174f75
     128  300 - #7b241c
     172  300 - #c43f31
     392  300 - #2470a3
     376  304 - #1b5983
     420  304 - #297db5
     444  304 - #2b82bc
     144  308 - #7b241c
     388  308 - #1f6492
     364  312 - #174f75
     460  312 - #246fa1
     572  312 - #85b085
     384  316 - #185279
     400  316 - #206593
     128  320 - #7b241c
     228  320 - #c43f31
     432  320 - #246fa2
     448  320 - #236ea0
     392  324 - #174f75
     408  324 - #1d5e8a
     420  324 - #206694
     148  328 - #7b241c
     184  328 - #c43f31
     372  328 - #174f75
     136  332 - #7b241c
     264  332 - #959470
     400  332 - #174f75
     440  332 - #19557c
     264  336 - #959470
     388  336 - #174f75
     264  340 - #a5785b
     428  340 - #174f75
     264  344 - #a5785b
     408  344 - #4e807d
     412  344 - #4e807d
     140  348 - #7b241c
     236  348 - #c43f31
     208  352 - #c43f31
     180  356 - #c43f31
     152  364 - #7b241c
     152  368 - #7b241c
     496  408 - #85b085
     592  420 - #85b085
       4  432 - #85b085
     224  452 - #85b085
     104  460 - #85b085
     328  460 - #85b085
     456  496 - #85b085
     168  540 - #85b085
       4  592 - #85b085
     332  592 - #85b085
     488  592 - #85b085
     592  592 - #85b085
";
