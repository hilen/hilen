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
/// its texture. The monkey and the tree have no uvs, so they also pin
/// that a mesh without a tangent frame still lights. The drop lands the
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

        // The tree brings its own ground plane, wider than the floor and
        // without a material, so it takes the node's color and sits a
        // hair below the floor to show only past its edge.
        self.make_node::<Prop>(
            Shape3::Model(Model::get("tree.glb")),
            Vec3::new(-3.2, -0.02, -1.0),
        )
        .set_color(Color::hex("#6b8e23"))
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
     248    4 - #597c95
     376    4 - #597c95
     108    8 - #3d641e
     120    8 - #436d20
     120   12 - #436d20
     124   20 - #436d1f
     592   32 - #597c95
     108   56 - #789369
     144   64 - #436e1f
     104   92 - #5a7c43
     472  104 - #597c95
      72  108 - #293f1a
     160  108 - #293f1a
     300  120 - #597c95
     148  144 - #739063
     108  148 - #577a40
     592  148 - #597c95
     160  180 - #6e8c5d
     112  196 - #6a8959
      60  200 - #3d641d
     152  224 - #678755
     196  232 - #436f1f
     384  236 - #6b8a35
     436  236 - #c19967
     488  236 - #ba9162
     512  236 - #d8b38e
     112  244 - #638450
     416  244 - #6f583d
     528  244 - #e2bc93
       4  248 - #6b8a35
      56  252 - #6b8a35
     300  252 - #e0a265
     544  252 - #e7bf95
     276  256 - #ad7d4e
     448  256 - #cea471
     472  256 - #c59a69
     324  260 - #eeb27c
     412  264 - #70563a
     532  264 - #423837
     556  264 - #d7d9e3
     308  268 - #eeb27c
     500  268 - #4e4341
     244  272 - #b58251
     276  272 - #aa7a4c
     320  272 - #eeb27c
     332  272 - #eeb27c
     368  272 - #dda066
     552  272 - #493d3d
     292  276 - #ecaf76
     436  276 - #6e573d
     592  276 - #6b8a35
     264  280 - #e1a368
     344  280 - #eeb179
     524  280 - #423837
     132  284 - #5d8048
     292  284 - #815d3b
     324  284 - #9b7046
     308  288 - #cd945e
     320  288 - #ac7c4d
     324  288 - #9b6f46
     372  288 - #c78f5a
     412  288 - #6f583c
     468  288 - #362f2e
     176  292 - #5d8148
     240  292 - #9f7248
     252  292 - #9a6f45
     340  292 - #7f5c3a
      88  296 - #3d651d
     264  296 - #9d7147
     288  296 - #ebaf78
     332  296 - #eeb077
     556  296 - #413636
     296  300 - #ce955d
     312  300 - #b58251
     328  300 - #dda066
     352  300 - #ac7c4e
     428  300 - #6f583c
     508  300 - #803d37
     268  304 - #986e45
     272  308 - #986e45
     300  312 - #dda168
     316  312 - #d49961
     276  316 - #7b5838
     288  316 - #b38151
     336  316 - #825e3b
     208  320 - #446f1f
     300  320 - #dda167
     308  320 - #976d44
     320  320 - #d49961
     468  320 - #2e2c2b
      36  324 - #3d641d
     528  324 - #413535
     296  332 - #986d45
     292  336 - #9a6f46
     424  336 - #55422f
     116  340 - #95a49a
     136  340 - #4f3b23
     508  344 - #d2ba85
     300  348 - #b28050
     548  352 - #d6d6d6
     464  356 - #2f2928
     548  356 - #d5d5d6
     296  360 - #906841
     324  360 - #996e45
     308  364 - #7b5838
     544  364 - #d6d2ce
     428  368 - #2b2420
     544  368 - #d4d3d6
     496  372 - #c1a271
     544  372 - #d2d3d2
     484  420 - #bcc4cc
       4  452 - #abb2b9
     232  452 - #b1b8c0
     592  452 - #bec6cf
     416  460 - #bcc4cc
     528  464 - #c0c8d0
     340  492 - #b7bfc7
     128  496 - #aeb5bd
     476  500 - #bec7cf
     592  500 - #c0c9d1
     428  512 - #bcc4cc
     552  524 - #c0c8d0
     508  532 - #bec7cf
     592  544 - #bfc7d0
     496  576 - #bcc4cc
       4  592 - #acb3bb
     248  592 - #b2b9c1
     592  592 - #bdc5cd
