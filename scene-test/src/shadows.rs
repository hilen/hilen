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
     152    4 - #597c95
     360    4 - #597c95
     592    4 - #597c95
     476   48 - #597c95
     256   52 - #597c95
     592  120 - #597c95
     188  144 - #6e7078
     120  152 - #b04c1f
     156  152 - #b04c1f
     180  164 - #ad4a1e
     472  168 - #ecc233
     296  172 - #cc935c
     312  172 - #d89d63
     448  172 - #ecc233
     492  172 - #ecc233
     284  176 - #b88553
     324  176 - #dda065
     460  176 - #ecc233
     152  180 - #98411d
     188  180 - #ad4a1f
     300  184 - #c18b58
     124  188 - #98411c
     280  188 - #8c6540
     320  188 - #daa371
     468  188 - #ecc234
     252  192 - #9b7046
     272  192 - #ca925d
     336  192 - #dfa673
     484  192 - #ecc234
     292  196 - #d59c68
     308  196 - #daa270
     336  196 - #dfa773
     360  196 - #b78454
     436  196 - #846b1b
     500  196 - #ecc234
     276  200 - #bd8855
     340  200 - #c48d58
     160  204 - #98411c
     352  204 - #a4764a
     192  208 - #ad4a1f
     308  208 - #a7784b
     332  208 - #7b5838
     480  208 - #ecc234
     520  208 - #8e741c
     136  212 - #98411c
     320  212 - #b68353
     292  216 - #a7794c
     328  216 - #ab7b4d
     336  216 - #a37549
     416  216 - #846b1b
       4  220 - #597c95
     500  220 - #8e741c
     300  224 - #bf8a58
     448  224 - #846b1b
     316  228 - #b88553
     324  228 - #7b5838
     168  232 - #98411c
     196  236 - #ad4a1f
     300  236 - #9a6f46
     308  236 - #b88554
     468  236 - #846b1b
     136  240 - #98411c
     308  244 - #aa7b4d
     492  244 - #8e741d
     304  248 - #9b7046
     312  248 - #ae7e4f
     188  256 - #ad4a1e
     308  256 - #a27449
     452  256 - #846b1b
     304  264 - #7b5838
     168  268 - #98411c
     200  272 - #b26849
     280  276 - #2f75a7
     140  284 - #98411c
     252  288 - #2a6691
     200  296 - #ad4a1f
     288  300 - #4d8abd
     292  300 - #558fc1
     296  300 - #538fc2
     300  300 - #488bbf
     284  304 - #4b88b9
     292  304 - #77a3cf
     304  304 - #488abf
     244  308 - #275d84
     288  308 - #6e9cc8
     292  308 - #95b8de
     300  308 - #709fcc
     324  308 - #327db3
     264  312 - #2e71a1
      96  316 - #6e7072
     148  316 - #b5a29c
     160  316 - #b5a29c
     172  316 - #b5a29c
     176  316 - #b5a29c
     180  316 - #b5a29c
     184  316 - #b5a29c
     188  316 - #b5a29c
     192  316 - #b5a29c
     296  316 - #6395c2
     304  316 - #4787ba
     232  324 - #708596
      60  332 - #6e7072
     256  332 - #285e86
     328  332 - #2d6f9e
     300  336 - #3074a5
     280  340 - #2c6a97
     240  348 - #214865
     376  348 - #6e7072
     108  352 - #6e7072
     260  352 - #224b6a
     164  356 - #6e7072
     308  356 - #285f88
     248  360 - #214865
     288  360 - #25567b
     300  368 - #214865
     272  372 - #214865
     212  384 - #6e7072
     592  396 - #bfc2c6
     184  420 - #6e7072
     244  424 - #6e7072
     464  476 - #bfc2c6
       4  484 - #bec2c6
     244  548 - #bec2c6
     360  588 - #bfc2c6
     128  592 - #bec2c6
     476  592 - #bfc2c6
     592  592 - #bfc2c6
