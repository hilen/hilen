use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Shape3, Vec3},
    refs::Weak,
    scene::{
        Atmosphere, Camera, NodeTemplates, Occluder, Prop, SceneCreation, SceneSetup, SceneTest, SpaceSky,
        Sun, scene,
    },
    ui::Color,
    ui_test::{check_colors, set_record_probe_count},
};

const PLANET: f32 = 20_000.0;
const AIR: f32 = 5000.0;
const CENTER: Vec3 = Vec3::new(0.0, -PLANET, 0.0);

/// A camera 2 m over the ground of a planet of 20 km with 5 km of air,
/// the air of Earth shrunk to it, looking out along the ground at posts
/// that fade into the haze. At noon the sky is blue and pales toward the
/// horizon. At sunset the sky toward the sun glows red and the light on
/// the ground turns warm. At night the sky is dark and the stars show.
/// Last the camera goes 60 km out into space and sees the whole planet,
/// its day side, its night side, and the blue ring of its air.
#[scene]
#[derive(Default)]
struct PlanetSky {}

/// The camera on the ground looking toward +x and a little up.
fn on_the_ground() -> Camera {
    Camera {
        position: Vec3::new(0.0, 2.0, 0.0),
        target: Vec3::new(100.0, 12.0, 0.0),
        far: f32::INFINITY,
        ..Camera::default()
    }
}

fn sun_toward(to_sun: Vec3) -> Sun {
    Sun {
        direction: -to_sun.normalize(),
        intensity: 1.0,
        ..Sun::default()
    }
}

impl SceneSetup for PlanetSky {
    fn setup(&mut self) {
        self.camera = on_the_ground();
        self.space_sky = Some(SpaceSky {
            atmosphere: Some(Atmosphere::scaled(CENTER, PLANET, AIR)),
            ..SpaceSky::default()
        });
        self.occluders = vec![Occluder::new(CENTER, PLANET)];
        self.sun = sun_toward(Vec3::new(-0.3, 1.0, 0.4));
        self.make_node::<Prop>(Shape3::Plane(6000.0), Vec3::ZERO)
            .set_color(Color::hex("#6f8f5a"))
            .set_roughness(0.9);
        // Each post twice as far and twice as big, each a little more to
        // the right, so the same post fades step by step into the haze.
        for step in 0..8_u8 {
            let x = 20.0 * 2.0_f32.powi(i32::from(step));
            let side = (f32::from(step) * 0.07 - 0.3) * x;
            self.make_node::<Prop>(
                Shape3::cuboid(x * 0.03, x * 0.12, x * 0.03),
                Vec3::new(x, x * 0.06, side),
            )
            .set_color(Color::hex("#c0703a"))
            .set_roughness(0.8);
        }
    }
}

impl SceneTest for PlanetSky {
    fn perform_test(mut scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(48);
        wait_for_next_frame();
        check_colors(NOON)?;

        from_main(move || scene.sun = sun_toward(Vec3::new(1.0, 0.02, 0.0)));
        wait_for_next_frame();
        check_colors(SUNSET)?;

        from_main(move || scene.sun = sun_toward(Vec3::new(1.0, -0.4, 0.0)));
        wait_for_next_frame();
        check_colors(NIGHT)?;

        from_main(move || {
            scene.sun = sun_toward(Vec3::new(1.0, 0.3, 0.6));
            scene.camera = Camera {
                position: Vec3::new(0.0, 0.0, 60_000.0),
                target: CENTER,
                far: f32::INFINITY,
                ..Camera::default()
            };
            scene
                .make_node::<Prop>(Shape3::Ball(PLANET), CENTER)
                .set_color(Color::hex("#5f7f4a"))
                .set_roughness(0.9);
        });
        wait_for_next_frame();
        check_colors(FROM_SPACE)
    }
}

/// A blue sky paling toward the horizon, the far posts in the haze.
const NOON: &str = r"
       4    4 - #2c4365
     544    4 - #2e4567
     332    8 - #2b4263
     168   52 - #2d4466
      64  100 - #31496c
     592  100 - #334c6f
     276  124 - #31496c
     464  128 - #334b6f
     592  208 - #3d587c
      60  216 - #3d587c
     172  224 - #3d587c
     248  228 - #3d587c
     352  228 - #3d587c
     456  232 - #3e597d
     532  256 - #435f83
     592  288 - #4a6789
     284  292 - #805652
     404  292 - #836f84
     320  300 - #805d63
     356  304 - #816676
       4  312 - #526f8f
      56  312 - #516e8f
     248  312 - #805144
     208  316 - #804e3a
      96  320 - #537090
     500  324 - #536f8e
     276  332 - #557290
     152  340 - #597592
     424  340 - #577490
     336  344 - #597591
      12  348 - #5c7993
     376  348 - #5a7691
     532  348 - #5a7590
      48  352 - #6f878f
     116  352 - #6f878e
     468  352 - #6e848c
     572  352 - #6f858c
     244  356 - #7c5f4d
     300  356 - #6f8866
     188  372 - #8d5538
     140  404 - #804c30
      28  472 - #6e8759
     272  484 - #6e8758
     468  492 - #6e8759
     176  576 - #6d8758
       4  592 - #6d8758
     348  592 - #6d8758
     592  592 - #6d8758
