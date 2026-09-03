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
     128    4 - #597c95
     592    4 - #597c95
     376   40 - #4c86c0
     536   40 - #4683bf
     304   44 - #6292c7
     308   44 - #6091c9
     164   48 - #4182bf
     224   48 - #6f97c8
     312   48 - #6090c5
     308   52 - #6b93c2
      40   56 - #32618e
     428   56 - #315981
     100   68 - #35699b
     480   72 - #35628f
     592   88 - #597c95
     148  104 - #416381
     372  104 - #416381
     520  104 - #416381
     304  128 - #72a9e7
     312  128 - #5c95d1
     400  128 - #447fba
      40  132 - #31618f
     308  132 - #5f95cf
     100  140 - #356a9d
     192  140 - #406381
     252  140 - #366c9f
     420  148 - #406381
     556  152 - #325e89
     476  156 - #2d547c
     364  176 - #27496c
      80  196 - #3976af
     552  200 - #427fb9
     228  204 - #86bbf9
     300  204 - #5898d8
     304  204 - #6cacf1
     472  204 - #437fb9
     312  208 - #5795d5
       4  212 - #597c95
     228  212 - #518dca
     308  212 - #538fcd
     152  216 - #346a9d
     420  224 - #3f6280
     364  232 - #25486b
     512  232 - #25486b
     592  244 - #597c95
      52  276 - #316698
     164  276 - #336b9e
     228  280 - #569de3
     304  280 - #5ca4eb
     300  284 - #59a0e6
     304  284 - #70b1f6
     308  284 - #6baef5
     312  284 - #5296da
     384  284 - #4f91d2
     508  284 - #2e5d8a
     228  288 - #7db7f9
     304  288 - #60a7ee
     116  292 - #275078
     308  292 - #4786c2
     420  300 - #3e6280
     460  308 - #2c5883
     556  316 - #295179
       4  320 - #597c95
      76  328 - #23476b
     520  348 - #346b9e
     152  352 - #2e6292
     228  356 - #3f7fba
     304  356 - #4d95d9
     296  360 - #407fbb
     304  360 - #6aaff6
     220  364 - #3e7bb5
     304  364 - #6eb1f7
     308  364 - #68aef6
     312  364 - #4e96db
     384  364 - #4a90d2
      40  368 - #26527c
     228  368 - #54a0e8
     476  372 - #356ca0
      96  392 - #21466a
     556  392 - #2b5984
     304  432 - #3f83c2
     228  436 - #5faaf4
     304  436 - #5da9f4
     496  436 - #597c95
     220  440 - #3b7bb6
     236  440 - #346fa5
     296  440 - #3e80be
     300  440 - #58a6f1
     304  440 - #74b4f9
     308  440 - #6aaff8
     384  440 - #4893d8
     304  444 - #62acf5
     192  448 - #3c6180
     228  448 - #3b7cb8
     296  448 - #336ba0
     304  448 - #448ccf
     308  448 - #438acc
     544  448 - #2d6191
     148  456 - #245079
      76  460 - #234f77
     440  460 - #234f77
       4  464 - #597c95
     592  464 - #597c95
     528  496 - #2c6496
     304  512 - #4ea1ed
     168  516 - #1c4569
     300  516 - #55a6f2
     304  516 - #71b3f9
     308  516 - #64adf7
     380  516 - #428fd5
     468  516 - #306ca1
     236  520 - #2e679a
     296  520 - #397dba
     304  520 - #71b3f9
     312  520 - #4493da
     316  520 - #306ca1
     384  520 - #4390d6
     104  524 - #3b617f
     304  524 - #50a3ef
     228  528 - #2c6395
     304  528 - #387bb8
      40  536 - #1c4569
     552  556 - #214f78
       4  592 - #597c95
     164  592 - #597c95
     268  592 - #597c95
     348  592 - #597c95
     428  592 - #597c95
";