";

const SIDE: &str = r"
       4    4 - #597c95
     452    4 - #597c95
     112   16 - #597c95
     224   16 - #4a712f
     340   24 - #597c95
     232   68 - #4b722e
     592   92 - #597c95
     196  112 - #415e58
     264  112 - #415e58
      64  116 - #597c95
     472  128 - #597c95
     232  156 - #4b732e
     280  180 - #385c1c
     204  200 - #4b752b
     344  224 - #6b8a35
     160  228 - #6b8a35
     400  232 - #6b8a35
      84  240 - #6b8a35
     296  248 - #626d2e
       4  252 - #6b8a35
     204  252 - #4a7429
     564  256 - #628365
     332  260 - #d59a60
     436  260 - #c69e6c
     480  268 - #bf9765
     256  272 - #e9a96b
     264  272 - #b98553
     392  276 - #cca46f
      48  280 - #6b8a35
     152  280 - #3d641d
     248  280 - #815d3b
     268  280 - #b88553
     344  280 - #ab7b4d
     420  284 - #c69c6a
     516  284 - #ba8e60
     560  284 - #ddb993
     264  288 - #eeb27c
     280  288 - #9b6f46
     360  288 - #dea268
     496  288 - #ba8e5f
     552  288 - #e1bc93
     252  292 - #c18b57
     288  292 - #7f5c3a
     352  292 - #9c7046
     540  292 - #e5bc93
     548  292 - #e5c097
     272  296 - #a37549
     536  296 - #e5bf96
     300  300 - #87613d
     340  300 - #7d5a39
     356  300 - #ce955d
     472  300 - #bb8d5d
     524  300 - #e7bd90
     216  304 - #4f3b22
     236  304 - #4c3921
     284  304 - #d79b62
     316  304 - #936a43
     332  304 - #835e3c
     440  304 - #453a38
     260  308 - #b48151
     520  308 - #e8c29a
     308  312 - #a27549
     328  312 - #835e3b
     504  312 - #eabf96
     512  312 - #d57362
     280  316 - #d49960
     320  316 - #835e3b
     352  316 - #b17f4f
     400  316 - #372f2e
     500  316 - #eac197
     312  320 - #7b5838
     540  320 - #7a8ca5
     304  324 - #7f5b3a
     268  328 - #976d44
     296  328 - #946b43
     460  328 - #413736
     496  328 - #dadee1
     360  332 - #8e9197
     560  332 - #8294a9
     424  336 - #3d3332
     360  340 - #6f6f73
     528  340 - #7c8fa5
     292  344 - #a27549
     360  344 - #504d4e
     272  356 - #e5a669
     448  356 - #3f3532
     288  360 - #c68f59
     556  360 - #8a99ab
     396  364 - #b53f34
     504  364 - #7d90a6
     396  368 - #a93d2d
     536  376 - #996869
     112  380 - #afb7bf
     360  384 - #312a28
     468  384 - #3e3433
     384  388 - #2e2827
     536  388 - #98777d
     404  400 - #2f2828
     500  400 - #8a98ab
       4  404 - #aeb6be
     440  408 - #d3b984
     436  412 - #d3ba88
     380  420 - #2e2826
     432  420 - #d1ba82
     404  424 - #2e2827
     428  428 - #d0b681
     424  436 - #cdb27f
     484  436 - #614a42
     488  444 - #d6d6d6
     448  452 - #ab8a66
     240  460 - #bcc4cc
     184  468 - #bcc4cc
     320  480 - #bcc4cc
     592  484 - #b0b7bf
     132  488 - #bcc4cc
       4  496 - #b6bec5
     368  508 - #bcc4cc
     264  512 - #bfc8d0
     212  536 - #c0c8d1
     312  548 - #bfc7cf
      80  552 - #bcc4cd
     488  556 - #b7bfc7
     380  564 - #bcc4cd
       4  592 - #bac2ca
      52  592 - #bcc4cc
     152  592 - #bfc7d0
     260  592 - #c0c8d0
     592  592 - #b3bbc2
";
