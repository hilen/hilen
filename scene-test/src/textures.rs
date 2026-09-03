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
     232    4 - #597c95
     592    4 - #597c95
     412   48 - #597c95
     120   92 - #597c95
     592  156 - #597c95
     288  224 - #7b8987
     308  224 - #7b8987
     336  228 - #e3b35e
     360  228 - #daab5a
     108  232 - #bd8644
     156  232 - #79533e
     132  236 - #a5653e
      84  240 - #a96c40
     180  240 - #a4623b
     300  240 - #deaf5c
     212  244 - #5c4336
     316  244 - #c79d53
     348  244 - #c59b52
     284  248 - #c99e54
     364  252 - #a17e43
     104  256 - #483d36
     420  256 - #b5b5b5
     476  256 - #cbcbcb
     304  260 - #c49a52
     324  260 - #8e703b
     164  264 - #c58446
     208  264 - #8d5a37
     444  264 - #c1c1c1
     288  268 - #d5a859
     320  268 - #d2a658
     340  268 - #d2a657
     116  272 - #e1a050
     136  272 - #d7954b
     456  272 - #e1e1e1
     476  272 - #dedede
     192  276 - #483a32
     368  276 - #b6a584
     400  276 - #8b8b8b
     464  276 - #f0f0f0
     504  276 - #c4c4c4
      88  280 - #e48c4f
     292  280 - #8f703b
     488  280 - #e1e1e1
     228  284 - #8b9096
     312  284 - #734b30
     460  284 - #e5e5e5
     340  288 - #d2a557
     396  288 - #d0d0d0
     428  288 - #7c7c7c
     444  288 - #a6a6a6
     468  288 - #dedede
     480  288 - #b7b7b7
     160  296 - #313031
     384  296 - #969a9d
     120  300 - #b16b40
     140  300 - #b46d44
     208  300 - #2e2d2d
     280  300 - #302d2a
      96  304 - #d08449
     304  304 - #383634
     352  304 - #cca055
     408  304 - #939393
     252  308 - #9d613b
     188  312 - #312e2d
     440  312 - #9b9b9b
     452  312 - #989898
     412  316 - #999999
     484  316 - #767676
     248  320 - #9e643d
     592  320 - #a8b0b7
     168  324 - #ad6c47
     220  324 - #69432e
     316  324 - #ad7e68
     396  324 - #7f7f7f
     428  324 - #dcdcdc
     492  324 - #dcdcdc
     104  328 - #8c5839
     316  328 - #b8846d
     320  328 - #665d59
       4  332 - #adb5bc
     268  332 - #555557
     340  332 - #b77947
     476  332 - #c2c2c2
     148  336 - #985b38
     268  336 - #535352
     456  336 - #9b9b9b
     496  336 - #999999
     360  340 - #ae6a37
     464  340 - #999999
     296  344 - #5c4235
     408  344 - #7a7a7a
     184  348 - #6d4732
     204  348 - #553e2f
     244  348 - #a3603b
     432  348 - #9a9a9a
     476  348 - #b7b7b7
      96  352 - #302f2e
     116  356 - #564134
     160  356 - #49382d
     220  360 - #2c2c2c
     332  360 - #a66036
     452  360 - #838587
     344  364 - #262729
     360  364 - #794c2d
     260  368 - #2e2824
     140  372 - #b57238
     300  372 - #9a603d
     192  376 - #2e2c2b
     340  376 - #252626
     168  380 - #2e2c2a
     268  384 - #272423
     112  388 - #905736
     336  396 - #252221
     264  400 - #352721
     312  400 - #38261d
     272  404 - #221f1e
     332  404 - #2b2420
     592  444 - #aab1b9
       4  460 - #c8d1d9
     116  512 - #c3cbd4
     276  512 - #b6bdc5
     452  512 - #adb5bc
       4  592 - #bfc7d0
     208  592 - #b6bec6
     344  592 - #b0b8c0
     500  592 - #acb4bb
     592  592 - #abb2ba
