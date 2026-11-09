use std::f32::consts::FRAC_PI_4;

use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::{
        LossyConvert,
        volume::{Quat, Shape3, Vec3},
    },
    refs::{Weak, manage::DataManager},
    scene::{Camera, Model, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, scene},
    ui::Color,
    ui_test::{capture_screenshot, check_colors, set_record_probe_count},
};

/// Frames of the 45 degree turn between the checks.
const TURN_FRAMES: usize = 20;

/// A tall post, a ball, a turned crate and the monkey, all floating
/// above a light floor under a low sun that casts shadows, so every
/// shadow lies on the floor apart from what throws it. The post throws
/// a long one, the ball shades itself and its shadow, and the monkey, a
/// model, casts like a primitive. The turn moves the camera, not the
/// sun, so every shadow stays where it fell.
#[scene]
#[derive(Default)]
struct Shadows {}

impl SceneSetup for Shadows {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 6.5, 9.5),
            target: Vec3::new(0.0, 1.5, 0.0),
            ..Camera::default()
        };
        self.sun.direction = Vec3::new(-0.6, -1.0, -0.35);
        self.sun.shadows = true;
        // The default is 1024 on a phone and in the browser.
        self.sun.shadow_map_size = 2048;

        self.make_node::<Prop>(Shape3::Plane(14.0), Vec3::ZERO)
            .set_color(Color::hex("#c8ccd0"))
            .set_roughness(0.9);

        self.make_node::<Prop>(Shape3::cuboid(1.0, 3.2, 1.0), Vec3::new(-2.6, 3.0, 0.0))
            .set_color(Color::hex("#d35400"))
            .set_roughness(0.6);

        self.make_node::<Prop>(Shape3::Ball(0.8), Vec3::new(-0.3, 2.2, 2.2))
            .set_color(Color::hex("#2980b9"))
            .set_roughness(0.4);

        self.make_node::<Prop>(Shape3::cube(1.1), Vec3::new(3.0, 3.6, 0.6))
            .set_color(Color::hex("#f1c40f"))
            .set_rotation(Quat::from_rotation_y(0.6) * Quat::from_rotation_x(0.5))
            .set_roughness(0.5);

        self.make_node::<Prop>(Shape3::Model(Model::get("Monkey.glb")), Vec3::new(0.0, 2.6, -2.0))
            .set_color(Color::hex("#e0a060"))
            .set_roughness(0.6);
    }
}

impl SceneTest for Shadows {
    fn perform_test(mut scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);

