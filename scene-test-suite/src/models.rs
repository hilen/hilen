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
        SceneSetup, SceneTest, Wall, scene, step_scene,
    },
    ui::Color,
    ui_test::{capture_screenshot, check_colors, set_record_probe_count},
};

/// Steps of 1/60 s for the monkey to fall from `DROP` and come to rest.
const SETTLE_FRAMES: u32 = 240;
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
    // The scene steps by the real time of its frames, and 240 free frames of
    // a fast headless run ended with the monkey still in the air.
    fn stepped() -> bool {
        true
    }

    fn perform_test(mut scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);

        step_scene(SETTLE_FRAMES);
        wait_for_next_frame();

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
             112   16 - #498a42
             476   92 - #597c95
              80   96 - #2a4d27
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
              56  260 - #375f35
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
             372  272 - #d79c63
             292  276 - #ecaf76
             364  276 - #8c653f
             528  276 - #423837
             264  280 - #e1a368
              40  284 - #3b6b38
             292  284 - #815d3b
             324  284 - #9b7046
             348  284 - #efb179
             420  284 - #70593d
             308  288 - #cd945e
             320  288 - #ac7c4d
             324  288 - #9b6f46
             372  288 - #c78f5a
             240  292 - #9f7248
             244  292 - #eaab6e
             252  292 - #9a6f45
             292  292 - #d99d65
             332  292 - #efb17a
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
             296  320 - #b48151
             308  320 - #976d44
             320  320 - #d49961
             528  324 - #413535
             160  328 - #604833
             312  328 - #da9e64
             132  332 - #56402e
             292  332 - #9a6f46
             296  332 - #986d45
             292  336 - #9a6f46
             424  336 - #55422f
             472  344 - #2f2929
             508  344 - #d2ba85
             300  348 - #b28050
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
               4  416 - #aab2b9
             460  420 - #bcc4cc
             524  444 - #bfc7cf
             592  448 - #bec6ce
             232  452 - #b1b9c0
             412  468 - #bcc4cc
             124  484 - #aeb5bd
             516  488 - #c0c8d1
             340  496 - #b7bfc7
             592  496 - #c0c9d1
             472  500 - #bec7cf
             428  516 - #bcc4cc
             552  520 - #c0c8d1
             508  532 - #bec7cf
             592  544 - #bfc8d0
             460  552 - #bcc4cc
             504  584 - #bcc4cc
               4  592 - #acb4bb
             136  592 - #afb6be
             248  592 - #b2bac1
             364  592 - #b6bec6
             592  592 - #bdc5cd
";

const SIDE: &str = r"
               4    4 - #597c95
             408    4 - #597c95
             592    4 - #597c95
             228   44 - #498942
             112   48 - #597c95
             204   80 - #478340
             248  108 - #376235
             200  124 - #40733d
             500  124 - #597c95
              28  148 - #597c95
             280  148 - #376235
             392  156 - #597c95
             220  180 - #4d9246
             164  200 - #488641
             272  212 - #396636
             212  228 - #41753d
             288  244 - #3a6736
             324  252 - #d3985f
             176  260 - #41753d
             436  260 - #c69e6c
             468  264 - #c29968
             252  268 - #a9b0b7
             224  272 - #644a35
             264  272 - #b98553
             304  272 - #ecad70
             500  272 - #bf9667
             532  276 - #b98f60
             256  280 - #815d3b
             268  280 - #b88553
             396  280 - #cca46f
             248  284 - #815d3a
             560  284 - #ddb993
             280  288 - #9b6f46
             344  288 - #aa7b4d
             360  288 - #dea268
             424  288 - #c39866
               4  292 - #a9b0b7
             272  292 - #a6774b
             540  292 - #e5bc93
             548  292 - #e5c097
             264  296 - #a17449
             288  296 - #efb37c
             348  296 - #a27549
             448  296 - #bb8e5d
             468  296 - #bb8d5d
             492  296 - #ba8d5d
             536  296 - #e5bf96
             224  300 - #644a35
             300  300 - #87613d
             356  300 - #ce955d
             284  304 - #d79b62
             316  304 - #936a43
             524  304 - #e8c197
             260  308 - #b48151
             344  308 - #ebae76
             520  308 - #e8c29a
             564  308 - #7b8ea5
             312  312 - #a27549
             504  312 - #eabf96
             512  312 - #d57362
             280  316 - #d49960
             304  316 - #a27449
             336  316 - #d1975e
             352  316 - #b17f4f
             428  316 - #5b4a45
             476  316 - #4a3d3d
             500  316 - #eac197
             292  320 - #825e3b
             264  324 - #d2985f
             388  324 - #332c2b
             296  328 - #946b43
             496  328 - #dadee1
             460  332 - #413636
             556  332 - #8193a9
             292  340 - #a27549
             432  340 - #493837
             524  340 - #7b8fa5
             268  344 - #bb8754
             496  348 - #493c3c
             360  352 - #312929
             272  356 - #e5a669
             296  356 - #996e45
             280  360 - #b07e4f
             440  360 - #4e3c3d
             556  360 - #8a99ab
             116  364 - #aeb6bd
             396  364 - #b53f34
             504  364 - #7d90a6
             396  368 - #a93d2d
             536  376 - #996869
             368  380 - #2f2928
             472  380 - #3e3433
             420  384 - #322b2a
             516  384 - #8897ab
             536  388 - #98777d
             500  400 - #8a98ab
             396  404 - #2f2828
             444  404 - #cfb481
             440  408 - #d3b984
             472  408 - #3d3332
             360  412 - #3c302a
             436  412 - #d3ba88
             432  420 - #d1ba82
               8  424 - #b0b8c0
             428  428 - #d0b681
             412  436 - #302a28
             424  436 - #cdb27f
             484  436 - #614a42
             488  444 - #d6d6d6
             448  452 - #ab8a66
             240  460 - #bcc4cc
             288  460 - #bcc4cc
             188  468 - #bcc4cc
             136  484 - #bcc4cc
             592  484 - #b0b8bf
             320  492 - #bdc5ce
             368  504 - #bcc4cc
             272  532 - #c0c8d1
              72  540 - #bcc4cc
             212  540 - #c0c9d1
             380  552 - #bcc4cd
             488  560 - #b7bfc7
               4  592 - #bac2ca
              56  592 - #bcc4cc
             156  592 - #bfc8d0
             268  592 - #c0c8d0
             332  592 - #bec6cf
             592  592 - #b3bbc3
";