";

const SIDE: &str = r"
       4    4 - #597c95
     332    4 - #597c95
     592    4 - #597c95
     168   44 - #597c95
     464  100 - #597c95
       4  144 - #597c95
     160  212 - #606466
     184  220 - #905a3b
     248  220 - #503627
     136  224 - #98603b
     228  224 - #815137
     208  228 - #9f5f3a
     372  232 - #dfaf5c
     108  236 - #3d3e3e
     348  240 - #e6b55f
     180  244 - #c28045
     376  244 - #e6b55f
     400  244 - #e5b45f
     132  248 - #de9f50
     156  248 - #92683e
     236  248 - #362a24
     384  248 - #e5b55f
     408  248 - #e6b560
     328  252 - #886b39
     428  252 - #bf964e
     360  256 - #8e6f3b
     112  260 - #dd854b
     216  260 - #312f30
     252  264 - #939294
     444  264 - #d6d6d6
     460  264 - #d6d6d6
     184  268 - #a36a4a
     240  272 - #342924
     372  272 - #c59b52
     136  276 - #b36a41
     356  276 - #a98546
     396  276 - #c39a51
     424  276 - #d8d8d8
     560  276 - #8196a6
     336  280 - #e5b666
     488  280 - #dcdcdc
     156  284 - #b06841
     228  288 - #d49446
     424  288 - #b0b0b0
     456  288 - #e2e2e2
     116  292 - #ba7541
     208  292 - #614637
     328  292 - #8b6d3a
     444  292 - #f0f0f0
     252  296 - #b5bcbd
     356  296 - #b9914c
     388  296 - #8c8c8c
     468  296 - #dddddd
     140  300 - #b36b40
     172  300 - #aa6a47
     336  300 - #e4b564
     228  308 - #4a413b
     376  308 - #ddaf5f
     188  312 - #ca7749
     388  312 - #b9b9b9
     448  312 - #dcdcdc
     120  316 - #ad693e
     264  316 - #8f6641
     284  316 - #282826
     416  316 - #848484
     496  316 - #b2b2b2
     288  324 - #272725
     460  324 - #999999
     468  324 - #8f8f8f
     176  328 - #7c4d33
     244  328 - #6a6460
     348  328 - #c39950
     516  328 - #dadada
     136  332 - #302e2d
     204  332 - #9f6541
     296  332 - #252525
     376  332 - #8b8b8b
      40  336 - #bdc5cd
     292  336 - #262623
     300  336 - #533727
     400  336 - #ababab
     152  340 - #b0673d
     388  340 - #a6a6a6
     412  340 - #a0a0a0
     440  340 - #d9d9d9
     268  344 - #4d3428
     388  344 - #a1a1a1
     504  344 - #a8a8a8
     172  348 - #332e2b
     400  348 - #acacac
     220  352 - #a46138
     452  352 - #909090
     468  352 - #8e8e8e
     296  360 - #222121
     384  360 - #848484
     240  364 - #27282a
     496  364 - #c9c9c9
     184  368 - #362b25
     264  368 - #794f31
     400  368 - #767676
     456  368 - #dcdcdc
     508  368 - #767676
     412  372 - #797979
      92  376 - #c6cfd7
     212  376 - #4e3021
     440  376 - #767676
     492  376 - #767676
     468  380 - #777777
     288  384 - #25201e
     404  384 - #767676
     428  384 - #878787
     452  384 - #969696
     416  388 - #7a7a7a
     260  392 - #3a2920
     480  392 - #767676
     460  396 - #767676
     232  400 - #6d4028
     444  400 - #909498
       4  408 - #c8d1da
     148  408 - #bcc4cc
     104  440 - #bcc4cc
      40  464 - #bcc4cc
     592  464 - #a9b0b8
     468  536 - #aab1b9
       4  592 - #b0b8c0
     176  592 - #adb5bc
     344  592 - #abb2ba
     592  592 - #a9b0b8
";
