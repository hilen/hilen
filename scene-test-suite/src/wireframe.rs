use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Quat, Shape3, Vec3},
    refs::{Weak, manage::DataManager},
    scene::{Camera, Light, Model, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, scene},
    ui::Color,
    ui_test::{capture_screenshot, check_colors, set_record_probe_count},
};

/// Frames for a flag flipped on the main thread to reach the screen.
const SETTLE_FRAMES: usize = 2;

/// The edges of every triangle drawn as white lines over its node: the
/// rings and meridians of a ball, the faces of a turned crate split in
/// 2 triangles each, every part of the monkey model and a translucent
/// ball, whose back edges show through it. The floor is 2 triangles
/// with 1 edge across. Checked with the wireframe off, then on.
#[scene]
#[derive(Default)]
struct Wireframe {}

impl SceneSetup for Wireframe {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 4.0, 9.0),
            target: Vec3::new(0.0, 1.0, 0.0),
            ..Camera::default()
        };
        self.lights
            .push(Light::point(Vec3::new(3.0, 4.0, 4.0)).intensity(5.0).range(14.0));

        self.make_node::<Prop>(Shape3::Plane(14.0), Vec3::ZERO)
            .set_color(Color::hex("#b0b8c0"))
            .set_roughness(0.9);

        self.make_node::<Prop>(Shape3::Ball(0.8), Vec3::new(-2.6, 0.8, 0.0))
            .set_color(Color::hex("#3070d0"))
            .set_roughness(0.4);

        self.make_node::<Prop>(Shape3::cube(1.4), Vec3::new(0.0, 0.7, 0.0))
            .set_rotation(Quat::from_rotation_y(0.6))
            .set_color(Color::hex("#e08040"))
            .set_roughness(0.7);

        self.make_node::<Prop>(Shape3::Model(Model::get("Monkey.glb")), Vec3::new(2.8, 1.0, 0.0))
            .set_color(Color::hex("#e0c0a0"))
            .set_roughness(0.6);

        self.make_node::<Prop>(Shape3::Ball(0.6), Vec3::new(-0.6, 0.6, 2.4))
            .set_color(Color::hex("#40c060").with_alpha(0.5))
            .set_roughness(0.5);
    }
}

impl SceneTest for Wireframe {
    fn perform_test(mut scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(192);

        for _ in 0..SETTLE_FRAMES {
            wait_for_next_frame();
        }
        check_colors(OFF)?;

        from_main(move || scene.show_wireframe = true);
        for _ in 0..SETTLE_FRAMES {
            wait_for_next_frame();
        }
        capture_screenshot()?;
        check_colors(ON)
    }
}