";

const SIDE: &str = r"
       4    4 - #597c95
     136    4 - #597c95
     320    4 - #597c95
     592    4 - #597c95
     456   52 - #597c95
     184  128 - #c85621
     240  128 - #c85621
     204  136 - #98411c
     232  152 - #ad4a1e
     592  152 - #597c95
     208  156 - #98411c
     184  160 - #98411c
     248  172 - #6e7077
     204  176 - #98411c
     364  180 - #d3985f
     184  184 - #696d77
     248  184 - #98573c
     388  184 - #d3995f
     188  188 - #98411c
     220  188 - #ad4a1e
     344  188 - #c08a56
       4  192 - #597c95
     372  192 - #d59a63
     360  196 - #d59a63
     392  196 - #e0a267
     452  200 - #c7b149
     456  200 - #a39f63
     208  204 - #98411c
     320  204 - #9d7147
     332  204 - #7b5838
     376  204 - #d99f69
     404  204 - #ecc130
     428  204 - #ecc230
     352  208 - #7b5838
     340  212 - #e2a974
     228  216 - #ad4a1e
     328  216 - #9e7247
     360  216 - #7b5838
     196  220 - #98411c
     336  220 - #a8794c
     328  224 - #7b5838
     344  224 - #906841
     356  224 - #bd8956
     372  224 - #7b5838
     396  224 - #ecc231
     332  228 - #7c5938
     368  228 - #b18050
     428  228 - #ecc231
     248  232 - #ad4a1d
     340  232 - #be8956
     372  232 - #a37549
     348  236 - #b88553
     460  236 - #ecc232
     360  240 - #7b5838
     376  240 - #7b5838
     344  244 - #cb945e
     368  244 - #7b5838
     404  244 - #ecc232
     220  248 - #ad4a1d
     340  248 - #7b5838
     188  252 - #bfc2c6
     180  256 - #bfc2c6
     184  256 - #bfc2c6
     336  256 - #8a633f
     344  256 - #9b6f46
     352  256 - #c58e59
     452  256 - #ecc233
     252  260 - #bba49c
     344  260 - #9b6f46
     392  260 - #8e741c
     428  260 - #a68822
     432  260 - #bd9b28
     252  264 - #b68672
     360  264 - #b68352
     224  268 - #3079ac
     344  268 - #7b5838
     376  268 - #b3af9c
     352  272 - #966c44
     376  272 - #a79b71
     404  272 - #8e741c
     200  276 - #4088be
     376  276 - #9a8847
     192  280 - #468abf
     208  280 - #4288bd
     436  280 - #8e741c
     456  280 - #8e741c
     160  284 - #2d6f9e
     200  284 - #77a5d2
     236  284 - #2d71a2
     584  284 - #597c95
     196  288 - #88b0da
     204  288 - #78a5d1
     188  292 - #4587bb
     200  292 - #85aed7
     208  292 - #508dbf
     300  292 - #6e7072
     420  292 - #8e741c
     192  296 - #4a89bc
     200  296 - #5c93c2
     244  296 - #275c84
     392  300 - #8e741c
     172  304 - #2f75a7
     220  304 - #3178ab
     392  304 - #b3af9c
     240  308 - #29618a
     340  308 - #6e7072
     448  308 - #8e741c
     160  320 - #245173
     188  320 - #2d6f9f
     236  320 - #26577b
     172  328 - #275a80
     212  332 - #285e86
     228  332 - #214865
     180  340 - #214865
     200  344 - #214865
     320  344 - #6e7072
     360  348 - #6e7072
     228  372 - #6e7072
     336  380 - #6e7072
     380  384 - #6e7072
       4  412 - #bfc2c6
     592  412 - #bfc2c6
     480  448 - #bec2c6
     132  496 - #bec2c6
     420  552 - #bec2c6
       4  592 - #bec2c6
     252  592 - #bec2c6
     592  592 - #bfc2c6
";
