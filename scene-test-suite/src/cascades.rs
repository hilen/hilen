use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::{
        LossyConvert,
        volume::{Shape3, Vec3},
    },
    refs::Weak,
    scene::{Camera, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, scene},
    ui::Color,
    ui_test::{capture_screenshot, check_colors, set_record_probe_count},
};

/// Frames of each walk between the checks.
const WALK_FRAMES: usize = 20;
/// How far the camera walks down the field in all, and how far it
/// stops in for the middle check.
const WALK: f32 = 120.0;
const SEAM: f32 = 8.0;
/// How far the shadows reach. The field is far longer, so the last
/// cascade stops here instead of covering it whole with coarse texels,
/// and the posts past it stand in the sun.
const SHADOW_DISTANCE: f32 = 200.0;

/// A field hundreds of units long seen from above down its length, a
/// thin pole close to the camera and a row of thick posts marching
/// away, under a low sun from the side. One map over the whole field
/// would smear the pole's shadow into a faint band, the near cascade
/// keeps it crisp, while the posts further off still throw shadows
/// through the coarser cascades up to the shadow distance. The camera
/// then walks down the field, so the cascades refit around it and the
/// shadows stay where they fell. The middle check stops where the near
/// post's shadow once broke off: the floor there is too coarse a pixel
/// for the maps that hold it and the last map does not reach back that
/// far, so the lookup has to keep the coarsest map that holds the
/// point instead of falling through to the sun.
#[scene]
#[derive(Default)]
struct Cascades {}

impl SceneSetup for Cascades {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 12.0, 12.0),
            target: Vec3::new(0.0, 0.0, -60.0),
            ..Camera::default()
        };
        self.sun.direction = Vec3::new(-1.0, -0.45, -0.2);
        self.sun.shadows = true;
        // The default is 1024 on a phone and in the browser.
        self.sun.shadow_map_size = 2048;
        self.sun.shadow_distance = SHADOW_DISTANCE;

        self.make_node::<Prop>(Shape3::Plane(900.0), Vec3::ZERO)
            .set_color(Color::hex("#cdd1d4"))
            .set_roughness(0.9);

        self.make_node::<Prop>(Shape3::cuboid(0.4, 3.0, 0.4), Vec3::new(1.6, 1.5, -4.0))
            .set_color(Color::hex("#c0392b"))
            .set_roughness(0.6);

        for step in 0..8u8 {
            let z = -20.0 - f32::from(step) * 40.0;
            let x = if step % 2 == 0 { 6.0 } else { -6.0 };
            self.make_node::<Prop>(Shape3::cuboid(2.5, 10.0, 2.5), Vec3::new(x, 5.0, z))
                .set_color(Color::hex("#e67e22"))
                .set_roughness(0.6);
        }

        self.make_node::<Prop>(Shape3::Ball(8.0), Vec3::new(-4.0, 8.0, -380.0))
            .set_color(Color::hex("#2980b9"))
            .set_roughness(0.4);
    }
}

impl SceneTest for Cascades {
    fn perform_test(mut scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);

        wait_for_next_frame();
        check_colors(START)?;

        let frames: f32 = WALK_FRAMES.lossy_convert();
        let walk = move |distance: f32| {
            let step = distance / frames;
            for _ in 0..WALK_FRAMES {
                from_main(move || {
                    // Level, the line of sight points down at the floor.
                    let mut ahead = scene.camera.target - scene.camera.position;
                    ahead.y = 0.0;
                    let ahead = ahead.normalize() * step;
                    scene.camera.position += ahead;
                    scene.camera.target += ahead;
                });
                wait_for_next_frame();
            }
        };

        walk(SEAM);
        check_colors(SEAM_CHECK)?;

        walk(WALK - SEAM);
        capture_screenshot()?;
        check_colors(WALKED)
    }
}