const OFF: &str = r"
               4    4 - #597c95
             280    4 - #597c95
             592    4 - #597c95
             436   76 - #597c95
             144  108 - #597c95
             312  136 - #597c95
             592  144 - #597c95
               4  180 - #597c95
             464  244 - #e7c7a7
             480  248 - #ebcaaa
             428  256 - #b89e84
             444  256 - #ebcbad
             496  256 - #efd0b1
             512  260 - #a8b0b7
             516  260 - #a8b0b7
             464  264 - #eccdae
             484  264 - #f0d0b1
             512  264 - #a9b0b7
             516  264 - #a9b0b7
             148  268 - #356ac1
             396  268 - #c1a68b
             408  268 - #a9b0b7
             412  268 - #a9b0b7
             440  268 - #bca288
             452  268 - #ebccad
             176  272 - #3770cb
             292  272 - #d98048
             388  272 - #c7ab90
             428  272 - #ebccad
             520  272 - #8f7b67
             320  276 - #db8148
             420  276 - #806e5c
             488  276 - #c4a88d
             128  280 - #3264b6
             252  280 - #7c482a
             344  280 - #dd8249
             384  280 - #7b6958
             444  280 - #debfa1
             456  280 - #917d68
             468  280 - #8f7b67
             500  280 - #917c68
             192  284 - #356ac1
             400  284 - #9b8670
             416  284 - #806e5c
             432  284 - #cfb296
             508  284 - #877461
             156  288 - #5282dc
             164  288 - #5786df
             268  288 - #7c482a
             404  288 - #a68e77
             436  288 - #8b7764
             476  288 - #eecdad
             520  288 - #a48d76
             152  292 - #5281d9
             160  292 - #81a1eb
             172  292 - #4a7edb
             416  292 - #a79078
             448  292 - #b99f86
             488  292 - #cfb296
             116  296 - #2c559a
             156  296 - #7899e6
             160  296 - #98b0f0
             164  296 - #8ca8ed
             168  296 - #678fe2
             248  296 - #7c482a
             308  296 - #d37c46
             404  296 - #84725f
             424  296 - #82705e
             436  296 - #a89079
             464  296 - #e5c5a6
             500  296 - #e3c3a4
             512  296 - #bfa489
             152  300 - #4f7ed5
             160  300 - #7a9be6
             284  300 - #7c482a
             416  300 - #e3c4a5
             476  300 - #bda388
             488  300 - #a48d76
             136  304 - #366bc1
             156  304 - #4e7dd3
             164  304 - #5682d8
             172  304 - #5682d8
             260  304 - #7c482a
             352  304 - #d17b45
             432  304 - #7e6c5a
             444  308 - #9c8670
             460  308 - #c4a88d
             500  308 - #b59b82
             200  312 - #2f5da8
             280  312 - #7c482a
             468  312 - #e6c6a7
             488  312 - #d8ba9c
             448  316 - #7b6958
             476  316 - #a78f78
               4  320 - #a8b0b7
             268  320 - #7c482a
             316  320 - #d07a45
             464  320 - #897562
             116  324 - #244072
             124  324 - #2a5091
             152  324 - #3264b5
             172  324 - #3466b9
             196  324 - #2c569c
             288  324 - #7c482a
             456  324 - #9d8771
             480  324 - #e1c2a3
             136  328 - #2d579f
             340  328 - #cf7944
             472  332 - #cfb296
             476  332 - #cfb296
             124  336 - #244072
             176  336 - #2c569c
             188  336 - #284b87
             452  336 - #9d8671
             132  340 - #244072
             152  340 - #294f8e
             252  340 - #608145
             264  340 - #628547
             452  340 - #9c8670
             456  340 - #9d8771
             144  344 - #244073
             164  344 - #274882
             180  344 - #254277
             276  344 - #638749
             448  344 - #9b8570
             452  344 - #9c8670
             472  344 - #c2a78c
             484  344 - #cfb295
             176  348 - #244175
             256  348 - #64894a
             280  348 - #64894a
             456  348 - #b69c83
             148  352 - #244072
             160  352 - #244072
             268  352 - #678d4d
             460  352 - #9a846f
             228  356 - #74aa87
             280  356 - #668d4c
             448  356 - #7d6b5a
             468  356 - #e7c7a8
             480  356 - #b0977f
             244  360 - #7bbb91
             292  360 - #8b9e55
             324  360 - #cd7843
             260  364 - #8ec7a0
             264  364 - #90c9a2
             268  364 - #8ec9a1
             284  364 - #678d4d
             292  364 - #8ba056
             264  368 - #95cba6
             268  368 - #92caa4
             272  368 - #8ec9a1
             292  368 - #8ba056
             296  368 - #899a53
             260  372 - #8fc6a1
             268  372 - #90c8a2
             216  384 - #6c8f7b
             240  384 - #77b18c
             216  388 - #6c8f7a
             288  388 - #7bba91
             220  396 - #6c907a
             260  396 - #79b38d
             220  400 - #6c907b
             224  400 - #6d917c
             228  408 - #6c907b
             284  408 - #74a486
             232  412 - #6d907b
             236  416 - #6d907b
             244  416 - #6d917b
             264  420 - #6d917c
             512  424 - #bdc5cd
             444  428 - #bcc4cc
               4  432 - #aab2b9
             592  432 - #bcc4cc
             124  480 - #aeb5bd
             368  488 - #b9c1c9
             464  488 - #bec6cf
             556  512 - #c0c8d1
             280  524 - #b4bbc3
             448  540 - #bbc4cc
             496  576 - #bcc4cc
               4  592 - #acb3bb
             188  592 - #b0b7bf
             372  592 - #b6bec6
             592  592 - #bdc5cd
";

