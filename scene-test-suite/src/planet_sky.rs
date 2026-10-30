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
        404  292 - #846f85
        320  300 - #805d63
        356  304 - #816677
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
        556   16 - #878483
        584   16 - #868483
        280   40 - #2c3949
        132   92 - #303e4d
        424  112 - #33404f
        336  172 - #404b57
        532  204 - #3e4c58
         32  208 - #3d4c58
        240  244 - #5c6063
        156  272 - #5a6163
        436  272 - #5b6264
        320  276 - #7b766c
        384  296 - #7a766c
         96  300 - #5a6363
        208  300 - #7b776c
        588  304 - #525e5f
        264  308 - #9c8d75
        236  324 - #a29175
        316  324 - #b8a17c
        276  332 - #c2a87e
        348  336 - #bca37a
        300  340 - #fedfba
        300  352 - #f1c482
        316  352 - #ecc07f
        336  352 - #e4ba7b
        392  352 - #b49769
        428  352 - #9e8863
        492  352 - #7f7259
         60  356 - #4a402c
        164  356 - #605033
        200  356 - #6c5837
        232  356 - #78613c
        236  356 - #7a633d
        272  356 - #8a6f44
        352  356 - #7e663f
        556  356 - #453d2a
        120  360 - #493e27
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
      140    4 - #020305
      280    4 - #020305
      416   16 - #020305
      556   16 - #e5d9cb
      584   16 - #e5d9ca
        4   36 - #030405
      164   80 - #040507
      340   84 - #040507
      576   88 - #040506
       88   96 - #040507
      492   96 - #040507
      240  100 - #050608
      412  156 - #07090a
      568  160 - #060809
      160  168 - #07090b
       20  172 - #06080a
      288  192 - #090b0d
      488  216 - #090c0d
      576  232 - #0a0c0d
       92  236 - #0b0d0e
      212  240 - #0d0f10
      360  252 - #0e1111
      516  288 - #101210
      440  300 - #131511
      588  304 - #12130f
      156  308 - #141511
       16  312 - #13140f
      296  320 - #000000
      512  368 - #000100
      376  380 - #000100
       96  388 - #000100
      220  388 - #000100
      592  444 - #000100
      452  448 - #000100
       20  452 - #000100
      304  456 - #000100
      160  468 - #000100
       88  476 - #000100
      236  516 - #000100
      380  516 - #000100
      528  516 - #000100
       28  524 - #000100
       96  548 - #000100
      456  584 - #000100
        4  592 - #000100
      188  592 - #000100
      316  592 - #000100
      592  592 - #000100
";

/// The planet from 60 km, the line between day and night, the ring of air.
const FROM_SPACE: &str = r"
      592    4 - #000000
      140   52 - #e6e8f1
      304  120 - #374b73
      348  124 - #283a5f
      232  128 - #1b2846
      372  128 - #16233c
      384  128 - #e2e2eb
      360  132 - #435b83
      392  136 - #101c31
      196  164 - #2e3f58
      264  172 - #3b4640
      408  196 - #5c6f59
      152  204 - #0e141e
      452  216 - #6e859e
      460  228 - #5d789a
      356  232 - #546649
      240  236 - #374137
      472  248 - #384e77
      460  268 - #657a69
      200  284 - #242b32
      472  288 - #8d8c9c
      136  292 - #1c201d
      272  296 - #414e3b
      460  304 - #647965
      476  320 - #4a658d
      460  324 - #647967
      388  344 - #586a4d
      204  348 - #222932
      472  348 - #3e567f
      468  356 - #4a648c
      316  360 - #4a5941
      468  368 - #2b3e64
      256  376 - #384238
      456  380 - #597497
      444  396 - #6d839b
        4  412 - #000000
      284  428 - #3a453f
      332  428 - #475547
      196  436 - #2f393f
      376  456 - #637991
      368  460 - #607690
      392  460 - #192743
      360  468 - #3b5179
      380  468 - #132038
      308  476 - #4a5f7e
      244  480 - #0d1629
      568  580 - #e6e8f1
        4  592 - #000000
";