const SIDE: &str = r"
      64    4 - #597c95
     144    4 - #597c95
     212    4 - #597c95
     592    4 - #315a82
     356   24 - #4681bb
     392   24 - #4683be
     444   24 - #36628f
     492   24 - #376390
     300   48 - #7ba1d0
     244   60 - #4585c3
     560   64 - #4583bf
     188   72 - #4387c6
       4   76 - #597c95
     164   84 - #3c77b0
     448   84 - #4784c0
     476   88 - #4683bf
     520   96 - #315b85
     132  100 - #3c77b0
     360  104 - #4582bd
     592  104 - #325e8a
     388  108 - #4582bd
     444  116 - #356492
      60  124 - #597c95
     300  124 - #67a1e0
     296  128 - #679fdc
     300  128 - #77aeec
     304  128 - #69a1df
     300  132 - #6096d1
     360  132 - #346492
     232  140 - #5a92ce
     196  144 - #3c7ab4
     168  148 - #3b78b1
     128  160 - #3b78b0
     556  164 - #417eb8
       4  172 - #597c95
     384  188 - #427eb9
     464  188 - #437fb9
     288  196 - #3c77b0
     300  200 - #5191d0
     300  204 - #6cacf1
     176  208 - #3976ae
     228  208 - #3c77b0
     300  208 - #6faef2
     300  212 - #5593d2
     592  212 - #2d567f
     116  224 - #356ca0
     520  224 - #25486b
     352  236 - #25486b
      36  248 - #597c95
     188  260 - #3f6280
     452  264 - #3c77b0
     572  268 - #3c77b0
     292  276 - #3c77b0
     308  276 - #3c77b0
     132  280 - #346ca1
     236  280 - #4381bd
     300  280 - #59a1e8
     292  284 - #4a8bc9
     296  284 - #61a8f0
     300  284 - #71b1f7
     304  284 - #63a9f1
     308  284 - #4b8ccc
     300  288 - #65abf2
     380  288 - #4583be
     236  292 - #4582be
     300  292 - #4b8bc9
     512  300 - #23476b
      72  316 - #597c95
       4  324 - #597c95
     584  328 - #23476b
     456  336 - #3e6280
     136  340 - #2d6191
     236  352 - #3c79b2
     292  356 - #3b78b0
     300  356 - #498fd1
     184  360 - #2f6494
     300  360 - #66adf5
     292  364 - #478aca
     296  364 - #60a9f3
     300  364 - #72b2f8
     304  364 - #61aaf3
     300  368 - #58a4ed
     372  368 - #4b91d4
     456  368 - #3b78b0
     464  368 - #3b78b0
     528  368 - #326799
     460  372 - #3b78b0
     572  372 - #366fa5
      36  400 - #597c95
     236  400 - #3d6180
     172  412 - #25537e
     592  412 - #2d5e8c
     368  416 - #3d6180
     236  424 - #346fa5
     532  424 - #21466a
     112  432 - #20496f
     300  436 - #56a5f0
     292  440 - #4389cb
     296  440 - #5faaf4
     300  440 - #73b3f9
     304  440 - #60abf5
     300  444 - #6aaff7
     308  444 - #4187c7
     380  444 - #3978b2
     372  452 - #4792d7
     436  452 - #2e6395
     236  472 - #3c6180
     484  472 - #2e6495
       4  476 - #597c95
     556  476 - #2e6495
     236  496 - #2e689c
     372  500 - #3c6180
     244  504 - #3a80bf
     160  508 - #1c4569
     236  512 - #306ba0
     300  512 - #4798e1
     296  520 - #60abf6
     308  520 - #3f8acd
     300  524 - #5aa8f4
     372  536 - #4392d8
      76  540 - #597c95
     452  540 - #2c6497
     560  552 - #2c6496
     368  584 - #3b617f
       4  592 - #597c95
     152  592 - #597c95
     252  592 - #597c95
     468  592 - #245480
";