const START: &str = r"
       4    4 - #597c95
     272    4 - #597c95
     372    4 - #597c95
     592    4 - #597c95
     484   40 - #597c95
     188   48 - #597c95
      92   56 - #597c95
     356   72 - #597c95
     592   88 - #597c95
     292   96 - #597c95
     144  100 - #597c95
     476  112 - #597c95
      16  116 - #597c95
     404  124 - #597c95
     212  148 - #597c95
     536  152 - #597c95
     120  164 - #597c95
     368  184 - #597c95
     460  184 - #597c95
     288  212 - #214865
     292  212 - #26587e
     296  212 - #2e6e9c
     300  212 - #3b7fb3
     292  216 - #245071
     296  216 - #2d6792
     592  216 - #597c95
     292  220 - #214865
     296  220 - #285d85
     304  220 - #327cb0
     168  224 - #597c95
     292  224 - #214865
     300  224 - #2a6590
       4  228 - #a2a5a7
     268  228 - #a2a5a7
     272  228 - #a2a5a7
     296  228 - #214865
     312  228 - #995625
      84  232 - #a2a5a7
     268  232 - #a2a5a7
     272  232 - #a2a5a7
     272  236 - #707274
     244  240 - #707274
     268  240 - #707274
     272  240 - #707274
     268  244 - #a1a4a7
     272  244 - #a1a4a7
     332  244 - #995625
     272  248 - #a1a4a7
     408  248 - #995626
     508  248 - #a2a5a7
     268  252 - #a1a4a7
     376  264 - #7e471f
     244  268 - #707274
     300  268 - #707274
     332  268 - #995625
     420  268 - #995626
     264  276 - #995624
     396  280 - #995625
     420  292 - #995626
     108  296 - #707274
     132  296 - #707274
     164  296 - #707274
     192  296 - #707274
     224  300 - #707274
     260  300 - #985624
     376  300 - #7e471f
     592  300 - #a2a5a7
       4  316 - #a1a4a6
     408  316 - #995625
     384  332 - #995625
     416  344 - #995625
     512  352 - #a1a4a6
     372  360 - #7e471f
     400  364 - #995625
      96  376 - #707274
     136  380 - #707274
      60  384 - #707274
     376  384 - #7e471f
     172  388 - #707274
     248  388 - #707274
     292  388 - #707274
     416  388 - #995625
     592  388 - #a1a4a6
     208  396 - #707274
     336  396 - #707274
     312  400 - #707274
     380  408 - #995625
     404  408 - #995625
     468  432 - #a1a4a6
       4  440 - #a1a4a6
     532  448 - #a1a4a6
     140  464 - #a0a4a6
     256  468 - #a0a4a6
     352  488 - #96352d
     348  492 - #802e28
     592  492 - #a1a4a6
      60  496 - #a0a3a6
     344  496 - #802e28
     448  496 - #a1a4a6
     352  500 - #802e28
     344  504 - #802e28
     348  508 - #802e28
     352  512 - #802e28
     344  516 - #802e28
     352  520 - #802e28
     348  524 - #802e28
     344  528 - #802e28
     352  528 - #802e28
     348  532 - #802e28
     528  532 - #a1a4a6
     352  536 - #802e28
     344  540 - #802e27
     164  544 - #707274
     348  544 - #802e27
     188  548 - #707274
     212  548 - #707274
     352  548 - #802e27
     344  552 - #802e27
     416  552 - #a0a3a6
     256  556 - #707274
     348  560 - #802e27
     300  564 - #707274
     352  568 - #802e27
     344  572 - #802e27
       4  592 - #a0a3a6
      92  592 - #a0a3a6
     468  592 - #a0a4a6
     592  592 - #a1a4a6
";

const SEAM_CHECK: &str = r"
       4    4 - #597c95
     268    4 - #597c95
     372    4 - #597c95
     592    4 - #597c95
     100    8 - #597c95
     480   28 - #597c95
      52   48 - #597c95
     188   48 - #597c95
     352   64 - #597c95
     416   76 - #597c95
     528   84 - #597c95
     236   88 - #597c95
     296   92 - #597c95
     124   96 - #597c95
      48  112 - #597c95
     592  120 - #597c95
     480  140 - #597c95
     232  148 - #597c95
     380  148 - #597c95
     316  152 - #597c95
       4  164 - #597c95
      80  172 - #597c95
     148  172 - #597c95
     432  192 - #597c95
     548  196 - #597c95
     288  212 - #214865
     292  212 - #26597f
     296  212 - #2e6e9c
     300  212 - #3b7fb3
     284  216 - #214865
     288  216 - #214865
     292  216 - #245173
     296  216 - #2e6893
     292  220 - #214865
     296  220 - #295e86
     304  220 - #327cb1
     292  224 - #214865
     300  224 - #2a6691
      32  228 - #a2a5a7
     296  228 - #214865
     312  228 - #995625
     108  232 - #a2a5a7
     264  232 - #a2a5a7
     268  232 - #a2a5a7
     272  232 - #a2a5a7
     264  236 - #a2a5a7
     268  236 - #a2a5a7
     272  236 - #a2a5a7
     300  236 - #a2a5a7
     240  244 - #995624
     272  244 - #a1a4a7
     332  244 - #995625
     264  248 - #a1a4a7
     268  248 - #a1a4a7
     264  252 - #a1a4a7
     268  252 - #a1a4a7
     272  252 - #a1a4a7
     208  256 - #707274
     404  256 - #b3652a
     432  256 - #b3652b
     460  260 - #995626
     236  272 - #707274
     300  272 - #707274
     332  272 - #995625
     592  272 - #a2a5a7
     260  280 - #995624
     420  284 - #995626
     460  288 - #995626
     524  288 - #a1a4a7
      88  304 - #707274
     124  304 - #707274
     152  308 - #707274
     184  308 - #707274
     224  308 - #707274
     260  308 - #985624
     444  308 - #995626
     396  312 - #7e471f
       4  328 - #a1a4a6
     460  332 - #995626
     412  336 - #995625
     440  352 - #995625
     544  356 - #a1a4a6
     396  360 - #7e471f
     316  364 - #a1a4a6
     136  368 - #a1a4a6
     200  376 - #a1a4a6
     424  380 - #995625
     400  384 - #7e471f
     456  392 - #995625
     408  408 - #995625
      52  416 - #707274
     104  420 - #707274
     520  420 - #a1a4a6
     152  428 - #707274
     180  428 - #707274
       4  432 - #707274
     216  432 - #707274
     244  432 - #707274
     440  432 - #995625
      32  436 - #707274
     412  436 - #995625
      76  440 - #707274
     296  440 - #707274
     328  440 - #707274
     124  444 - #707274
     368  444 - #707274
     592  444 - #a1a4a6
     196  452 - #707274
     268  456 - #707274
     396  456 - #7e471f
     312  460 - #707274
     424  460 - #995624
     344  464 - #707274
     452  468 - #995625
     528  488 - #a1a4a6
      88  504 - #a0a3a6
     256  512 - #a0a3a6
     592  520 - #a1a4a6
     396  524 - #a0a4a6
       4  532 - #a0a3a6
     172  548 - #a0a3a6
     332  548 - #a0a3a6
     496  560 - #a1a4a6
      20  592 - #a0a3a6
      84  592 - #a0a3a6
     264  592 - #a0a3a6
     400  592 - #a0a3a6
     592  592 - #a1a4a6
