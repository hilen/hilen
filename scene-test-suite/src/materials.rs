use std::f32::consts::FRAC_PI_6;

use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::{
        LossyConvert,
        volume::{Shape3, Vec3},
    },
    refs::Weak,
    scene::{Camera, Material, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, scene},
    ui::Color,
    ui_test::{capture_screenshot, check_colors, set_record_probe_count},
};

/// Steps of roughness across and of metallic down.
const STEPS: usize = 7;
const SPACING: f32 = 1.1;
/// Frames of each turn between the checks. A turn is 30 degrees, enough
/// to move every highlight and not enough to hide one ball behind another.
const TURN_FRAMES: usize = 15;

/// The material chart every engine has, a wall of balls with roughness
/// from a mirror on the left to matte on the right and metallic from a
/// dielectric on the top row to a pure metal on the bottom one, all of
/// one base color under the default sun. The highlight has to widen and
/// fade rightwards, the diffuse has to drain downwards while the
/// reflection takes the base color, and the two turns, sideways and then
/// up, move every highlight with the eye, which a diffuse only shade
/// never does.
#[scene]
#[derive(Default)]
struct Materials {}

impl SceneSetup for Materials {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 0.0, 7.5),
            target: Vec3::ZERO,
            ..Camera::default()
        };

        for row in 0..STEPS {
            for column in 0..STEPS {
                let step = |i: usize| i.lossy_convert() / (STEPS - 1).lossy_convert();
                let x = (column.lossy_convert() - (STEPS - 1).lossy_convert() / 2.0) * SPACING;
                let y = ((STEPS - 1).lossy_convert() / 2.0 - row.lossy_convert()) * SPACING;
                self.make_node::<Prop>(Shape3::Ball(0.45), Vec3::new(x, y, 0.0))
                    .set_material(Material {
                        color: Color::hex("#3d85c6"),
                        metallic: step(row),
                        roughness: step(column),
                        ..Material::default()
                    });
            }
        }
    }
}

impl SceneTest for Materials {
    fn perform_test(scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);

        wait_for_next_frame();
        check_colors(FRONT)?;

        turn(scene, FRAC_PI_6, 0.0);
        check_colors(SIDE)?;

        turn(scene, 0.0, FRAC_PI_6);
        capture_screenshot()?;
        check_colors(ABOVE)
    }
}

/// Orbits the camera by `yaw` and `pitch` radians over `TURN_FRAMES`
/// frames, a fixed step per frame so every machine lands on the same view.
fn turn(mut scene: Weak<Materials>, yaw: f32, pitch: f32) {
    let frames: f32 = TURN_FRAMES.lossy_convert();
    for _ in 0..TURN_FRAMES {
        from_main(move || scene.camera.orbit(yaw / frames, pitch / frames));
        wait_for_next_frame();
    }
}

const FRONT: &str = r"
       4    4 - #597c95
      80    4 - #597c95
     456    4 - #597c95
     592    4 - #597c95
     132   40 - #3f7eb9
     388   40 - #508cc8
     532   40 - #4581bc
      80   44 - #4285c3
     304   44 - #6292c7
     308   44 - #6092c9
     472   44 - #4c88c4
     224   48 - #6e96c7
     312   48 - #6190c5
     308   52 - #6a92c2
      40   56 - #32608e
     592   72 - #597c95
      96   80 - #2b5278
     176   84 - #284a6c
     352   88 - #284a6c
     472   88 - #284a6c
     228  124 - #69a3e1
     300  124 - #568fc9
     396  124 - #4884c0
     548  124 - #427fb9
      84  128 - #3c79b3
     312  128 - #5d95d1
     132  132 - #366c9f
     308  132 - #5f94ce
     504  132 - #305982
      40  148 - #27496c
     176  152 - #2d5984
     464  160 - #27496c
     352  164 - #27496c
     520  176 - #27496c
     592  176 - #597c95
      48  200 - #346b9e
      88  200 - #3873ab
     304  200 - #5393d3
     228  204 - #89bcfa
     300  204 - #5897d8
     312  204 - #5392d2
     388  204 - #4f8dcb
     308  208 - #6babf0
     148  212 - #376fa4
     228  212 - #508cc8
     548  224 - #336392
     460  232 - #2a5279
      44  240 - #25486b
     516  276 - #356a9d
     228  280 - #589fe6
     304  280 - #5da5ec
     300  284 - #589fe6
     304  284 - #70b1f6
     308  284 - #6caff5
     312  284 - #5297da
     384  284 - #4f91d2
      80  288 - #31679a
     228  288 - #7ab6f9
     304  288 - #5fa6ed
     308  288 - #5fa6ee
     140  292 - #2f6393
     308  292 - #4785c1
     456  304 - #2e5c88
     560  308 - #326290
       4  336 - #597c95
      76  348 - #2d6191
     436  352 - #306393
     528  352 - #346b9f
     228  356 - #4080bc
     304  356 - #4d96db
     220  360 - #3b77b0
     296  360 - #407fba
     384  360 - #4b92d5
     220  364 - #3d7ab4
     308  364 - #68aef6
     312  364 - #4e96db
     228  368 - #529de5
     480  372 - #346a9e
     140  384 - #27537d
     592  396 - #597c95
     532  404 - #21466a
      60  424 - #24517b
     448  428 - #31699c
     304  432 - #4084c4
     228  436 - #62abf5
     220  440 - #3a7ab5
     236  440 - #3570a6
     296  440 - #3d7fbd
     300  440 - #57a5f0
     312  440 - #4996dc
     384  440 - #4893d8
     304  444 - #61abf5
     228  448 - #3b7ab5
     308  448 - #4389ca
     480  460 - #2b5e8c
     144  464 - #224d75
     528  464 - #24517a
      64  484 - #1f456a
       8  500 - #597c95
     592  500 - #597c95
     460  504 - #2e679b
     304  508 - #3677b2
     236  516 - #2d6597
     296  516 - #397dbb
     300  516 - #54a5f2
     308  516 - #65aef7
     380  516 - #428fd5
     236  520 - #2e679a
     296  520 - #387cb9
     304  520 - #70b3f9
     312  520 - #4493da
     316  520 - #306ca1
     384  520 - #4390d6
     164  528 - #1c4569
     228  528 - #2b6293
     304  528 - #377ab6
     308  528 - #3678b3
     100  532 - #1c4569
     532  540 - #23537d
      44  544 - #1c4569
     480  544 - #255682
       4  592 - #597c95
      72  592 - #597c95
     168  592 - #597c95
     268  592 - #597c95
     352  592 - #597c95
     476  592 - #597c95
     592  592 - #597c95
