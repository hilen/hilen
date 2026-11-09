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
             296    4 - #597c95
             592    4 - #597c95
             444   80 - #597c95
              52   92 - #597c95
             148  108 - #597c95
             320  136 - #597c95
             592  144 - #597c95
               4  176 - #597c95
             224  180 - #597c95
             468  244 - #eacaaa
             480  248 - #ebcaaa
             432  252 - #b89e84
             456  252 - #e8c8a8
             492  252 - #ebcbaa
             444  256 - #ebcbad
             472  256 - #efcfb1
             484  260 - #f0d0b1
             512  260 - #a9b0b7
             516  260 - #a9b0b7
             420  264 - #c5a98e
             460  264 - #eccdae
             512  264 - #a9b0b7
             516  264 - #a9b0b7
             148  268 - #356ac0
             296  268 - #d97f48
             396  268 - #c1a68b
             408  268 - #a9b0b8
             412  268 - #a9b0b8
             428  268 - #ebccad
             440  268 - #bca288
             476  268 - #f0d0b2
             492  268 - #efd0b1
             176  272 - #3770cb
             324  272 - #db8048
             388  272 - #c7ab90
             400  272 - #82705d
             416  272 - #7b6958
             424  272 - #ebccad
             456  272 - #d5b79a
             520  272 - #8f7b67
             132  276 - #3366b8
             408  276 - #84715f
             420  276 - #806e5c
             252  280 - #7c482a
             384  280 - #7b6958
             392  280 - #937f6a
             412  280 - #85725f
             432  280 - #c8ac90
             456  280 - #917d68
             484  280 - #c2a78c
             500  280 - #917c68
             164  284 - #467cdb
             192  284 - #356ac1
             396  284 - #9b8570
             400  284 - #9b8670
             416  284 - #806e5c
             432  284 - #cfb296
             468  284 - #8e7a66
             508  284 - #877461
             120  288 - #2f5ba5
             152  288 - #487cd8
             168  288 - #4d80dd
             404  288 - #a68e77
             436  288 - #8b7764
             444  288 - #debfa1
             464  288 - #e7c8a9
             476  288 - #eecdad
             520  288 - #a48d76
             160  292 - #83a2eb
             172  292 - #4a7edb
             276  292 - #7c482a
             352  292 - #d27b46
             396  292 - #aa927a
             420  292 - #7b6958
             480  292 - #a58d77
             488  292 - #cfb296
             500  292 - #e3c4a5
             136  296 - #376dc6
             156  296 - #7799e6
             160  296 - #97b0ef
             164  296 - #8ca8ed
             168  296 - #678fe2
             172  296 - #4f80da
             328  296 - #d27c46
             404  296 - #84725f
             428  296 - #83705e
             452  296 - #c4a98e
             464  296 - #e5c5a6
             484  296 - #bda388
             512  296 - #bfa489
             152  300 - #4e7dd4
             160  300 - #7899e6
             164  300 - #7396e4
             168  300 - #5f89dd
             172  300 - #5281d9
             308  300 - #d27b46
             416  300 - #e3c4a5
             436  300 - #a89079
             444  300 - #8c7865
             460  300 - #d0b396
             472  300 - #bda288
             488  300 - #a48d76
             116  304 - #2c5499
             156  304 - #4d7cd3
             172  304 - #5682d8
             248  304 - #7c482a
             440  304 - #a89079
             492  304 - #8a7663
             508  304 - #c1a68b
             284  308 - #7c482a
             444  308 - #9c8670
             468  308 - #e6c7a8
             500  308 - #b59b82
               4  312 - #a8b0b7
             200  312 - #2f5da8
             452  312 - #c2a78c
             476  312 - #a79079
             488  312 - #d8ba9c
             140  316 - #3365b8
             184  316 - #356ac0
             268  316 - #7c482a
             476  316 - #a78f78
             124  320 - #2c5499
             336  320 - #d07a45
             464  320 - #897562
             116  324 - #244072
             164  324 - #3466b9
             456  324 - #9d8771
             460  324 - #9e8872
             480  324 - #e1c2a3
             132  328 - #2b5497
             196  328 - #294d8b
             288  328 - #7c482a
             124  332 - #254276
             140  332 - #2c559a
             188  332 - #2c5499
             272  332 - #7c482a
             472  332 - #cfb296
             476  332 - #cfb296
             160  336 - #2d579e
             172  336 - #2d579e
             252  336 - #7c482a
             312  336 - #cf7944
             452  336 - #9d8671
             456  336 - #9d8771
             128  340 - #244072
             136  340 - #254276
             184  340 - #26467d
             352  340 - #cd7844
             452  340 - #9c8670
             456  340 - #9d8771
             484  340 - #cfb295
             148  344 - #254378
             448  344 - #9b8570
             452  344 - #9c8670
             472  344 - #c2a78c
             140  348 - #244072
             168  348 - #244175
             176  348 - #244174
             288  348 - #7c482a
             592  348 - #afb6be
             148  352 - #244072
             156  352 - #244072
             164  352 - #244072
             272  352 - #7c482a
             448  352 - #8a7763
             460  352 - #9a846f
             484  352 - #c2a78c
             328  356 - #cc7843
             468  356 - #e7c7a8
             292  368 - #cc7843
             432  432 - #bcc4cc
             508  432 - #bec6ce
               4  436 - #abb2b9
             216  436 - #b0b8bf
             592  440 - #bdc5cd
             108  448 - #adb4bc
             412  484 - #bcc4cc
             516  484 - #c0c8d1
             296  488 - #b5bdc4
             464  500 - #bec6ce
             556  516 - #c0c9d1
             180  528 - #b0b7bf
             460  552 - #bcc4cc
             504  580 - #bcc4cc
               4  592 - #acb4bb
             104  592 - #aeb6bd
             256  592 - #b2bac2
             356  592 - #b6bec5
             592  592 - #bdc5cd