const ON: &str = r"
               4    4 - #597c95
             168    4 - #597c95
             272    4 - #597c95
             592    4 - #597c95
             432   76 - #597c95
             136  104 - #597c95
             268  140 - #597c95
             588  144 - #597c95
             372  164 - #597c95
               4  188 - #597c95
             464  244 - #e7c7a7
             440  252 - #b99f86
             448  252 - #e8c8a8
             452  256 - #ebccad
             460  256 - #ecccae
             480  256 - #f0d0b1
             492  256 - #efd0b1
             432  260 - #bba086
             448  260 - #ebccad
             468  260 - #eccdae
             404  264 - #a9b0b7
             420  264 - #c5a98e
             440  264 - #bca288
             496  264 - #efd0b1
             516  264 - #a9b0b7
             148  268 - #ffffff
             168  268 - #ffffff
             304  268 - #d98048
             408  268 - #a9b0b7
             412  268 - #a9b0b7
             448  268 - #ebccad
             472  268 - #eccdaf
             284  272 - #d98048
             432  272 - #bba087
             480  272 - #f0d0b1
             332  276 - #dc8149
             392  276 - #c1a68b
             504  276 - #f2d9c1
             420  280 - #806e5c
             460  280 - #927d69
             488  280 - #c3a88d
             260  284 - #7c482a
             432  284 - #cfb296
             520  284 - #e9c9a9
             176  288 - #3f79d8
             248  288 - #7c482a
             424  288 - #83715e
             148  292 - #4478d4
             276  292 - #7c482a
             400  292 - #ffffff
             428  292 - #83715e
             472  292 - #8e7a66
             136  296 - #376ec6
             180  296 - #3e77d4
             192  296 - #376fc9
             260  296 - #7c482a
             312  296 - #d37c46
             456  296 - #ffffff
               4  300 - #597c95
             172  300 - #5281d9
             272  300 - #7c482a
             344  300 - #d27b46
             428  300 - #7e6c5a
             116  304 - #2c5599
             128  304 - #3365b6
             188  304 - #386fca
             476  304 - #bca288
             268  308 - #7c482a
             280  308 - #7c482a
             328  308 - #d17b45
             436  308 - #7b6958
             120  312 - #2d579d
             168  312 - #3f73cb
             444  312 - #7b6958
             456  312 - #c3a88d
             468  312 - #e6c6a7
             492  312 - #d8ba9c
             176  316 - #386dc4
             308  316 - #d17b45
             252  320 - #7c482a
             276  320 - #7c482a
             120  324 - #274984
             188  324 - #3160af
             132  328 - #2b5498
             164  328 - #3262b2
             180  328 - #3160ae
             268  328 - #7c482a
             324  328 - #cf7a45
             152  332 - #2e5ba5
             160  332 - #2f5da9
             468  332 - #d0b396
             164  336 - #2d589f
             172  336 - #2d579e
             456  336 - #9d8771
             484  336 - #d0b396
             188  340 - #ffffff
             256  340 - #ffffff
             268  340 - #ffffff
             308  340 - #cf7944
             464  340 - #baa086
             140  344 - #244072
             156  344 - #274780
             276  344 - #ffffff
             472  344 - #c2a78c
             232  348 - #ffffff
             352  352 - #ffffff
             448  352 - #8a7763
             248  356 - #ffffff
             264  356 - #ffffff
             292  360 - #ffffff
             324  360 - #cd7843
             260  364 - #ffffff
             240  376 - #79b58e
             248  376 - #7dba92
             256  376 - #84c098
             264  376 - #8ac49d
             296  376 - #ffffff
             252  380 - #7ebb93
             232  384 - #75aa88
             252  388 - #7ab68f
             280  392 - #7bb991
             272  396 - #7ab68f
             296  396 - #ffffff
             280  404 - #ffffff
             232  412 - #ffffff
             264  416 - #ffffff
             276  416 - #ffffff
               4  432 - #aab2b9
             592  440 - #bdc5cd
             364  472 - #b8c0c8
             124  480 - #aeb5bd
             464  488 - #bec6cf
             284  532 - #b4bbc3
               4  592 - #acb3bb
             184  592 - #b0b7bf
             380  592 - #b6bec6
             484  592 - #bac2ca
             592  592 - #bdc5cd
";