";

const SIDE: &str = r"
       4    4 - #597c95
     144    4 - #597c95
     592    4 - #315982
     388   20 - #4987c4
     356   24 - #4681bb
     448   24 - #366390
     512   24 - #284a6c
      76   32 - #597c95
     300   48 - #7ba1d0
     248   60 - #4384c1
     220   64 - #3f7db8
     548   64 - #4582bd
     584   72 - #437fb9
     164   80 - #3e7bb6
     200   84 - #3c77b0
     456   84 - #4886c3
     140   88 - #4285c4
      28   92 - #597c95
     116   96 - #3c79b2
     544  100 - #325e8a
     380  108 - #4e89c5
     492  116 - #35628f
     300  124 - #68a2e0
     296  128 - #669edb
     300  128 - #77aeec
     304  128 - #6aa2df
     300  132 - #5f96d0
     220  136 - #3b78b0
     188  140 - #3e7ebb
     240  140 - #8fbef7
     120  156 - #3c79b2
     164  160 - #34689a
     556  164 - #417eb8
     592  172 - #597c95
       4  176 - #597c95
     468  176 - #4381bc
     384  188 - #427fb9
     532  192 - #336493
     288  196 - #3c77b0
     300  200 - #5292d1
     428  200 - #336493
     244  204 - #3b78b0
     300  204 - #6dadf1
     300  208 - #6faef2
     216  212 - #356b9f
     300  212 - #5592d0
     132  236 - #316394
      48  248 - #597c95
     452  264 - #3c77b0
     372  268 - #3c77b0
     572  268 - #3c77b0
     236  280 - #4482be
     300  280 - #5aa2e9
     292  284 - #4a8ac9
     296  284 - #61a8f0
     300  284 - #71b1f7
     304  284 - #63a9f1
     308  284 - #4b8dcc
     116  288 - #306495
     300  288 - #65aaf2
     236  292 - #4481bc
     300  292 - #4a8ac8
     160  304 - #28527a
     512  304 - #23476b
       4  316 - #597c95
     424  316 - #23476b
     576  332 - #23476b
     184  348 - #2e6394
     236  352 - #3c79b3
     300  356 - #4a90d2
     292  360 - #4385c3
     300  360 - #67adf5
     292  364 - #4689c9
     296  364 - #5fa9f2
     300  364 - #71b2f8
     304  364 - #61aaf3
     308  364 - #488ccd
     300  368 - #57a3ec
     372  368 - #4b91d4
     456  368 - #3b78b0
     464  368 - #3b78b0
     460  372 - #3b78b0
      60  376 - #597c95
     144  376 - #224a6f
     528  380 - #306393
     592  404 - #306394
     236  424 - #3570a6
     112  428 - #214b72
     304  432 - #3978b2
     300  436 - #57a5f1
       4  440 - #597c95
     296  440 - #5faaf4
     300  440 - #73b4f9
     304  440 - #61abf5
     292  444 - #4084c3
     300  444 - #69aff7
     380  444 - #3978b2
     168  448 - #20496f
     372  452 - #4792d7
     472  452 - #326a9e
     536  456 - #306799
     588  476 - #31699c
     420  488 - #1f456a
     112  496 - #1c4569
     240  496 - #2e689d
     244  504 - #3b81c0
     236  512 - #2f699e
     240  512 - #2c6597
     300  512 - #4799e3
     520  512 - #1f456a
      28  516 - #597c95
     300  516 - #6cb1f9
     308  516 - #3d84c5
     296  520 - #5eabf6
     300  520 - #76b5fa
     304  520 - #61acf6
     172  524 - #1c4569
     300  524 - #59a8f4
     372  536 - #4391d8
     448  540 - #2c6395
     560  552 - #2c6496
      84  576 - #597c95
     512  588 - #1c4569
       4  592 - #597c95
     164  592 - #597c95
     256  592 - #597c95
     328  592 - #597c95
     592  592 - #295e8e
