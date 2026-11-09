use std::f32::consts::FRAC_PI_6;

use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::{
        LossyConvert,
        volume::{Quat, Shape3, Vec3},
    },
    refs::{Weak, manage::DataManager},
    scene::{Camera, Light, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, scene},
    ui::{Color, Image, Size},
    ui_test::{capture_screenshot, check_colors, set_record_probe_count},
};

/// Texels of the generated normal map, and bumps along each side.
const MAP_SIZE: u32 = 128;
const BUMPS: u32 = 8;
/// Frames of the 30 degree turn between the checks.
const TURN_FRAMES: usize = 15;

/// A crate texture on a box and on a ball, so it wraps a flat face and a
/// curved one, next to a ball and a box with a normal map of round
/// bumps, under the sun and a lamp on the left. Every bump has to shade
/// like a dome standing out of the surface, lit on the side the lamp is
/// on, and the turn moves the highlights with the eye while the bumps
/// stay put.
#[scene]
#[derive(Default)]
struct Textures {}

impl SceneSetup for Textures {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 4.0, 8.0),
            target: Vec3::new(0.0, 1.0, 0.0),
            ..Camera::default()
        };

        self.lights
            .push(Light::point(Vec3::new(-3.0, 3.0, 3.0)).intensity(4.0).range(12.0));

        let crate_texture = Image::get("crate_box.png");
        let bumps = bump_normal_map();

        self.make_node::<Prop>(Shape3::Plane(14.0), Vec3::ZERO)
            .set_color(Color::hex("#b0b8c0"))
            .set_roughness(0.9);

        let mut crate_box = self.make_node::<Prop>(Shape3::cube(2.0), Vec3::new(-2.4, 1.0, 0.0));
        crate_box
            .set_color(Color::hex("#ffffff"))
            .set_rotation(Quat::from_rotation_y(0.4))
            .set_roughness(0.7);
        crate_box.material.texture = Some(crate_texture);

        let mut crate_ball = self.make_node::<Prop>(Shape3::Ball(0.9), Vec3::new(0.0, 0.9, 1.6));
        crate_ball.set_color(Color::hex("#ffffff")).set_roughness(0.5);
        crate_ball.material.texture = Some(crate_texture);

        let mut bump_ball = self.make_node::<Prop>(Shape3::Ball(1.0), Vec3::new(2.4, 1.0, 0.0));
        bump_ball.set_color(Color::hex("#d8d8d8")).set_roughness(0.4);
        bump_ball.material.normal_map = Some(bumps);

        let mut bump_box = self.make_node::<Prop>(Shape3::cube(1.6), Vec3::new(0.5, 0.8, -2.2));
        bump_box.set_color(Color::hex("#e6b45a")).set_roughness(0.5);
        bump_box.material.normal_map = Some(bumps);
    }
}

/// A grid of round bumps as a tangent space normal map, green up.
fn bump_normal_map() -> Weak<Image> {
    let cell = MAP_SIZE / BUMPS;
    let radius: f32 = cell.lossy_convert() * 0.4;
    let mut data = Vec::with_capacity((MAP_SIZE * MAP_SIZE * 4) as usize);

    for y in 0..MAP_SIZE {
        for x in 0..MAP_SIZE {
            let dx: f32 = (x % cell).lossy_convert() - cell.lossy_convert() / 2.0 + 0.5;
            let dy: f32 = (y % cell).lossy_convert() - cell.lossy_convert() / 2.0 + 0.5;
            let inside = dx * dx + dy * dy < radius * radius;
            let normal = if inside {
                Vec3::new(
                    dx / radius,
                    -dy / radius,
                    (1.0 - (dx * dx + dy * dy) / (radius * radius)).sqrt(),
                )
            } else {
                Vec3::Z
            }
            .normalize();
            for channel in [normal.x, normal.y, normal.z] {
                let byte: u8 = ((channel * 0.5 + 0.5) * 255.0).round().lossy_convert();
                data.push(byte);
            }
            data.push(255);
        }
    }

    Image::from_raw_data(data, "bumps_normal_map", Size::new(MAP_SIZE, MAP_SIZE), 4)
}

