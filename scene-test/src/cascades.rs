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
     268    4 - #597c95
     372    4 - #597c95
     592    4 - #597c95
     484    8 - #597c95
     188   48 - #597c95
      92   52 - #597c95
     536   64 - #597c95
     412   72 - #597c95
     296   92 - #597c95
      20  116 - #597c95
     480  116 - #597c95
     592  116 - #597c95
     216  144 - #597c95
     384  148 - #597c95
     124  164 - #597c95
     444  188 - #597c95
     288  212 - #214865
     292  212 - #26597e
     296  212 - #2e6e9c
     300  212 - #3a7fb3
     184  216 - #597c95
     292  216 - #245072
     296  216 - #2d6893
     292  220 - #214865
     296  220 - #285e85
     304  220 - #327cb1
     292  224 - #214865
     300  224 - #2a6591
       4  228 - #a1a4a7
      80  228 - #a1a4a7
     248  228 - #a65d28
     268  228 - #a1a4a7
     272  228 - #a1a4a7
     296  228 - #214865
     312  228 - #985625
     504  228 - #a2a5a7
     592  228 - #a2a5a7
     268  232 - #a1a4a6
     272  232 - #a1a4a6
     272  236 - #707274
     292  236 - #707274
     332  236 - #985625
     268  240 - #707274
     272  240 - #707274
     312  240 - #988d84
     268  244 - #a1a4a6
     272  244 - #a1a4a6
     312  244 - #988d84
     248  248 - #985524
     272  248 - #a1a4a6
     376  248 - #7e471f
     268  252 - #a1a4a6
     376  260 - #7e471f
     396  260 - #985626
     420  260 - #985626
     244  268 - #707274
     296  268 - #707274
     332  268 - #985625
     376  272 - #7e471f
     264  276 - #985524
     376  284 - #7e471f
     392  284 - #985625
     416  284 - #985626
     248  288 - #985524
     108  296 - #707274
     144  296 - #707274
     180  296 - #707274
     376  296 - #7e471f
     224  300 - #707274
     260  300 - #985524
     376  308 - #7e471f
     396  308 - #985625
     420  308 - #9d7d66
     528  312 - #a1a4a6
       4  316 - #a1a4a6
     372  316 - #988d84
     372  320 - #988d84
     372  328 - #907663
     372  332 - #907663
     404  332 - #985625
     372  340 - #875e41
     372  344 - #875e41
     396  352 - #985625
     416  360 - #985625
     376  364 - #7e471f
     112  376 - #707274
     372  376 - #7e471f
      60  384 - #707274
     216  384 - #707274
     400  384 - #985525
     300  388 - #707274
     372  388 - #7e471f
     160  392 - #707274
     244  392 - #707274
     272  400 - #707274
     592  400 - #a1a4a6
     324  404 - #707274
     388  408 - #985524
     412  408 - #985525
     504  428 - #a0a3a6
     208  468 - #a0a3a5
       4  476 - #a0a3a5
     104  480 - #a0a3a5
     352  488 - #95352d
     576  496 - #a0a3a6
     344  500 - #7f2e27
     352  508 - #7f2e27
     344  516 - #7f2e27
     480  520 - #a0a3a5
     352  532 - #7f2e27
     340  544 - #928484
     352  544 - #7f2e27
       4  548 - #a0a3a5
     180  548 - #707274
     340  548 - #928484
     220  552 - #707274
     340  552 - #928484
     260  556 - #707274
     340  556 - #928484
     352  556 - #7f2e27
     340  560 - #856564
     300  564 - #707274
     352  568 - #7f2e27
     516  588 - #a0a3a5
      60  592 - #a0a3a5
     436  592 - #a0a3a5
     592  592 - #a0a3a6
";

const SEAM_CHECK: &str = r"
       4    4 - #597c95
     268    4 - #597c95
     368    4 - #597c95
     592    4 - #597c95
     100    8 - #597c95
     480   20 - #597c95
     184   56 - #597c95
       4   80 - #597c95
     388   80 - #597c95
     548   88 - #597c95
     296   96 - #597c95
      72  108 - #597c95
     448  128 - #597c95
     232  148 - #597c95
     364  152 - #597c95
       4  156 - #597c95
     592  172 - #597c95
     156  184 - #597c95
     504  196 - #597c95
     288  212 - #214865
     292  212 - #275a80
     296  212 - #2f6e9c
     300  212 - #3b7fb3
     284  216 - #214865
     288  216 - #214865
     292  216 - #245174
     296  216 - #2e6893
     292  220 - #214865
     296  220 - #295f86
     304  220 - #327cb1
     292  224 - #214865
     296  224 - #255275
     300  224 - #2a6691
      48  228 - #a1a4a7
     296  228 - #214865
     312  228 - #985625
     260  232 - #cf752f
     264  232 - #a1a4a6
     268  232 - #a1a4a6
     272  232 - #a1a4a6
     260  236 - #bd6a2c
     264  236 - #a1a4a6
     268  236 - #a1a4a6
     272  236 - #a1a4a6
     300  236 - #a1a4a6
     336  236 - #985625
     260  240 - #bd6a2c
     260  244 - #bd6a2c
     272  244 - #a1a4a6
     260  248 - #bd6a2c
     264  248 - #a1a4a6
     268  248 - #a1a4a6
     264  252 - #a1a4a6
     268  252 - #a1a4a6
     272  252 - #a1a4a6
     208  256 - #707274
     424  256 - #b3652a
     448  256 - #b3652b
     532  260 - #a1a4a7
     336  264 - #9d7d66
     400  264 - #7e471f
     336  268 - #9d7d66
     300  272 - #707274
     336  272 - #9d7d66
     464  276 - #9d7d66
     240  280 - #9d7d65
     240  284 - #9d7d65
     432  284 - #985626
     240  288 - #9d7d65
     396  288 - #988d84
     396  296 - #907663
     396  300 - #907663
     588  300 - #a1a4a6
      88  304 - #707274
     128  304 - #707274
     396  304 - #875e41
     184  308 - #707274
     216  308 - #707274
     260  308 - #985524
     440  312 - #985626
       4  320 - #a1a4a6
     396  332 - #7e471f
     424  344 - #985625
     460  352 - #9d7d66
     300  364 - #a0a3a6
     396  364 - #7e471f
     540  364 - #a1a4a6
     456  368 - #985625
     424  384 - #985625
     400  392 - #7e471f
     456  396 - #985625
     416  412 - #985525
      48  416 - #707274
     392  416 - #988c84
     440  416 - #985525
     392  420 - #988c84
       4  424 - #707274
     132  424 - #707274
     392  424 - #8f7563
     592  424 - #a1a4a6
     200  428 - #707274
     392  428 - #8f7563
     232  432 - #707274
     392  432 - #875e41
      88  436 - #707274
     264  436 - #707274
     392  436 - #875e41
     520  436 - #a0a3a6
     312  440 - #707274
     360  444 - #707274
     432  444 - #985524
     172  448 - #707274
     284  460 - #707274
     340  464 - #707274
     376  468 - #707274
     412  468 - #985524
     452  468 - #985524
      28  508 - #a0a3a5
     544  508 - #a0a3a6
     216  512 - #a0a3a5
     416  540 - #a0a3a5
     108  544 - #a0a3a5
     284  548 - #a0a3a5
     476  584 - #a0a3a5
       4  592 - #a0a3a5
     208  592 - #a0a3a5
     360  592 - #a0a3a5
     592  592 - #a0a3a6