        wait_for_next_frame();
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
     228    4 - #597c95
     380    4 - #597c95
     592    4 - #597c95
     116   36 - #597c95
     484   52 - #597c95
     592  120 - #597c95
     168  144 - #c85622
     144  148 - #c85621
     120  156 - #98411d
     188  164 - #ad4a1f
     472  168 - #ecc233
     160  172 - #98411d
     300  172 - #d1965e
     448  172 - #ecc233
     492  172 - #ecc233
     288  176 - #d3985f
     324  176 - #dda065
     460  176 - #ecc233
     280  180 - #9d7147
     480  180 - #ecc233
     308  184 - #daa371
     280  188 - #8c6540
     296  188 - #c18b58
     320  188 - #daa371
     468  188 - #ecc234
     180  192 - #98411d
     252  192 - #9b7046
     336  192 - #dfa673
     484  192 - #ecc234
     140  196 - #98411c
     272  196 - #ca925d
     308  196 - #daa270
     336  196 - #dfa773
     360  196 - #b78454
     436  196 - #846b1b
     500  196 - #ecc234
     284  200 - #a37549
     292  200 - #7b5838
     340  200 - #c48d58
     276  204 - #b38150
     352  204 - #a4764a
     192  208 - #ad4a1f
     308  208 - #a7784b
     332  208 - #7b5838
     480  208 - #ecc234
     520  208 - #8e731c
     288  212 - #a17549
     320  212 - #b68353
     328  212 - #ca925c
     340  212 - #996e45
       4  216 - #597c95
     292  216 - #a7794c
     416  216 - #846b1b
     160  220 - #98411c
     332  220 - #a37549
     500  220 - #8e731c
     448  224 - #846b1b
     188  228 - #ad4a1e
     300  228 - #bf8a58
     312  228 - #b58352
     324  228 - #7b5838
     132  236 - #98411c
     304  236 - #b38151
     468  236 - #846b1b
     436  240 - #846b1b
     196  244 - #ad4a1f
     492  244 - #8e731c
     304  248 - #9b7046
     316  248 - #c58e59
     160  252 - #98411c
     312  252 - #ae7e4f
     308  256 - #a27449
     452  256 - #846b1b
     192  264 - #ad4a1e
     304  264 - #7b5838
     276  280 - #3079ac
     296  280 - #327eb4
     148  284 - #98411c
     200  284 - #ad4a1f
     592  288 - #597c95
     248  292 - #286089
     268  296 - #3077aa
     288  300 - #4d8bbd
     292  300 - #5690c2
     296  300 - #5490c2
     300  300 - #498bbf
     240  304 - #255477
     284  304 - #4a87b9
     304  304 - #488abf
     196  308 - #ad4a1e
     288  308 - #6d9bc7
     292  308 - #95b8de
     300  308 - #71a0cd
     324  308 - #327db3
     164  312 - #98411c
     304  312 - #4e8bbe
     112  316 - #6e7072
     296  316 - #6193c1
     244  320 - #26577c
     268  320 - #2e709f
     328  328 - #2e72a2
      60  332 - #6e7072
     256  332 - #285e86
     300  336 - #2f74a5
     268  344 - #285e86
     240  348 - #214865
     376  348 - #6e7072
     104  352 - #6e7072
     148  356 - #6e7072
     288  356 - #275c83
     308  356 - #285f88
     256  360 - #214865
     272  372 - #214865
     288  372 - #214865
     192  380 - #6e7072
     232  388 - #6e7072
     592  396 - #bfc3c6
     252  420 - #6e7072
     200  428 - #6e7072
     352  472 - #bfc2c6
     464  476 - #bfc3c6
       4  484 - #bfc2c6
     244  544 - #bfc2c6
     360  588 - #bfc2c6
     128  592 - #bfc2c6
     476  592 - #bfc3c6
     592  592 - #bfc3c6
";

const SIDE: &str = r"
       4    4 - #597c95
     140    4 - #597c95
     276    4 - #597c95
     408    4 - #597c95
     592    4 - #597c95
     492   88 - #597c95
     208  128 - #c85621
     240  128 - #c85621
     180  132 - #98411c
     592  152 - #597c95
     212  160 - #ad4a1e
     244  160 - #ad4a1e
     184  168 - #98411c
     364  180 - #d3985f
     384  180 - #d3995f
     240  188 - #ad4a1e
     344  188 - #c08a56
     388  192 - #e0a267
     400  192 - #d99d62
       4  196 - #597c95
     188  196 - #98411c
     360  196 - #d59a63
     376  200 - #da9f69
     428  200 - #ecc230
     320  204 - #9d7147
     332  204 - #7b5838
     456  204 - #ecc231
     324  208 - #7b5838
     352  208 - #7b5838
     340  212 - #e2a974
     372  212 - #d99d62
     244  216 - #ad4a1e
     324  216 - #7b5838
     328  216 - #9e7247
     360  216 - #7b5838
     400  216 - #ecc231
     212  224 - #98411c
     328  224 - #7b5838
     344  224 - #906841
     356  224 - #bd8956
     376  224 - #996e45
     332  228 - #7c5938
     364  228 - #ab7b4d
     368  228 - #b18050
     340  232 - #be8956
     372  232 - #a37549
     420  232 - #ecc232
     452  232 - #ecc232
     340  236 - #a7794b
     348  236 - #b88553
     360  240 - #7b5838
     376  240 - #7b5838
     248  244 - #ad491d
     352  244 - #7b5838
     360  244 - #7b5838
     368  244 - #7b5838
     340  248 - #7b5838
     188  252 - #bfc3c6
     220  252 - #ad491d
     348  252 - #b48251
     436  252 - #ecc233
     180  256 - #bfc3c6
     184  256 - #bfc3c6
     336  256 - #8a633f
     344  256 - #9b6f46
     352  256 - #c58e59
     360  256 - #b48251
     400  256 - #ecc233
     340  260 - #9b7046
     344  260 - #9b6f46
     460  260 - #ecc234
     336  264 - #7b5838
     340  264 - #a17449
     348  264 - #ae7d4e
     360  264 - #b68352
     344  268 - #7b5838
     352  268 - #a5774a
     228  272 - #2f77aa
     200  276 - #4088be
     428  276 - #8e731c
     584  276 - #597c95
     168  280 - #3079ac
     192  280 - #468abf
     208  280 - #4288bd
     200  284 - #79a6d3
     188  288 - #4689bd
     196  288 - #87b0d9
     200  288 - #9bbee4
     204  288 - #79a6d2
     208  288 - #5590c2
     240  288 - #2b6a97
     200  292 - #83acd6
     208  292 - #508dbf
     300  292 - #6e7072
     192  296 - #4988bb
     204  296 - #528dbe
     224  300 - #3078ab
     376  300 - #8e731c
     452  300 - #8e731c
     156  304 - #29618a
     244  304 - #255578
     412  304 - #8e731c
     340  308 - #6e7072
     240  312 - #275b81
     184  316 - #2e72a2
     208  316 - #2f73a4
     232  320 - #285f87
     164  324 - #255478
     172  332 - #234f70
     224  332 - #245173
     196  336 - #26597e
     180  340 - #214865
     212  340 - #214865
     320  344 - #6e7072
     360  348 - #6e7072
     232  368 - #6e7072
     196  380 - #6e7072
     336  380 - #6e7072
     380  384 - #6e7072
     592  400 - #bfc2c6
       4  408 - #bfc2c6
     480  448 - #bfc2c6
     316  496 - #bfc2c6
     128  512 - #bfc2c6
     420  552 - #bfc2c6
       4  592 - #bfc2c6
     252  592 - #bfc2c6
     592  592 - #bfc2c6
";
