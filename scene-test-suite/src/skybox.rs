use std::f32::consts::FRAC_PI_6;

use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::{
        LossyConvert,
        volume::{Shape3, Vec3},
    },
    refs::Weak,
    scene::{Camera, Material, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, Sky, scene},
    ui::Color,
    ui_test::{capture_screenshot, check_colors, set_record_probe_count},
};

const COLUMNS: usize = 5;
const SPACING: f32 = 1.5;
/// Frames of the 30 degree turn between the checks.
const TURN_FRAMES: usize = 15;

/// A gradient sky over a plane, with a row of chrome balls from mirror
/// to matte in front of a row of white ones. The sky has to fill the
/// frame behind the plane, the mirror ball has to show the blue zenith
/// on its top and the brown ground on its bottom, the rougher balls
/// blur that into a gradient, the white ones take their light from it,
/// blue above and brown below, and the turn moves every reflection with
/// the eye.
#[scene]
#[derive(Default)]
struct Skybox {}

impl SceneSetup for Skybox {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 2.5, 9.0),
            target: Vec3::new(0.0, 0.8, 0.0),
            ..Camera::default()
        };

        self.sky = Some(Sky::gradient(
            Color::hex("#2f6fd6"),
            Color::hex("#b9cfe8"),
            Color::hex("#4a4034"),
        ));

        self.make_node::<Prop>(Shape3::Plane(16.0), Vec3::ZERO)
            .set_color(Color::hex("#9aa5ad"))
            .set_roughness(0.85);

        for (row, (metallic, color, z)) in
            [(1.0, "#f2f2f2", 1.0), (0.0, "#e8e8e8", -1.5)].into_iter().enumerate()
        {
            for column in 0..COLUMNS {
                let x = (column.lossy_convert() - (COLUMNS - 1).lossy_convert() / 2.0) * SPACING;
                let y = 0.6 + row.lossy_convert() * 0.3;
                self.make_node::<Prop>(Shape3::Ball(0.6), Vec3::new(x, y, z))
                    .set_material(Material {
                        color: Color::hex(color),
                        metallic,
                        roughness: column.lossy_convert() / (COLUMNS - 1).lossy_convert(),
                        ..Material::default()
                    });
            }
        }
    }
}