";

/// The sky red toward the setting sun, the ground lit warm.
const SUNSET: &str = r"
       4    4 - #283646
     180    4 - #293646
     356    4 - #293646
     592    4 - #283646
     200  176 - #3f4b57
     532  204 - #3e4c58
      28  208 - #3d4c58
     292  236 - #5c6062
     156  272 - #5a6163
     440  272 - #5a6163
     332  280 - #7d776d
     244  288 - #7e786d
     384  296 - #7a766c
      96  300 - #5a6363
     592  300 - #505c5f
     304  304 - #9d8e76
     232  320 - #9a8c73
     316  324 - #b8a17c
     268  328 - #b9a17b
     348  336 - #bca37a
     300  340 - #fedfba
     372  348 - #bca277
     300  352 - #f1c481
     316  352 - #ecbf7e
     336  352 - #e3b97a
     392  352 - #b39567
     432  352 - #9b8661
     496  352 - #7d7157
      60  356 - #4a402c
     164  356 - #605033
     200  356 - #6c5837
     232  356 - #78613c
     236  356 - #7a633d
     272  356 - #8a6f44
     348  356 - #80673f
     560  356 - #453c2a
     120  360 - #493e27
     284  360 - #80663d
     296  364 - #81673d
       4  380 - #292719
     324  396 - #493d24
     196  440 - #282718
     372  460 - #292718
     264  484 - #282718
     504  488 - #1c1f14
      68  592 - #161d13
     348  592 - #1b1f14
     592  592 - #151c12
";

/// A dark sky with stars.
const NIGHT: &str = r"
     112    4 - #020305
     280    4 - #020305
     364    4 - #020305
     508    4 - #020305
     592    4 - #020304
     196    8 - #020305
       4   28 - #030405
     436   48 - #030406
      72   76 - #030506
     228   88 - #040507
     504  100 - #040507
     324  112 - #050608
     592  112 - #040607
     136  124 - #050708
      16  140 - #dfceb7
     428  140 - #060809
     236  188 - #090b0d
     516  188 - #dbc8ad
     376  216 - #0b0d0e
     144  220 - #0b0d0e
       4  240 - #0b0d0d
     468  256 - #0d0f10
     592  284 - #0f110e
     248  288 - #111412
      84  296 - #c4a679
     164  300 - #131511
     400  300 - #000000
     480  336 - #17170f
       4  340 - #15150d
     328  352 - #000000
     560  360 - #000100
     192  376 - #000100
      96  396 - #000100
     408  396 - #000100
     268  412 - #000100
       4  432 - #000100
     592  436 - #000100
     456  472 - #000100
     320  480 - #000100
     536  504 - #000100
      84  512 - #000100
     196  512 - #000100
     468  560 - #000100
       4  592 - #000100
     140  592 - #000100
     276  592 - #000100
     388  592 - #000100
     552  592 - #000100
";

/// The planet from 60 km, the line between day and night, the ring of air.
const FROM_SPACE: &str = r"
       4    4 - #000000
     416  116 - #e6e8f1
     264  124 - #384c70
     240  128 - #29375a
     348  128 - #425981
     216  136 - #1e2946
     368  136 - #4a648a
     392  152 - #687e98
     188  160 - #202939
     260  192 - #696c67
     216  196 - #494d55
     168  208 - #232a36
     464  212 - #16233d
     320  224 - #878b7b
     460  232 - #6c859f
     392  236 - #9ca18d
     208  240 - #494c52
     256  244 - #6c6f66
     472  248 - #384e77
       4  276 - #000000
     224  280 - #595b58
     320  284 - #888c7a
     160  296 - #212a35
     436  296 - #a5aa96
     260  304 - #6e7166
     376  308 - #989d89
     208  320 - #494b50
     236  336 - #5d5f5b
     472  344 - #466089
     468  356 - #4a648c
     392  364 - #989d8a
     236  368 - #585a59
     472  368 - #192642
     152  372 - #12151e
     336  384 - #868a7a
     464  388 - #142139
     444  396 - #6d839b
     224  412 - #3d4350
     268  432 - #5b5e60
     188  444 - #0f1520
     392  448 - #5e7693
     380  468 - #132038
     352  472 - #32466d
     240  480 - #0a1223
     296  480 - #2f4166
     248  564 - #e6e8f1
       4  592 - #000000
     592  592 - #000000
";