";

const WALKED: &str = r"
       4    4 - #597c95
     224    4 - #597c95
     436    4 - #597c95
     520    4 - #597c95
     304    8 - #597c95
     112   28 - #597c95
     592   36 - #597c95
     372   52 - #597c95
     460   80 - #597c95
      32   96 - #597c95
     276  100 - #597c95
     592  116 - #597c95
     160  128 - #597c95
     376  152 - #597c95
     228  160 - #597c95
     472  172 - #597c95
      40  192 - #597c95
     120  204 - #597c95
     288  208 - #26597f
     292  208 - #2b6894
     296  208 - #2f74a5
     560  208 - #597c95
     284  212 - #214865
     288  212 - #255579
     292  212 - #2a6590
     296  212 - #3273a2
     300  212 - #3d80b2
     280  216 - #214865
     284  216 - #214865
     288  216 - #244f71
     292  216 - #296089
     304  216 - #4286ba
     288  220 - #214865
     292  220 - #275a80
     296  220 - #2f6995
     304  220 - #3c80b4
     288  224 - #214865
     292  224 - #255275
     296  224 - #2a628b
     300  224 - #2f6e9c
     304  224 - #3177aa
     288  228 - #214865
     292  228 - #224866
     296  228 - #27597f
     300  228 - #2b6792
     304  228 - #2d70a0
     324  228 - #995625
     424  228 - #597c95
     288  232 - #214865
     292  232 - #214865
     296  232 - #234f70
     300  232 - #275d84
     236  236 - #707274
     252  236 - #707274
     272  240 - #995625
     188  248 - #985524
     224  248 - #e07e33
     324  248 - #995625
     352  248 - #995625
     224  256 - #e07e33
     268  260 - #995625
     220  264 - #e07e32
     200  268 - #985524
     244  268 - #707274
     224  272 - #e07e32
     336  272 - #995625
     180  280 - #985524
     224  280 - #e07e32
      80  284 - #a1a4a6
     224  288 - #e07e32
     200  292 - #985524
     224  296 - #e07e32
     252  296 - #707274
     280  296 - #707274
     592  296 - #a2a5a7
     180  300 - #985524
     312  300 - #707274
     328  300 - #707274
     348  300 - #995625
     224  308 - #e07e32
     192  316 - #985524
     224  316 - #e07e32
     472  320 - #a1a4a6
     224  324 - #e07e32
     224  332 - #e07e32
     204  336 - #985524
     224  340 - #e07e32
     184  348 - #985524
     224  348 - #e07e32
     204  356 - #985524
     224  356 - #e07e32
     224  364 - #e07e32
     192  376 - #985524
     224  376 - #e07e32
       4  380 - #707274
     400  380 - #a1a4a6
      28  384 - #707274
      72  384 - #707274
     224  384 - #e07e32
     312  384 - #a1a4a6
     128  388 - #707274
     208  388 - #985524
      96  392 - #707274
     144  392 - #707274
     168  392 - #707274
     228  392 - #e07e32
      48  396 - #707274
      80  400 - #707274
     120  400 - #707274
     136  404 - #707274
     156  404 - #707274
     224  404 - #e07e32
     188  408 - #985524
     592  420 - #a1a4a6
     472  444 - #a1a4a6
     344  460 - #a1a4a6
       4  484 - #a0a4a6
     252  500 - #a0a3a6
      96  508 - #a0a3a6
     544  508 - #a1a4a6
     396  524 - #a0a4a6
     468  568 - #a0a4a6
     184  576 - #a0a3a6
      20  592 - #a0a3a6
     104  592 - #a0a3a6
     264  592 - #a0a3a6
     348  592 - #a0a3a6
     592  592 - #a1a4a6
";
