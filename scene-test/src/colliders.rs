use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Quat, Shape3, Vec3},
    refs::{Weak, manage::DataManager},
    scene::{Camera, Light, Model, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, Wall, scene},
    ui::Color,
    ui_test::{capture_screenshot, check_colors, set_record_probe_count},
};

/// Frames for a flag flipped on the main thread to reach the screen.
const SETTLE_FRAMES: usize = 2;

/// Every collider drawn as a green wireframe over its node: three rings
/// on a ball, the edges of a turned crate and the box around the
/// monkey's bounds that its ears and chin do not fill. The floor is a
/// wall too and stays as it is, a plane's slab is not drawn. The crate
/// at the back is a prop, it has no collider and shows none. Checked
/// with the colliders off, then on.
#[scene]
#[derive(Default)]
struct Colliders {}

impl SceneSetup for Colliders {
    fn needs_physics(&self) -> bool {
        true
    }

    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 4.0, 9.0),
            target: Vec3::new(0.0, 1.0, 0.0),
            ..Camera::default()
        };
        self.lights
            .push(Light::point(Vec3::new(3.0, 4.0, 4.0)).intensity(5.0).range(14.0));

        self.make_node::<Wall>(Shape3::Plane(14.0), Vec3::ZERO)
            .set_color(Color::hex("#b0b8c0"))
            .set_roughness(0.9);

        self.make_node::<Wall>(Shape3::Ball(0.8), Vec3::new(-2.6, 0.8, 0.0))
            .set_color(Color::hex("#3070d0"))
            .set_roughness(0.4);

        self.make_node::<Wall>(Shape3::cube(1.4), Vec3::new(0.0, 0.7, 0.0))
            .set_rotation(Quat::from_rotation_y(0.6))
            .set_color(Color::hex("#e08040"))
            .set_roughness(0.7);

        self.make_node::<Wall>(Shape3::Model(Model::get("Monkey.glb")), Vec3::new(2.8, 1.0, 0.0))
            .set_color(Color::hex("#e0c0a0"))
            .set_roughness(0.6);

        self.make_node::<Prop>(Shape3::cube(1.0), Vec3::new(0.0, 0.5, -3.5))
            .set_color(Color::hex("#808890"))
            .set_roughness(0.8);
    }
}

impl SceneTest for Colliders {
    fn perform_test(mut scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(192);

        for _ in 0..SETTLE_FRAMES {
            wait_for_next_frame();
        }
        check_colors(OFF)?;

        from_main(move || scene.show_colliders = true);
        for _ in 0..SETTLE_FRAMES {
            wait_for_next_frame();
        }
        capture_screenshot()?;
        check_colors(ON)
    }
}