impl SceneTest for Textures {
    fn perform_test(mut scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);

        wait_for_next_frame();
        check_colors(FRONT)?;

        let frames: f32 = TURN_FRAMES.lossy_convert();
        for _ in 0..TURN_FRAMES {
            from_main(move || scene.camera.orbit(FRAC_PI_6 / frames, 0.0));
            wait_for_next_frame();
        }

        capture_screenshot()?;
        check_colors(SIDE)
    }
}

const FRONT: &str = r"
       4    4 - #597c95
     244    4 - #597c95
     592    4 - #597c95
     420   60 - #597c95
     124   88 - #597c95
     592  156 - #597c95
     300  228 - #e0b05c
     320  228 - #deae5b
     356  228 - #e3b25e
     108  232 - #bd8644
     156  232 - #79533e
     132  236 - #a5653e
      84  240 - #a96c40
     180  240 - #a4623b
     336  240 - #dcad5a
     212  244 - #5c4336
     288  244 - #e6b86a
     304  244 - #c89e53
     320  248 - #c99e53
     352  248 - #c89d53
     164  252 - #a3633c
     368  256 - #c59b52
     420  256 - #b5b5b5
     476  256 - #cbcbcb
     312  260 - #8e703b
      92  264 - #413e3a
     196  264 - #4f3b30
     444  264 - #c1c1c1
     120  268 - #c28c4c
     288  268 - #d5a859
     320  268 - #d2a658
     340  268 - #d2a657
     144  272 - #a4733f
     168  272 - #9b6a3d
     224  272 - #7a4a32
     396  272 - #9c9c9c
     408  272 - #bbbbbb
     456  272 - #e1e1e1
     476  272 - #dedede
     464  276 - #f0f0f0
     504  276 - #c4c4c4
     292  280 - #8f703b
     360  280 - #a48144
     404  280 - #898989
     488  280 - #e1e1e1
     312  284 - #734b30
     396  284 - #9d9d9d
     460  284 - #e5e5e5
     340  288 - #d2a557
     392  288 - #cccccc
     428  288 - #7c7c7c
     468  288 - #dedede
     480  288 - #b7b7b7
      92  292 - #db874a
     208  292 - #2e2e2f
     272  292 - #735334
     444  292 - #818181
     140  296 - #c27445
     164  300 - #c98154
     388  300 - #898989
     296  304 - #323331
     364  304 - #c69c52
     408  304 - #939393
     228  308 - #2f2d2c
     252  308 - #9d613b
     184  312 - #916144
     272  312 - #6d4d3d
     424  312 - #7b7b7b
     440  312 - #9b9b9b
     484  316 - #767676
     148  320 - #a9653c
     248  320 - #9e643d
     392  320 - #838383
     408  320 - #8b8b8b
     592  320 - #a9b0b7
     316  324 - #ad7e68
     428  324 - #dcdcdc
     492  324 - #dcdcdc
       4  328 - #adb5bc
     104  328 - #8c5839
     208  328 - #694b37
     316  328 - #b8846d
     320  328 - #665d59
     268  332 - #555557
     340  332 - #b77947
     476  332 - #c2c2c2
     236  336 - #7a4c30
     268  336 - #535352
     360  336 - #ae6c39
     456  336 - #9b9b9b
     496  336 - #999999
     408  340 - #858585
     144  344 - #483830
     296  344 - #5c4235
     184  348 - #6d4732
     204  348 - #553e2f
     432  348 - #9a9a9a
     476  348 - #b7b7b7
     340  356 - #28292b
     456  356 - #767676
     104  360 - #ac673d
     220  360 - #2c2c2c
     344  364 - #262729
     120  368 - #2e2d2d
     264  368 - #262829
     140  372 - #b57238
     240  372 - #573523
     300  372 - #9a603d
     192  376 - #2e2c2b
     340  376 - #252626
     168  380 - #2e2c2a
     268  384 - #272423
     112  388 - #905736
     336  396 - #252221
     260  400 - #4b3022
     312  400 - #38261d
     272  404 - #221f1e
     304  412 - #573623
     592  444 - #aab2b9
       4  460 - #c8d1da
     468  488 - #adb4bc
     112  504 - #c4cdd5
     292  524 - #b4bcc4
     196  584 - #b8bfc7
     488  588 - #adb4bc
       4  592 - #bfc8d0
     388  592 - #afb7be
     592  592 - #abb3ba
