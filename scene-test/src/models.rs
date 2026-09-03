use std::f32::consts::FRAC_PI_4;

use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::{
        LossyConvert,
        volume::{Shape3, Vec3},
    },
    refs::{Weak, manage::DataManager},
    scene::{
        Body, Camera, CoefficientCombineRule, Light, Model, Node, NodeTemplates, Prop, SceneCreation,
        SceneSetup, SceneTest, Wall, scene,
    },
    ui::Color,
    ui_test::{capture_screenshot, check_colors, set_record_probe_count},
};

/// Frames for the monkey to fall from `DROP` and come to rest.
const SETTLE_FRAMES: usize = 240;
/// Frames of the 45 degree turn between the checks.
const TURN_FRAMES: usize = 20;
const DROP: f32 = 3.0;
/// How far the resting origin may sit from the height its bounds put it at.
const REST_TOLERANCE: f32 = 0.02;

/// Suzanne, the Blender monkey, dropped as a body onto the floor next to
/// a tree of five meshes and a cube with an embedded texture, the three
/// shipped `.glb` fixtures. The monkey has no material and takes the
/// node's, the tree brings its own colors and node placements, the cube
/// its texture. The monkey has no uvs, so it also pins that a mesh
/// without a tangent frame still lights. The drop lands the
/// monkey on its bounds, so the resting height proves the collider sits
/// around the model, not at its origin.
#[scene]
#[derive(Default)]
struct Models {
    monkey: Weak<Body>,
}

impl SceneSetup for Models {
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

        self.make_node::<Wall>(Shape3::Plane(16.0), Vec3::ZERO)
            .set_color(Color::hex("#b0b8c0"))
            .set_roughness(0.9);

        let mut monkey =
            self.make_node::<Body>(Shape3::Model(Model::get("Monkey.glb")), Vec3::new(0.0, DROP, 0.0));
        monkey
            .set_color(Color::hex("#e0a060"))
            .set_roughness(0.6)
            .set_restitution(0.0, CoefficientCombineRule::Min)
            .set_damping(0.5, 2.0);
        monkey.lock_rotations();
        self.monkey = monkey;

        self.make_node::<Prop>(Shape3::Model(Model::get("tree.glb")), Vec3::new(-3.2, 0.0, -1.0))
            .set_roughness(0.9);

        self.make_node::<Prop>(
            Shape3::Model(Model::get("textured_cube.glb")),
            Vec3::new(3.2, 1.0, 0.0),
        )
        .set_roughness(0.7);
    }
}

impl SceneTest for Models {
    fn perform_test(mut scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);

        for _ in 0..SETTLE_FRAMES {
            wait_for_next_frame();
        }

        let (height, bounds) =
            from_main(move || (scene.monkey.position().y, Model::get("Monkey.glb").bounds));
        // The box around the bounds rests on the floor, so the origin
        // sits its lower half above it.
        let expected = bounds.half_extents().y - bounds.center().y;
        anyhow::ensure!(
            (height - expected).abs() < REST_TOLERANCE,
            "the monkey rests at {height}, its bounds put it at {expected}"
        );

        check_colors(FRONT)?;

        let frames: f32 = TURN_FRAMES.lossy_convert();
        for _ in 0..TURN_FRAMES {
            from_main(move || scene.camera.orbit(FRAC_PI_4 / frames, 0.0));
            wait_for_next_frame();
        }

        capture_screenshot()?;
        check_colors(SIDE)
    }
}

const FRONT: &str = r"
               4    4 - #597c95
             360    4 - #597c95
             592    4 - #597c95
             236    8 - #597c95
             112   12 - #478641
             476   92 - #597c95
              80   96 - #2a4d27
             112   96 - #366133
             148   96 - #2a4e27
             304  120 - #597c95
             592  136 - #597c95
             184  148 - #3f713b
             112  152 - #40733d
               4  164 - #597c95
             208  208 - #4c8f45
             132  212 - #4d9246
              76  236 - #375f35
             436  236 - #c19967
             460  236 - #bf9765
             488  236 - #ba9162
             512  236 - #d8b38e
             528  244 - #e2bc93
             300  252 - #e0a265
             544  252 - #e7bf95
             188  256 - #40733c
             276  256 - #ad7d4e
             412  256 - #6f573a
             448  256 - #cea471
             472  256 - #c59a69
             324  260 - #eeb27c
             556  264 - #d7d9e3
             264  268 - #ba8654
             308  268 - #eeb27c
             344  268 - #eeb178
             500  268 - #4e4341
             556  268 - #d9dbdf
             244  272 - #b58251
             276  272 - #aa7a4c
             320  272 - #eeb27c
             332  272 - #eeb27c
             368  272 - #dda066
             292  276 - #ecaf76
             528  276 - #423837
             264  280 - #e1a368
             280  280 - #be8956
             344  280 - #eeb179
             368  280 - #e1a367
              40  284 - #3b6b38
             292  284 - #815d3b
             324  284 - #9b7046
             420  284 - #70593d
             308  288 - #cd945e
             320  288 - #ac7c4d
             324  288 - #9b6f46
             332  288 - #85603c
             372  288 - #c78f5a
             240  292 - #9f7248
             244  292 - #eaab6e
             252  292 - #9a6f45
             340  292 - #7f5c3a
             156  296 - #614834
             264  296 - #9d7147
             288  296 - #ebaf78
             332  296 - #eeb077
             556  296 - #413636
             128  300 - #56402e
             296  300 - #ce955d
             312  300 - #b58251
             328  300 - #dda066
             352  300 - #ac7c4e
             508  300 - #803d37
             268  304 - #986e45
             412  304 - #6e563b
             272  308 - #986e45
             432  308 - #6f573b
             460  308 - #332c2b
             300  312 - #dda168
             316  312 - #d49961
             136  316 - #56402e
             276  316 - #7b5838
             288  316 - #b38151
             336  316 - #825e3b
             300  320 - #dda167
             308  320 - #976d44
             320  320 - #d49961
             528  324 - #413535
             160  328 - #604834
             132  332 - #56402e
             292  332 - #9a6f46
             296  332 - #986d45
             292  336 - #9a6f46
             424  336 - #55422f
             472  344 - #2f2929
             508  344 - #d2ba85
             300  348 - #b28050
             508  348 - #cbac7a
             548  352 - #d6d6d6
             548  356 - #d5d5d6
             296  360 - #906841
             324  360 - #996e45
             308  364 - #7b5838
             544  364 - #d6d2ce
             544  368 - #d4d3d6
             452  372 - #302a27
             496  372 - #c1a271
             544  372 - #d2d3d2
               4  416 - #aab1b9
             468  420 - #bcc4cc
             232  452 - #b1b8c0
             592  452 - #bec6cf
             524  456 - #bfc8d0
             472  460 - #bec7cf
             416  464 - #bcc4cc
             124  484 - #aeb5bd
             340  496 - #b7bfc7
             476  500 - #bec7cf
             592  500 - #c0c9d1
             428  512 - #bcc4cc
             552  524 - #c0c8d0
             508  532 - #bec7cf
             592  544 - #bfc7d0
             496  576 - #bcc4cc
             544  584 - #bcc5cd
               4  592 - #acb3bb
             136  592 - #aeb6be
             248  592 - #b2b9c1
             368  592 - #b6bec6
             592  592 - #bdc5cd