const OFF: &str = r"
               4    4 - #597c95
             184    4 - #597c95
             292    4 - #597c95
             592    4 - #597c95
             440   80 - #597c95
              52   92 - #597c95
             148  104 - #597c95
             316  136 - #597c95
             592  144 - #597c95
               4  176 - #597c95
             224  180 - #597c95
             464  244 - #e7c7a7
             452  248 - #e7c7a7
             480  248 - #ebcaaa
             440  252 - #b99f86
             468  256 - #eccdae
             496  256 - #efd0b1
             424  260 - #c5a98e
             456  260 - #ecccae
             512  260 - #a8b0b7
             516  260 - #a8b0b7
             432  264 - #bba086
             484  264 - #f0d0b1
             512  264 - #a9b0b7
             516  264 - #a9b0b7
             148  268 - #356ac1
             304  268 - #d98048
             396  268 - #c1a68b
             408  268 - #a9b0b7
             412  268 - #a9b0b7
             424  268 - #ebccac
             440  268 - #bca288
             496  268 - #f1d3b5
             176  272 - #3770cb
             276  272 - #d97f48
             388  272 - #c7ab90
             428  272 - #ebccad
             468  272 - #f2d3b5
             520  272 - #8f7b67
             336  276 - #dc8149
             408  276 - #84715f
             420  276 - #806e5c
             436  276 - #bba187
             488  276 - #c4a88d
             128  280 - #3264b6
             252  280 - #7c482a
             384  280 - #7b6958
             392  280 - #937f6a
             412  280 - #85725f
             444  280 - #debfa1
             456  280 - #917d68
             500  280 - #917c68
             192  284 - #356ac1
             396  284 - #9b8570
             400  284 - #9b8670
             416  284 - #806e5c
             432  284 - #cfb296
             464  284 - #8d7966
             508  284 - #877461
             156  288 - #5282dc
             164  288 - #5786df
             352  288 - #d37c46
             404  288 - #a68e77
             424  288 - #83715e
             436  288 - #8b7764
             444  288 - #debfa1
             476  288 - #eecdad
             520  288 - #a48d76
             152  292 - #5281d9
             160  292 - #81a1eb
             172  292 - #4a7edb
             396  292 - #aa927a
             416  292 - #a79078
             448  292 - #b99f86
             480  292 - #a58d77
             488  292 - #cfb296
             116  296 - #2c559a
             156  296 - #7899e6
             160  296 - #98b0f0
             164  296 - #8ca8ed
             168  296 - #678fe2
             172  296 - #4f80da
             248  296 - #7c482a
             280  296 - #7c482a
             316  296 - #d37c46
             404  296 - #84725f
             424  296 - #82705e
             436  296 - #a89079
             456  296 - #d9bb9d
             464  296 - #e5c5a6
             500  296 - #e3c3a4
             512  296 - #bfa489
             152  300 - #4f7ed5
             160  300 - #7a9be6
             172  300 - #5281d9
             188  300 - #3971cc
             416  300 - #e3c4a5
             476  300 - #bda388
             488  300 - #a48d76
             136  304 - #366bc1
             156  304 - #4e7dd3
             164  304 - #5682d8
             172  304 - #5682d8
             264  304 - #7c482a
             432  304 - #7e6c5a
             440  304 - #a89079
             112  308 - #677997
             348  308 - #d17b45
             444  308 - #9c8670
             468  308 - #e6c7a8
             500  308 - #b59b82
             112  312 - #677895
             200  312 - #2f5da8
             248  312 - #88634e
             460  312 - #c3a88d
             488  312 - #d8ba9c
             120  316 - #2b5397
             184  316 - #356ac0
             284  316 - #7c482a
             448  316 - #7b6958
             476  316 - #a78f78
               4  320 - #a8b0b7
             248  320 - #947d72
             268  320 - #7c482a
             304  320 - #d07a45
             464  320 - #897562
             116  324 - #244072
             124  324 - #2a5091
             152  324 - #3264b5
             172  324 - #3466b9
             196  324 - #2c569c
             452  324 - #948f8a
             456  324 - #9d8771
             480  324 - #e1c2a3
             136  328 - #2d579f
             248  328 - #9f9896
             332  328 - #cf7a45
             472  332 - #cfb296
             476  332 - #cfb296
             124  336 - #244072
             140  336 - #294f8e
             176  336 - #2c569c
             188  336 - #284b87
             276  336 - #7c482a
             452  336 - #9d8671
             456  336 - #9d8771
             132  340 - #244072
             152  340 - #294f8e
             452  340 - #9c8670
             456  340 - #9d8771
             484  340 - #cfb295
             144  344 - #244073
             164  344 - #274882
             180  344 - #254277
             448  344 - #9b8570
             452  344 - #9c8670
             472  344 - #c2a78c
             156  348 - #244174
             168  348 - #244175
             176  348 - #244175
             288  348 - #7c482a
             312  348 - #ce7944
             456  348 - #b69c83
             148  352 - #244072
             160  352 - #244072
             260  352 - #7c482a
             352  352 - #be9881
             460  352 - #9a846f
             340  356 - #cc7843
             448  356 - #7d6b5a
             468  356 - #e7c7a8
             480  356 - #b0977f
             284  368 - #7c482a
             516  424 - #bdc5cd
               4  432 - #aab2b9
             436  432 - #bbc4cc
             592  436 - #bcc5cd
             208  440 - #afb7bf
             108  456 - #adb4bc
             516  476 - #c0c8d1
             292  488 - #b4bcc4
             416  488 - #bcc4cc
             472  504 - #bec6cf
             556  512 - #c0c8d1
             180  536 - #b0b7bf
             460  552 - #bbc4cc
             508  584 - #bbc4cc
               4  592 - #acb3bb
             100  592 - #aeb5bd
             260  592 - #b2bac1
             356  592 - #b5bdc5
             592  592 - #bdc5cd
";