impl SceneTest for Skybox {
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
       4    4 - #a8c0e4
     288    4 - #a4bde4
     392    4 - #a4bde4
     492    4 - #a6bee4
     592    4 - #a8c0e4
     148   88 - #b1c8e6
     440   92 - #b2c9e6
     316  128 - #b4cbe6
       4  164 - #b7cde7
     592  164 - #b7cde7
     148  252 - #c3d4f3
     236  256 - #c9d6f6
     292  256 - #c4d2f4
     320  264 - #cdd9f5
     396  264 - #ced9f5
     440  264 - #c5d2f2
     364  268 - #c5d3f1
     592  268 - #b6cce4
     164  276 - #cfdbf3
     476  276 - #cdd9f1
     192  280 - #8ea2c8
     252  280 - #c8d6ee
     108  288 - #819ed2
     204  288 - #7e98c8
     108  292 - #4373ca
     120  292 - #6286ce
     212  292 - #5a7dc3
     324  292 - #afbcd1
     108  296 - #2d68c9
     112  296 - #2e69c9
     220  296 - #6382c4
     308  296 - #a5b5e0
     104  300 - #2c68c9
     108  300 - #2c68c9
     112  300 - #2c68c9
     196  300 - #4570c2
     128  304 - #5d83cd
     184  304 - #6181c3
     208  304 - #d0d9f7
     240  304 - #828e9e
     304  304 - #e8ecfb
     308  304 - #e8edfb
     360  304 - #808d9e
     432  304 - #7e8b9b
     452  304 - #808e9f
      88  308 - #6286ce
     152  308 - #7c8694
     156  308 - #7c8694
     192  308 - #577ac4
     208  308 - #fefeff
     300  308 - #e6ebfa
     304  308 - #f1f4fc
     308  308 - #f3f5fc
     312  308 - #eaeefb
     400  308 - #cad3ed
     100  312 - #587fcc
     116  312 - #5f84cd
     208  312 - #fcfcfe
     304  312 - #eff3fc
     308  312 - #f1f4fc
     312  312 - #eaeefa
     468  312 - #7d8aa4
     208  316 - #cbd7f4
     304  316 - #e4eaf9
     308  316 - #e6ecf9
     456  324 - #5f6c82
     360  328 - #6f7d93
     492  328 - #818b9e
     472  332 - #6b7689
     268  336 - #7c8a9b
     388  336 - #818b9c
     328  340 - #7c8796
     456  340 - #5b6778
     516  340 - #858d99
      80  344 - #808b9a
     108  344 - #5f6166
     132  344 - #606267
     176  344 - #788391
     232  344 - #707984
     300  344 - #6a707a
     368  344 - #65707f
     380  344 - #626b78
     408  344 - #787f8b
     476  344 - #57606d
      84  348 - #6f7680
      88  348 - #63676c
     120  348 - #4b443f
     132  348 - #59595b
     180  348 - #6a717b
     200  348 - #4e4c4b
     208  348 - #4e4b4b
     216  348 - #4f4d4d
     228  348 - #62676e
     292  348 - #5b5f66
     308  348 - #5e6269
     488  348 - #555d69
     504  348 - #6b727c
      92  352 - #555353
     100  352 - #49423a
     204  352 - #463f39
     220  352 - #4e4c4c
     280  352 - #606771
     316  352 - #5c6169
     392  352 - #585d66
     468  352 - #57606e
     108  356 - #463d33
     196  356 - #4a4643
     212  356 - #48433f
     296  356 - #505155
     380  356 - #616976
     412  356 - #626c7a
     108  360 - #5b5b5e
     112  360 - #5e5f63
     204  360 - #5a5c61
     400  360 - #616b78
     488  360 - #57606d
     500  360 - #5a6474
       4  456 - #8da2c8
     384  456 - #8da2c8
     592  456 - #8da2c8
     108  464 - #8da2c8
     288  480 - #8da2c8
     468  504 - #8da2c8
     176  540 - #8da2c8
       4  592 - #8da2c8
     348  592 - #8da2c8
     496  592 - #8da2c8
     592  592 - #8da2c8
";

const SIDE: &str = r"
       4    4 - #a8c0e4
     204    4 - #a5bde4
     308    4 - #a4bde4
     592    4 - #a8c0e4
     156   96 - #b2c8e6
     452   96 - #b2c9e6
     324  132 - #b5cbe6
     592  152 - #b7cde7
       4  168 - #b8cee7
     216  248 - #c6d3f6
     284  252 - #c5d4f6
     348  256 - #c7d4f5
     420  260 - #c9d5f5
     316  268 - #c5d3f1
     136  276 - #517bcc
     244  276 - #a6bad7
     476  276 - #c5d3f2
       4  280 - #b6cbe4
     128  280 - #356bc9
     140  280 - #326ac9
     196  280 - #7792c7
     124  284 - #386dca
     136  284 - #2c68c9
     140  284 - #2e69c9
     144  284 - #366cc9
     224  284 - #afbbcf
     392  284 - #c7d4ee
     528  284 - #cad6f1
     128  288 - #366cc9
     140  288 - #366cca
     152  288 - #6688ce
     204  288 - #446fc2
     228  288 - #909eb2
     232  288 - #8694a9
     268  288 - #8196c3
     112  292 - #88a2d2
     156  292 - #89a3d3
     184  292 - #5378c5
     196  292 - #879ede
     208  292 - #4e75c2
     228  292 - #818e9f
     232  292 - #828fa0
     256  292 - #7f94c4
     280  292 - #7f94c4
     172  296 - #829ac8
     268  296 - #adbbe7
     188  300 - #bfccf2
     192  300 - #fcfcfe
     196  300 - #fefeff
     200  300 - #d5ddf8
     216  300 - #7b95c7
     248  300 - #8196c3
     288  300 - #8297c4
     208  304 - #8099ca
     268  304 - #ecf0fc
     272  304 - #e7ecfa
     320  304 - #8491a4
     264  308 - #eef2fc
     268  308 - #f3f6fd
     272  308 - #eff2fc
     264  312 - #ebeffb
     268  312 - #f0f3fc
     272  312 - #ebeffb
     392  312 - #8290a2
     360  316 - #cbd4ed
     112  320 - #7f8a98
     116  320 - #757d88
     160  320 - #7b8591
     256  320 - #abbad4
     280  320 - #adbbd5
     408  320 - #78828e
     412  320 - #78828f
     116  324 - #6a6f77
     128  324 - #4e4946
     140  324 - #49423b
     148  324 - #4d4844
     508  324 - #808d9f
     144  328 - #473e34
     168  328 - #8592a2
     224  328 - #7d8998
     504  328 - #7b8694
     136  332 - #4a433c
     172  332 - #747e8b
     176  332 - #686f78
     220  332 - #6c737e
     296  332 - #8290a2
     316  332 - #838fa4
     180  336 - #57595c
     192  336 - #48443f
     200  336 - #494440
     208  336 - #4c4846
     188  340 - #4a4643
     204  340 - #47423d
     248  340 - #727b88
     280  340 - #707884
     196  344 - #555557
     416  344 - #7f899b
     260  348 - #595c61
     284  348 - #5a5f66
     320  352 - #6b7788
     388  352 - #7a8595
     444  352 - #8690a2
     272  356 - #595d63
     340  356 - #737b86
     464  356 - #8891a2
     360  360 - #6f757e
     372  360 - #6e7580
     416  360 - #5e6a7d
     432  364 - #77808e
     348  368 - #595f68
     356  372 - #5f6773
     432  376 - #565f6c
     464  376 - #757b84
     488  376 - #7b818c
     440  380 - #555c68
     444  388 - #565e6b
     456  388 - #535a65
     472  388 - #555e6a
       4  412 - #8da2c8
     592  460 - #8da2c8
     124  488 - #8da2c8
     312  488 - #8da2c8
     496  496 - #8da2c8
     416  560 - #8da2c8
       4  592 - #8da2c8
     136  592 - #8da2c8
     244  592 - #8da2c8
     592  592 - #8da2c8
";