";

const SIDE: &str = r"
       4    4 - #597c95
     332    4 - #597c95
     592    4 - #597c95
     168   48 - #597c95
     464  100 - #597c95
     592  176 - #597c95
     152  220 - #724f38
     184  220 - #905a3b
     248  220 - #503627
     228  224 - #815137
     208  228 - #9f5f3a
     372  232 - #dfaf5c
     108  236 - #3d3e3e
     424  236 - #e1b15d
     156  240 - #be854b
     348  240 - #e6b55f
     132  244 - #de9e51
     176  244 - #c58146
     376  244 - #e6b55f
     400  244 - #e5b45f
     228  248 - #3e3126
     384  248 - #e5b55f
     408  248 - #e6b560
     248  252 - #3b2b24
     340  256 - #c89d53
     360  256 - #8e6f3b
     112  260 - #dd854b
     148  260 - #694d3f
     444  264 - #d6d6d6
     460  264 - #d6d6d6
     168  268 - #c27e56
     200  268 - #825135
     404  268 - #c79c53
     240  272 - #342924
     376  272 - #c59b52
     356  276 - #a98546
     424  276 - #d8d8d8
     336  280 - #e5b666
     488  280 - #dcdcdc
     112  284 - #363331
     152  284 - #b46841
     392  288 - #c1c1c1
     456  288 - #e2e2e2
     132  292 - #b46b3f
     196  292 - #875538
     220  292 - #a57641
     328  292 - #8b6d3a
     444  292 - #f0f0f0
     252  296 - #b5bcbd
     172  300 - #aa6a47
     272  300 - #2a2928
     336  300 - #e4b564
     452  300 - #e6e6e6
     456  300 - #e7e7e7
     460  300 - #e5e5e5
     376  308 - #ddaf5f
     148  312 - #945c3b
     204  312 - #a07059
     388  312 - #b9b9b9
     400  312 - #7f7f7f
     416  312 - #afafaf
     120  316 - #ad693e
     184  316 - #c07345
     240  316 - #5b5b5b
     284  316 - #282826
     240  320 - #666666
     500  320 - #878787
     236  324 - #7f6d68
     240  324 - #6a6a6a
     264  324 - #523a30
     288  324 - #272725
     452  324 - #8d8d8d
     468  324 - #8f8f8f
     240  328 - #656565
     244  328 - #6a6460
     348  328 - #c39950
     208  332 - #af6840
     240  332 - #59595a
     296  332 - #252525
     368  332 - #be964e
      40  336 - #bdc5cd
     184  336 - #342f2c
     292  336 - #262623
     300  336 - #533727
     400  336 - #ababab
     152  340 - #b0673d
     388  340 - #a6a6a6
     412  340 - #a0a0a0
     448  340 - #e6e6e6
     460  340 - #e9e9e9
     128  344 - #925736
     388  344 - #a1a1a1
     504  344 - #a8a8a8
     168  348 - #332f2c
     212  352 - #6e402a
     440  352 - #767676
     464  352 - #939393
     276  356 - #a16332
     512  356 - #d7d7d7
     384  360 - #848484
     404  360 - #b6b6b6
     240  364 - #27282a
     296  364 - #222121
     184  368 - #362b25
     508  368 - #767676
      92  372 - #c6cfd7
     492  372 - #898989
     420  376 - #949494
     440  376 - #767676
     460  380 - #767676
     208  384 - #7f4b2b
     288  384 - #25201e
     404  384 - #767676
     480  384 - #939393
     260  392 - #3a2920
     420  392 - #777777
     140  396 - #bfc7cf
     452  396 - #767676
     232  400 - #6d4028
       4  408 - #c8d1da
     104  436 - #bdc5cd
      44  460 - #bdc5cd
     592  464 - #a9b0b8
     468  532 - #aab2b9
       4  592 - #b0b8c0
     172  592 - #aeb5bd
     340  592 - #abb3ba
     592  592 - #a9b1b8
";