";

const WALKED: &str = r"
       4    4 - #597c95
     224    4 - #597c95
     308    4 - #597c95
     460    4 - #597c95
     548    4 - #597c95
     116   12 - #597c95
     504   76 - #597c95
     344   80 - #597c95
     592   80 - #597c95
     424   88 - #597c95
      36  100 - #597c95
     244  112 - #597c95
     136  120 - #597c95
     468  156 - #597c95
     556  156 - #597c95
     380  160 - #597c95
      16  200 - #597c95
     108  208 - #597c95
     288  208 - #26597f
     296  208 - #2f74a5
     284  212 - #214865
     288  212 - #25557a
     292  212 - #2b6590
     296  212 - #3273a2
     280  216 - #214865
     284  216 - #214865
     288  216 - #245071
     292  216 - #296089
     304  216 - #4186ba
     288  220 - #214866
     292  220 - #275a80
     296  220 - #2f6995
     304  220 - #3c80b4
     288  224 - #214865
     292  224 - #255275
     296  224 - #2a628b
     300  224 - #2f6e9d
     304  224 - #3278aa
     428  224 - #597c95
     288  228 - #214865
     292  228 - #224867
     296  228 - #275a80
     300  228 - #2b6792
     304  228 - #2d70a0
     344  228 - #a65e28
     288  232 - #214865
     292  232 - #214865
     296  232 - #244f71
     300  232 - #285d85
     324  232 - #985625
     592  232 - #a2a5a7
     236  236 - #707274
     252  236 - #707274
     292  236 - #355369
     352  244 - #985625
     204  248 - #985524
     224  248 - #e07f33
     272  248 - #985525
     324  252 - #985625
     220  256 - #e07e32
     224  260 - #e07f33
     180  264 - #985524
     352  264 - #985625
     220  268 - #e07e32
     244  268 - #707274
     268  268 - #985524
     224  272 - #e07e32
     200  276 - #985524
     220  276 - #ce742f
     336  276 - #985625
     224  280 - #e07e32
     220  284 - #bc6a2b
     220  288 - #bc6a2b
     480  288 - #a1a4a6
     180  292 - #985524
      12  296 - #a1a4a6
     204  296 - #985524
     224  296 - #e07e32
     252  296 - #707274
     280  296 - #707274
     352  296 - #9d7d66
     312  300 - #707274
     332  300 - #707274
      96  308 - #a1a4a6
     224  308 - #e07e32
     196  312 - #985524
     224  316 - #e07e32
     224  328 - #e07e32
     180  336 - #9c7c65
     224  340 - #e07e32
     592  340 - #a1a4a6
     200  348 - #985524
     224  352 - #e07e32
     224  364 - #e07e32
     420  364 - #a0a3a6
     192  372 - #985524
     228  372 - #c0916c
     228  376 - #c0916c
      20  380 - #707274
     224  380 - #e07e32
      56  384 - #707274
     312  384 - #a0a3a6
     116  388 - #707274
     132  388 - #707274
     204  388 - #985524
     224  388 - #e07e32
       4  392 - #707274
      80  392 - #707274
     180  392 - #707274
      32  396 - #707274
     164  396 - #707274
     224  396 - #e07e32
     104  400 - #707274
     144  404 - #707274
     224  404 - #e07e32
     192  408 - #985524
     512  412 - #a0a3a6
     368  444 - #a0a3a5
     592  484 - #a0a3a6
       4  496 - #a0a3a5
     276  496 - #a0a3a5
     104  508 - #a0a3a5
     476  536 - #a0a3a5
     196  564 - #a0a3a5
      36  592 - #a0a3a5
     276  592 - #a0a3a5
     360  592 - #a0a3a5
     592  592 - #a0a3a6
";