";

const SIDE: &str = r"
               4    4 - #597c95
             120    4 - #597c95
             408    4 - #597c95
             592    4 - #597c95
             228   44 - #498942
             192  100 - #457f3f
             244  104 - #376235
             500  120 - #597c95
             256  132 - #376235
             180  148 - #3c6939
             280  148 - #376234
             228  152 - #3f713c
             392  156 - #597c95
               8  176 - #597c95
             232  200 - #4c9045
             168  204 - #488641
             272  212 - #396636
             292  248 - #8c844d
             296  248 - #637642
             332  260 - #d59a60
             440  260 - #c59c6b
             172  264 - #75937a
             252  268 - #a8b0b7
             488  268 - #bf9665
             224  272 - #644a35
             308  272 - #e9ab71
             400  276 - #caa06d
             532  276 - #b98f60
             248  280 - #815d3b
             256  280 - #815d3b
             264  280 - #e8aa71
             220  284 - #6c5e53
             428  284 - #c59b6a
             560  284 - #ddb993
             232  288 - #644a35
             504  288 - #ba8e5f
             272  292 - #a6774b
             288  292 - #7f5c3a
             360  292 - #8c653f
             548  292 - #e5c097
             220  296 - #6c5e53
             340  296 - #976d44
             484  296 - #ba8d5e
             536  296 - #e5bf96
             240  300 - #604733
             300  300 - #87613d
             356  300 - #ce955d
             464  300 - #bb8d5d
             292  304 - #e2a56a
             316  304 - #936a43
             524  304 - #e8c197
             260  308 - #b48151
             344  308 - #ebae76
             520  308 - #e8c29a
             564  308 - #7b8ea5
             308  312 - #a27549
             328  312 - #835e3b
             388  312 - #352d2c
             424  312 - #473b3b
             504  312 - #eabf96
             512  312 - #d57362
             280  316 - #d49960
             336  316 - #d1975e
             352  316 - #b17f4f
             500  316 - #eac197
             296  320 - #825e3b
             300  320 - #825e3b
             316  320 - #7b5838
             476  320 - #473b3b
             264  324 - #d2985f
             300  324 - #7e5a3a
             496  328 - #dadee1
             360  332 - #8e9197
             556  332 - #8193a9
             292  336 - #a27549
             360  340 - #6f6f73
             424  340 - #423232
             524  340 - #7b8fa5
             292  344 - #a27549
             360  344 - #504d4e
               4  348 - #abb2b9
             468  348 - #3f3434
             496  348 - #493c3c
             296  352 - #a4764a
             272  356 - #e5a669
             280  360 - #b07e4f
             440  360 - #4e3c3d
             556  360 - #8a99ab
             292  364 - #986e45
             396  364 - #b53f34
             504  364 - #7d90a6
             120  368 - #aeb6bd
             396  368 - #a93d2d
             364  376 - #2f2928
             536  376 - #996869
             472  380 - #3e3433
             420  384 - #322b2a
             536  388 - #98777d
             500  400 - #8a98ab
             396  404 - #2f2828
             444  404 - #cfb481
             440  408 - #d3b984
             360  412 - #3c302a
             436  412 - #d3ba88
             432  420 - #d1ba82
             428  428 - #d0b681
             412  436 - #302a28
             424  436 - #cdb27f
             484  436 - #614a42
             488  444 - #d6d6d6
             448  452 - #ab8a66
               4  456 - #b3bac2
             240  456 - #bcc4cc
             188  468 - #bcc4cc
             300  476 - #bdc5cd
             136  484 - #bcc4cc
             592  484 - #b0b7bf
             368  508 - #bcc4cc
             220  516 - #c0c8d0
             276  532 - #c0c8d0
              72  540 - #bcc4cc
             496  548 - #b6bec6
             328  564 - #bec7cf
             396  576 - #bcc4cc
               4  592 - #bac2ca
             156  592 - #bfc8d0
             268  592 - #c0c8d0
             592  592 - #b3bbc2
";