";

const ON: &str = r"
               4    4 - #597c95
             120    4 - #597c95
             272    4 - #597c95
             592    4 - #597c95
             432   72 - #597c95
             152  112 - #597c95
             312  132 - #597c95
             592  144 - #597c95
               4  172 - #597c95
             220  184 - #597c95
             460  244 - #e7c7a7
             380  248 - #597c95
             384  248 - #597c95
             388  248 - #597c95
             448  248 - #e7c7a7
             472  248 - #ebcaaa
             488  248 - #eacaaa
             384  252 - #597c95
             388  252 - #597c95
             436  252 - #b99f85
             388  256 - #597c95
             480  256 - #f0d0b1
             512  256 - #597c95
             384  260 - #a9b0b7
             388  260 - #a9b0b7
             424  260 - #c5a98e
             448  260 - #ebccad
             464  260 - #eccdae
             384  264 - #a9b0b7
             404  264 - #a9b0b7
             416  264 - #7b6958
             436  264 - #bca187
             492  264 - #efd0b1
             516  264 - #a9b0b7
             156  268 - #376fc9
             300  268 - #d98048
             384  268 - #a9b0b8
             408  268 - #a9b0b8
             476  268 - #f0d0b2
             504  268 - #efcfb0
             524  268 - #d6b89a
             140  272 - #356ac1
             172  272 - #3972d0
             420  272 - #806e5c
             432  272 - #bba087
             456  272 - #d5b79a
             332  276 - #dc8149
             400  276 - #cfb296
             412  276 - #d4b79a
             248  280 - #7c482a
             384  280 - #7b6958
             392  280 - #937f6a
             424  280 - #816f5d
             468  280 - #8f7b67
             484  280 - #c2a78c
             496  280 - #a08973
             524  280 - #c6aa8f
             188  284 - #3871cc
             396  284 - #9b8570
             400  284 - #9b8670
             416  284 - #806e5c
             448  284 - #877461
             508  284 - #877461
             120  288 - #2f5ba5
             156  288 - #5383dc
             164  288 - #5886df
             268  288 - #7c482a
             408  288 - #c5aa8f
             424  288 - #83715e
             436  288 - #8b7764
             464  288 - #e7c8a9
             476  288 - #eecdad
             136  292 - #376ec7
             160  292 - #83a2eb
             168  292 - #5e89e0
             396  292 - #aa927a
             488  292 - #cfb296
             516  292 - #bfa48a
             156  296 - #7799e6
             160  296 - #97b0ef
             164  296 - #8ca8ed
             168  296 - #678fe2
             288  296 - #00ff60
             404  296 - #84725f
             420  296 - #7b6958
             440  296 - #d6b89a
             452  296 - #c4a98e
             464  296 - #e5c5a6
             504  296 - #dabc9d
             120  300 - #2f5da9
             160  300 - #7899e6
             164  300 - #7396e4
             168  300 - #5f89dd
             172  300 - #5281d9
             288  300 - #00ff60
             316  300 - #d27b46
             376  300 - #00ff60
             412  300 - #857260
             428  300 - #7e6c5a
             460  300 - #d0b396
             476  300 - #bda388
             164  304 - #5581d7
             168  304 - #5481d7
             172  304 - #5682d8
             272  304 - #7c482a
             288  304 - #00ff60
             348  304 - #d17b45
             436  304 - #a79078
             492  304 - #8a7663
             508  304 - #c1a68b
             200  308 - #305fac
             256  308 - #7c482a
             288  308 - #00ff60
             444  308 - #9c8670
             500  308 - #b59b82
               4  312 - #a8b0b7
             124  312 - #2f5ca6
             456  312 - #c3a88d
             468  312 - #e6c6a7
             488  312 - #d8ba9c
             144  316 - #3467bb
             448  316 - #7b6958
             476  316 - #a78f78
             128  320 - #2e59a1
             160  320 - #3569be
             180  320 - #3568bc
             272  320 - #7c482a
             120  324 - #274983
             256  324 - #7c482a
             308  324 - #d07a45
             456  324 - #9d8771
             460  324 - #9e8872
             480  324 - #e1c2a3
             136  328 - #2d579e
             196  328 - #294d8b
             332  328 - #cf7944
             124  332 - #254276
             152  332 - #2e5ba5
             176  332 - #2f5ca7
             380  332 - #aeb6bd
             164  336 - #2d589f
             268  336 - #7c482a
             380  336 - #aeb6be
             452  336 - #9d8671
             456  336 - #9d8771
             128  340 - #244072
             144  340 - #284a85
             188  340 - #254277
             252  340 - #7c482a
             284  340 - #7c482a
             352  340 - #cd7844
             380  340 - #afb7be
             452  340 - #9c8670
             456  340 - #9d8771
             468  340 - #c3a88d
             156  344 - #264780
             180  344 - #254277
             448  344 - #9b8570
             452  344 - #9c8670
             484  344 - #cfb295
             140  348 - #244072
             172  348 - #244175
             308  348 - #ce7944
             592  348 - #afb6be
             148  352 - #244072
             160  352 - #244072
             460  352 - #9a846f
             448  356 - #7d6b5a
             456  356 - #99836e
             468  356 - #e7c7a8
             480  356 - #b0977f
             272  360 - #7c482a
             328  360 - #cc7843
             300  368 - #cc7843
             540  420 - #bcc4cc
               4  432 - #abb2b9
             592  444 - #bec6ce
             220  448 - #b0b8c0
             492  448 - #bfc7cf
             416  452 - #bcc4cc
             544  476 - #c1c9d1
             312  492 - #b6bdc5
             132  496 - #aeb6bd
             592  516 - #c0c9d1
             432  524 - #bcc4cc
             524  532 - #bfc7cf
               4  592 - #acb4bb
             104  592 - #aeb6bd
             236  592 - #b1b9c1
             348  592 - #b5bdc5
             520  592 - #bcc4cc
             592  592 - #bdc5cd
";