const ON: &str = r"
               4    4 - #597c95
             264    4 - #597c95
             592    4 - #597c95
             428   60 - #597c95
             152  112 - #597c95
             288  136 - #597c95
               8  168 - #597c95
             384  232 - #2dbe7b
             400  232 - #2dbe7b
             416  232 - #2dbe7b
             440  232 - #2dbe7b
             476  232 - #2dbe7b
             496  232 - #2dbe7b
             524  232 - #2dbe7b
             460  244 - #e7c7a7
             380  248 - #597c95
             384  248 - #597c95
             388  248 - #597c95
             384  252 - #597c95
             388  256 - #597c95
             436  256 - #b99f85
             452  256 - #ebccad
             484  256 - #f0d0b1
             512  256 - #597c95
             468  260 - #eccdae
             384  264 - #a9b0b7
             404  264 - #a9b0b7
             496  264 - #efd0b1
             516  264 - #a9b0b7
             156  268 - #376fc8
             312  268 - #d98048
             384  268 - #a9b0b7
             412  268 - #a9b0b7
             428  268 - #ebccad
             524  268 - #d6b89a
             176  272 - #1cb896
             280  272 - #d97f48
             388  272 - #c7ab90
             420  272 - #806e5c
             440  272 - #bca288
             456  272 - #d5b79a
             476  272 - #f0d1b2
             136  276 - #356bc2
             400  276 - #cfb296
             412  276 - #d4b79a
             568  276 - #2dbe7b
             252  280 - #7c482a
             384  280 - #7b6958
             412  280 - #85725f
             424  280 - #816f5d
             484  280 - #c2a78c
             496  280 - #a08973
             524  280 - #c6aa8f
             356  284 - #55d88c
             376  284 - #80c5a2
             396  284 - #9b8570
             400  284 - #9b8670
             452  284 - #907b67
             464  284 - #8d7966
             120  288 - #2f5ba6
             156  288 - #5282dc
             164  288 - #5786df
             356  288 - #55d88c
             428  288 - #83715f
             444  288 - #debfa1
             516  288 - #8b7764
             152  292 - #5281d9
             160  292 - #81a1eb
             192  292 - #376fc9
             328  292 - #d37c46
             356  292 - #80c5a3
             376  292 - #55d88d
             396  292 - #aa927a
             420  292 - #7b6958
             456  292 - #d9bb9e
             480  292 - #a58d77
             500  292 - #e3c4a5
             156  296 - #7899e6
             160  296 - #98b0f0
             164  296 - #8ca8ed
             168  296 - #678fe2
             252  296 - #7c482a
             268  296 - #7c482a
             288  296 - #00ff60
             376  296 - #2bec76
             392  296 - #80c5a3
             404  296 - #84725f
             132  300 - #3569bf
             160  300 - #7a9be6
             168  300 - #5f89de
             172  300 - #5281d9
             288  300 - #00ff60
             376  300 - #00ff60
             392  300 - #56d98d
             412  300 - #857260
             424  300 - #7c6b59
             436  300 - #a89079
             460  300 - #d0b396
             472  300 - #bda288
             488  300 - #a48d76
             164  304 - #5682d8
             172  304 - #5682d8
             288  304 - #00ff60
             508  304 - #c1a68b
             112  308 - #677997
             188  308 - #376ec6
             288  308 - #00ff60
             444  308 - #9c8670
             248  312 - #88634e
             288  312 - #1fd153
             376  312 - #56d98e
             456  312 - #c3a88d
             468  312 - #e6c6a7
             488  312 - #d8ba9c
             124  316 - #2d59a1
             140  316 - #3366b8
             200  316 - #2d579e
             268  316 - #7c482a
             288  316 - #1fd153
             376  316 - #81c7a4
             448  316 - #7b6958
             180  320 - #3568bd
             288  320 - #1fd153
             328  320 - #d07a45
               4  324 - #a8b0b7
             164  324 - #278ca3
             288  324 - #1fd153
             352  324 - #cf7944
             456  324 - #9d8771
             460  324 - #9e8872
             120  328 - #244175
             136  328 - #2d579f
             248  328 - #75ab80
             288  328 - #1fd153
             148  332 - #2e5aa3
             264  332 - #7c482a
             288  332 - #1fd153
             380  332 - #aeb5bd
             480  332 - #d4b699
             172  336 - #2d579e
             188  336 - #284b87
             248  336 - #56d98e
             284  336 - #7c482a
             380  336 - #aeb6bd
             452  336 - #9d8671
             456  336 - #9d8771
             380  340 - #afb6be
             452  340 - #9c8670
             132  344 - #244072
             156  344 - #274780
             168  344 - #274881
             248  344 - #2bec77
             288  344 - #1fd153
             448  344 - #9b8570
             468  344 - #c2a78c
             148  348 - #244072
             176  348 - #244175
             288  348 - #3ea445
             320  348 - #ce7944
             160  352 - #244072
             268  352 - #7c482a
             288  352 - #3ea445
             352  352 - #be9881
             460  352 - #9a846f
             484  352 - #c2a78c
             288  356 - #3ea445
             456  356 - #99836e
             468  356 - #e7c7a8
             288  360 - #3ea445
             288  364 - #3ea445
             288  368 - #3ea445
             300  368 - #cc7843
             392  368 - #86cbaa
             420  368 - #86ccaa
             440  368 - #87ccaa
             472  368 - #87cdaa
             492  368 - #87cdaa
             512  368 - #87ccaa
             548  368 - #86ccaa
             588  428 - #bbc4cc
               4  436 - #aab2b9
             464  444 - #bdc6ce
             548  484 - #c0c9d1
             136  492 - #aeb6bd
             320  492 - #b6bec6
             592  528 - #c0c8d0
             440  532 - #bbc4cc
             508  584 - #bbc4cc
               4  592 - #acb3bb
             236  592 - #b1b9c1
             352  592 - #b5bdc5
             592  592 - #bdc5cd
";