const ABOVE: &str = r"
     104    4 - #3f7db8
     412    4 - #4685c1
     120    8 - #597c95
     488   12 - #4581bc
     300   20 - #6698d0
     220   24 - #c2d6f4
     224   24 - #75a0d3
     296   24 - #6b9ad0
     544   24 - #4785c1
     304   28 - #6c9acf
       4   44 - #597c95
      72   56 - #284a6c
     152   56 - #315e8b
     456   60 - #284a6c
     576   72 - #416381
     388   76 - #2e567e
     232   80 - #284a6c
     488  104 - #284a6c
      84  116 - #3770a5
     188  116 - #366c9f
     296  128 - #649fde
     304  128 - #65a0e0
     136  132 - #30608e
     296  132 - #68a2e1
     296  136 - #5c96d3
     300  136 - #649ddc
     304  136 - #5e97d5
     244  152 - #2e5984
     380  152 - #4882bb
     516  156 - #437fba
     448  168 - #346391
     168  176 - #3975ad
       4  196 - #597c95
     556  196 - #27496c
     232  200 - #6bacf1
     228  204 - #7cb6f7
     236  204 - #68aaef
     232  208 - #73b1f4
     496  208 - #27496c
     116  212 - #264b6f
     300  212 - #5698d9
     368  212 - #3972a8
     304  216 - #61a4e9
     296  220 - #63a6ea
     300  224 - #5d9ee2
     180  228 - #25486b
     436  240 - #336290
     236  268 - #4382be
     588  268 - #3f7db7
     152  272 - #597c95
     236  272 - #83bafa
     528  272 - #284d72
     376  276 - #274d73
     236  280 - #559be0
     300  288 - #5ba4eb
     292  292 - #4a8bcb
     308  292 - #4a8ccb
     168  300 - #597c95
     300  300 - #4d90d1
     592  320 - #315f8b
       4  324 - #597c95
     424  324 - #30608e
     160  328 - #597c95
     484  332 - #376da1
     236  336 - #5ba6f0
     244  336 - #71b2f8
     240  340 - #65acf5
     540  344 - #3870a5
     176  348 - #597c95
     204  348 - #597c95
     300  352 - #5aa5ef
     372  352 - #23476b
     296  356 - #5ea8f2
     304  356 - #5ea8f1
     300  360 - #61aaf3
     220  364 - #597c95
     264  364 - #597c95
     168  376 - #597c95
     368  380 - #4a91d3
     220  384 - #597c95
     244  384 - #4084c4
      56  388 - #597c95
     156  392 - #1c4569
     436  392 - #3770a6
     180  396 - #1c4569
     212  396 - #597c95
     244  396 - #3a79b4
     136  400 - #3b617f
     564  400 - #326493
     140  408 - #1c4569
     300  408 - #5ea9f4
     168  412 - #1c4569
     200  412 - #285b89
     292  412 - #3e80bd
     296  412 - #5ba8f3
     308  412 - #3d7fbc
     196  416 - #285c8b
     224  416 - #1c4569
     268  416 - #597c95
     300  416 - #59a7f2
     188  436 - #1c4569
     240  436 - #2a5f8f
     364  436 - #4690d4
     488  436 - #21466a
     208  440 - #1c4569
     268  440 - #1d466b
     248  444 - #2d6699
     228  452 - #1d466a
     300  456 - #56a6f3
     424  456 - #326ba0
      16  460 - #597c95
     296  460 - #58a7f3
     308  460 - #3676b1
     300  468 - #397ebc
     548  468 - #306393
     356  484 - #408cd0
     360  488 - #428ed3
     476  504 - #1f456a
     108  512 - #597c95
     392  512 - #1c4569
     232  528 - #597c95
     524  552 - #2c6395
     464  556 - #1c4569
       4  592 - #597c95
      88  592 - #597c95
     176  592 - #597c95
     300  592 - #597c95
     592  592 - #597c95
";