";

const ABOVE: &str = r"
       4    4 - #597c95
     496    4 - #4683be
     592    4 - #597c95
     120    8 - #597c95
      68   12 - #4080bd
     408   12 - #4c8bc9
     300   20 - #6799d1
     304   20 - #6296cf
     168   24 - #3e7cb7
     220   24 - #c3d6f4
     224   24 - #76a1d4
     296   24 - #6b9ad0
     304   24 - #6d9cd1
     296   28 - #6997cd
     300   28 - #739ed2
     304   28 - #6c99ce
     544   36 - #4581bb
     456   56 - #2a4d70
      80   64 - #284a6c
     356   68 - #284a6c
     156   76 - #284a6c
     276   76 - #284a6c
     508   88 - #2b4f73
     592  104 - #284a6c
     220  120 - #5c97d6
     232  120 - #5893d0
     396  120 - #4280bc
     296  128 - #649fde
     304  128 - #66a1e1
     296  132 - #67a2e1
      88  136 - #284c71
     300  136 - #639ddb
     304  136 - #5d97d4
     152  140 - #316291
     516  156 - #437fba
       4  168 - #597c95
     376  192 - #27496c
     456  192 - #2b5076
     100  200 - #2e5c89
     164  200 - #3770a6
     580  200 - #346391
     228  204 - #7bb5f7
     236  204 - #69aaef
     232  208 - #71b0f3
     300  212 - #5798da
     296  216 - #61a3e8
     304  220 - #64a7ec
     300  224 - #5c9de0
     488  256 - #417eb8
       4  260 - #597c95
     396  264 - #316290
     588  268 - #3f7db7
     152  272 - #597c95
     236  272 - #86bbfa
     236  280 - #5399dd
     300  288 - #5ca4ec
     536  288 - #305d89
     292  292 - #4a8bca
     296  292 - #60a7ef
     308  292 - #4b8ccc
     168  300 - #597c95
     300  300 - #4d8fcf
     360  300 - #3973a9
     592  320 - #315f8b
     424  324 - #30608e
     160  328 - #597c95
     480  328 - #3971a7
      64  336 - #597c95
     236  336 - #59a5ef
     244  336 - #73b3f8
     212  344 - #597c95
     540  344 - #3870a5
     180  348 - #597c95
     300  348 - #4283c0
     372  352 - #23476b
     292  356 - #4486c4
     296  356 - #5da8f1
     304  356 - #5ea8f2
     308  356 - #4487c6
     300  360 - #60aaf3
     264  364 - #597c95
     224  368 - #597c95
     168  376 - #597c95
     368  380 - #4a91d3
     244  384 - #4186c6
     440  384 - #346a9e
     512  388 - #23476b
     216  392 - #597c95
     244  396 - #3978b2
     184  400 - #597c95
     564  400 - #326493
     300  408 - #5eaaf4
       4  412 - #597c95
     200  412 - #295d8c
     296  412 - #5aa7f2
     304  412 - #5ba8f3
     308  412 - #3d80bd
     196  416 - #275b89
     228  416 - #597c95
     268  416 - #597c95
     300  416 - #58a6f1
     240  436 - #295e8e
     364  436 - #4690d4
     424  436 - #264f77
     488  436 - #21466a
     280  440 - #597c95
     248  444 - #2d6597
     544  452 - #346b9e
     300  456 - #58a7f3
     296  460 - #57a7f3
     304  460 - #57a7f3
     300  464 - #5ba9f4
     300  468 - #397dba
     356  484 - #408cd0
     360  488 - #428ed3
     436  488 - #2c608f
     100  492 - #597c95
      12  500 - #597c95
     188  508 - #597c95
     540  508 - #2f6597
     400  532 - #22507a
     464  556 - #1c4569
     520  568 - #2b6293
      88  576 - #597c95
       4  592 - #597c95
     172  592 - #597c95
     296  592 - #597c95
     592  592 - #597c95
";
