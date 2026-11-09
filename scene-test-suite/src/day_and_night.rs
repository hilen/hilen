use std::f32::consts::{FRAC_PI_4, PI, TAU};

use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Shape3, Vec3},
    refs::Weak,
    scene::{Camera, DayNightSky, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, scene},
    ui::Color,
    ui_test::{check_colors, set_record_probe_count},
};

/// Scene seconds of one whole day while the clock runs.
const DAY_SECONDS: f32 = 48.0;
/// How far from the equator the sun's path is drawn.
const LATITUDE: f32 = FRAC_PI_4;

/// Noon, the afternoon, the sun on the horizon, the dusk and the night
/// before dawn, as parts of a day from midnight.
const NOON: f32 = 0.5;
const AFTERNOON: f32 = 0.68;
const SUNSET: f32 = 0.74;
const DUSK: f32 = 0.76;
const NIGHT: f32 = 0.2;

/// A field with posts, a hut and a ball under a `DayNightSky`, looking
/// west where the sun goes down. In presentation the clock runs a day
/// in 48 seconds. The test stops it and sets the hour: a blue sky and a
/// bright field at noon, the low sun of the afternoon, then the sun disc
/// on the horizon in an orange glow that tints the field, the dusk with
/// the first stars, and the night with stars and the moon over a field
/// in pale blue moonlight.
#[scene]
#[derive(Default)]
struct DayAndNight {
    /// The part of the day from midnight, 0 to 1.
    time:    f32,
    stopped: bool,
}

impl SceneSetup for DayAndNight {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(8.0, 3.0, 2.0),
            target: Vec3::new(-10.0, 4.5, -0.5),
            ..Camera::default()
        };

        self.make_node::<Prop>(Shape3::Plane(400.0), Vec3::ZERO)
            .set_color(Color::hex("#6f8f5a"))
            .set_roughness(0.9);

        for (x, z) in [
            (-2.0, -3.0),
            (-6.0, -2.0),
            (-10.0, 2.5),
            (-4.0, 3.5),
            (-14.0, -4.0),
        ] {
            self.make_node::<Prop>(Shape3::cuboid(0.5, 2.5, 0.5), Vec3::new(x, 1.25, z))
                .set_color(Color::hex("#c0703a"))
                .set_roughness(0.7);
        }
        self.make_node::<Prop>(Shape3::cuboid(3.0, 2.0, 3.0), Vec3::new(-8.0, 1.0, -6.0))
            .set_color(Color::hex("#e2d8c4"))
            .set_roughness(0.8);
        self.make_node::<Prop>(Shape3::Ball(1.0), Vec3::new(-5.0, 1.0, 1.0))
            .set_color(Color::hex("#2f6fb7"))
            .set_roughness(0.4);

        self.set_time(NOON);
    }

    fn update(&mut self, dt: f32) {
        if !self.stopped {
            self.set_time((self.time + dt / DAY_SECONDS).fract());
        }
    }
}

impl DayAndNight {
    /// Moves the sky, the sun and the ambient light to this hour. By day
    /// the one sun is the sun, by night the moon opposite it.
    fn set_time(&mut self, time: f32) {
        self.time = time;
        let (sun, height) = sun_path(time);
        let night = night_factor(sun.y);
        let sunset = (1.0 - height * 2.5).clamp(0.0, 1.0);

        self.day_night_sky = Some(DayNightSky {
            sun,
            moon: -sun,
            night,
            sunset,
            twinkle: time * 50.0,
            star_seed: 42.0,
        });

        if sun.y > 0.0 {
            let warm = (1.0 - height * 4.0).max(0.0);
            self.sun.direction = -sun;
            self.sun.color = Color::rgb(1.0, 1.0 - warm * 0.25, 1.0 - warm * 0.45);
            self.sun.intensity = 0.8 * height.min(1.0);
        } else {
            self.sun.direction = sun;
            self.sun.color = Color::rgb(0.7, 0.75, 0.9);
            self.sun.intensity = 0.25 * night;
        }

        let day = 1.0 - night;
        self.ambient = Color::rgb(0.08 + 0.37 * day, 0.09 + 0.36 * day, 0.14 + 0.33 * day);
    }
}

/// The direction to the sun at this part of the day, rising in the east,
/// +x, highest in the south, -z, at noon and setting in the west, and its
/// height from 0 on the horizon to 1 at noon.
fn sun_path(time: f32) -> (Vec3, f32) {
    let hour = (time - 0.5) * TAU;
    let sin_height = LATITUDE.cos() * hour.cos();
    let cos_height = sin_height.asin().cos().max(0.001);
    let turn = (-hour.sin() / cos_height).clamp(-1.0, 1.0).asin();
    let turn = if hour.cos() < 0.0 { PI - turn } else { turn };
    let direction = Vec3::new(turn.sin() * cos_height, sin_height, -turn.cos() * cos_height).normalize();
    (direction, sin_height.max(0.0) / LATITUDE.cos())
}

/// 0 while the sun is up, 1 once it is well below the horizon, blended
/// while it crosses.
fn night_factor(sun_height: f32) -> f32 {
    (1.0 - (sun_height + 0.1) / 0.2).clamp(0.0, 1.0)
}

impl SceneTest for DayAndNight {
    fn perform_test(mut scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);

        for (time, colors) in [
            (NOON, NOON_COLORS),
            (AFTERNOON, AFTERNOON_COLORS),
            (SUNSET, SUNSET_COLORS),
            (DUSK, DUSK_COLORS),
            (NIGHT, NIGHT_COLORS),
        ] {
            from_main(move || {
                scene.stopped = true;
                scene.set_time(time);
            });
            wait_for_next_frame();
            check_colors(colors)?;
        }
        Ok(())
    }
}

const NOON_COLORS: &str = r"
       4    4 - #abc8ec
     160    4 - #a8c6ec
     440    4 - #a8c6ec
     592    4 - #abc7ec
     300   64 - #acc8ec
     112   80 - #afcaec
     504   80 - #afcaec
      28  112 - #b3cdec
     588  128 - #b4ceec
     220  132 - #b2cdec
     372  140 - #b3cdec
     124  168 - #b7cfec
     472  172 - #b7d0ec
       4  220 - #bcd4ed
     276  224 - #bbd3ed
     388  244 - #bdd4ed
     592  256 - #bfd6ed
     192  264 - #bfd6ed
     100  272 - #c0d6ed
     488  272 - #c0d6ed
       4  352 - #688259
     592  352 - #6c855e
     368  356 - #583421
     208  360 - #583421
     148  368 - #583421
     424  372 - #c9c0b1
     428  372 - #c9c0b1
     432  372 - #c9c0b1
     436  372 - #c9c1b1
     440  372 - #c9c1b1
     444  372 - #c9c1b1
     448  372 - #c9c1b1
     452  372 - #c9c1b1
     456  372 - #c9c1b1
     496  372 - #c9c1b2
     500  372 - #c9c1b2
     504  372 - #c9c1b2
     384  376 - #583421
     452  376 - #c9c1b1
     456  376 - #c9c1b1
     496  376 - #c9c1b2
     500  376 - #c9c1b2
     168  380 - #583421
     272  384 - #3267a8
     252  388 - #275085
     264  388 - #2d5c98
     280  388 - #366bad
     148  392 - #583421
     208  392 - #583421
     260  392 - #2a558c
     276  392 - #3766a4
     240  396 - #1d365b
     256  396 - #264d7f
     268  396 - #2e5a93
     284  396 - #587cb4
     288  396 - #527ab6
     292  396 - #4273b5
     248  400 - #203e68
     264  400 - #295287
     280  400 - #5576aa
      72  404 - #678258
     168  404 - #583421
     272  404 - #2f588e
     292  404 - #7692c2
     296  404 - #4874b2
     376  404 - #583421
     476  404 - #583421
     244  408 - #1c3457
     260  408 - #234472
     280  408 - #406499
     288  408 - #738bb9
     232  412 - #1c3457
     276  416 - #294f81
     292  416 - #38629b
     304  416 - #3367a8
     204  420 - #583421
     256  420 - #1c3457
     284  420 - #2c5388
     148  424 - #583421
     300  424 - #2e5d99
     228  428 - #1c3457
     244  428 - #1c3457
     288  428 - #274e81
     264  432 - #1c3457
     276  432 - #1f3a62
     296  432 - #285287
     364  432 - #583421
     304  436 - #2a558c
     472  436 - #583421
     232  444 - #1c3457
     252  444 - #1c3457
     276  444 - #1c3457
     292  444 - #203f69
     168  448 - #aa673e
     592  448 - #6a835a
     168  452 - #aa673e
     264  452 - #1c3457
     384  452 - #583421
     144  456 - #583421
     168  456 - #aa673e
     244  456 - #1c3457
     168  460 - #aa673e
     284  460 - #1c3457
     496  460 - #583421
     168  464 - #aa673e
     256  464 - #1c3457
     272  464 - #1c3457
     168  468 - #aa673e
      20  472 - #678157
     168  472 - #aa673e
     464  472 - #583421
     168  476 - #aa673e
     144  480 - #583421
     488  496 - #583421
     352  504 - #678258
     564  520 - #688259
      80  524 - #678157
     236  524 - #678157
     408  528 - #678258
     300  540 - #678157
     176  548 - #678157
     528  576 - #688258
       4  592 - #668157
     120  592 - #668157
     232  592 - #678157
     364  592 - #678157
     464  592 - #678258
     592  592 - #688258
";

const AFTERNOON_COLORS: &str = r"
       4    4 - #abc8ec
     348    4 - #a8c5ec
     592    4 - #abc7ec
     220    8 - #a8c5ec
     472   12 - #aac7ec
     112   32 - #abc8ec
     248  108 - #b0cbec
     556  112 - #b3cdec
     408  116 - #ccdef2
      60  132 - #b4ceec
     356  132 - #cddff2
     444  140 - #cedff2
     336  164 - #cfe0f2
     388  168 - #f5f6ed
     400  168 - #f5f6ec
     432  168 - #dae8f4
     384  172 - #f6f6ec
     380  176 - #f1f5f1
     388  176 - #f8f3df
     404  176 - #f7f5e8
     392  180 - #f8f2dc
     464  180 - #cddef1
     384  184 - #f6f5e9
     396  184 - #f8f2df
     184  188 - #b8d0ec
     392  188 - #f7f4e5
     404  188 - #f1f5f1
     388  192 - #f1f5f1
     396  192 - #f3f5ef
     328  200 - #cddff1
     360  212 - #d9e7f3
     440  212 - #d4e4f2
     592  220 - #bcd4ed
     396  228 - #d9e7f3
     360  248 - #cddff0
     428  248 - #cddff0
       4  256 - #bfd6ed
     388  256 - #cddff0
     108  260 - #bfd6ed
      60  352 - #4e5c47
     252  352 - #606c5b
     312  352 - #6b7567
     380  352 - #757e71
     384  352 - #757e71
     388  352 - #757e71
     392  352 - #757f71
     396  352 - #757f71
     400  352 - #757f71
     404  352 - #757e71
     408  352 - #757e71
     412  352 - #757e71
     416  352 - #757e71
     528  352 - #657060
     560  352 - #606c5b
     364  356 - #583421
     368  356 - #583421
     212  360 - #583421
     284  360 - #646f5f
     148  368 - #583421
     336  368 - #6a7465
     424  372 - #aaa6a1
     428  372 - #a9a5a0
     432  372 - #a9a59f
     436  372 - #a8a49f
     440  372 - #a7a39e
     444  372 - #a6a29d
     448  372 - #a6a29c
     452  372 - #a5a19b
     456  372 - #a4a09a
     496  372 - #9b9791
     500  372 - #9a9690
     504  372 - #9a958f
     452  376 - #a49f9a
     456  376 - #a39f99
     496  376 - #9a9690
     500  376 - #9a958f
     268  384 - #36496a
     364  384 - #583421
     208  392 - #583421
     244  396 - #1c3457
     544  396 - #67635e
     340  400 - #616d5c
     168  404 - #583421
     264  404 - #1c3457
     292  404 - #1c3457
     388  404 - #636e5e
     472  408 - #583421
     248  416 - #1c3457
     428  416 - #67635e
       4  420 - #475740
     228  420 - #1c3457
     288  420 - #1c3457
     364  420 - #583421
     148  424 - #583421
     268  424 - #1c3457
     504  424 - #67635e
      84  428 - #4a5942
     304  432 - #1c3457
     284  436 - #1c3457
     244  440 - #1c3457
     548  440 - #67635e
     264  444 - #1c3457
     460  444 - #67635e
     168  448 - #734b38
     168  452 - #734b38
     368  452 - #583421
     168  456 - #724b37
     284  456 - #1c3457
     168  460 - #724b37
     496  460 - #583421
     168  464 - #724a37
     256  464 - #1c3457
     168  468 - #724a36
     168  472 - #724a36
     168  476 - #724a36
     144  480 - #583421
     464  480 - #583421
     496  496 - #583421
      52  508 - #46563e
     592  508 - #4d5b45
     396  520 - #4f5e49
     320  552 - #4b5a44
     504  584 - #4a5943
       4  592 - #44543b
     116  592 - #45553d
     228  592 - #47573f
     412  592 - #4a5943
     592  592 - #485841
";

const SUNSET_COLORS: &str = r"
       4    4 - #96b0d1
     120    4 - #94aed0
     248    4 - #93add0
     360    4 - #93aed0
     452    4 - #94aed0
     592    8 - #96b0d1
     544   64 - #9ab2d1
     116   72 - #99b2d1
     280   80 - #99b1d1
     384   80 - #99b2d1
     476   92 - #9bb3d1
     184   96 - #9ab3d1
      60  112 - #a0b5d0
     588  124 - #a4b6cf
     332  144 - #a3b5ce
     128  148 - #a7b6cd
     416  152 - #a7b6cd
     248  160 - #a9b6cc
     520  184 - #b0b8c9
     184  216 - #bcbbc4
     296  216 - #bcbbc5
       4  220 - #bbbac3
     112  224 - #bcb9c2
     360  228 - #bcbac2
     444  232 - #bcbac2
     500  236 - #bcbac3
     592  240 - #bbbbc4
      56  252 - #c2babe
     256  264 - #dfd3cc
     280  268 - #ded1ca
     224  276 - #e5d6cb
       4  284 - #c8bbbb
     124  288 - #ccbcb8
     248  288 - #eddccd
     272  288 - #ebdacc
     300  288 - #e2d1c7
     516  288 - #c6bcbc
     196  296 - #e4d1c5
     228  296 - #eddbcb
     400  300 - #cdbbb7
     244  312 - #f6ebd6
     248  312 - #f6eed8
     252  312 - #f6eed9
     256  312 - #f6ecd7
      56  316 - #cfbcb5
     216  316 - #eed9c6
     248  316 - #f7f0da
     252  316 - #f7f0db
     256  316 - #f7efd9
     260  316 - #f6ebd6
     244  320 - #f7eed9
     248  320 - #f7f0db
     252  320 - #f7f0db
     256  320 - #f7efda
     260  320 - #f6ecd6
     292  320 - #edd7c4
     324  320 - #e1cabb
     592  320 - #c9bdbc
     188  324 - #e7cfbe
     248  324 - #f7efda
     252  324 - #f7efda
     256  324 - #f6eed8
     248  328 - #f6ecd6
     252  328 - #f6ecd6
     464  328 - #d0bcb4
     108  340 - #d6bcaf
     164  340 - #dfc4b4
     332  344 - #dec4b3
       4  348 - #cdb9b0
     184  348 - #e1c7b5
     208  348 - #ead1bc
     232  348 - #eed6bf
     272  348 - #eed6bf
     300  352 - #333929
     444  372 - #524e4c
     416  376 - #4f4c4b
     512  376 - #514d4b
     540  376 - #504d4b
     280  388 - #142746
     432  388 - #4f4c4b
      96  392 - #2a3425
     260  392 - #142746
     240  396 - #142746
     452  396 - #4f4c4b
     496  396 - #4f4c4b
     296  400 - #142746
     548  400 - #4f4c4b
     524  404 - #4f4c4b
     268  408 - #142746
     420  408 - #4f4c4b
     284  412 - #142746
       4  416 - #293324
     228  416 - #142746
     244  416 - #142746
     160  420 - #432719
     304  420 - #142746
     460  420 - #4f4c4b
     500  420 - #4f4c4b
     264  424 - #142746
     520  424 - #4f4c4b
     548  424 - #4f4c4b
     228  432 - #142746
     288  432 - #142746
     304  436 - #142746
     440  436 - #4f4c4b
     248  440 - #142746
     272  444 - #142746
     460  444 - #4f4c4b
     504  444 - #4f4c4b
     532  444 - #4f4c4b
     292  452 - #142746
     256  464 - #142746
     276  464 - #142746
     112  476 - #293324
     372  500 - #283324
      24  504 - #283324
     592  504 - #273223
     440  508 - #283324
     188  512 - #293324
     312  520 - #293324
     492  552 - #273223
      84  564 - #283324
     220  564 - #283324
     280  576 - #283324
       4  592 - #273223
     164  592 - #283324
     392  592 - #283324
     592  592 - #273223
";

const DUSK_COLORS: &str = r"
     148    4 - #5f7189
     388    4 - #5f7189
     504    4 - #607189
     592    4 - #617289
     268   12 - #5f7189
       4   32 - #8390a3
      80   44 - #627389
     164   80 - #637489
     564   80 - #647489
     484   88 - #647489
     336   96 - #647489
      96  120 - #697689
     228  128 - #a1aaba
       4  152 - #747b89
     592  156 - #717a89
     304  172 - #747a89
     160  180 - #787c88
     396  184 - #777c89
     500  188 - #777c89
     592  236 - #7e8089
     108  248 - #8a8487
     212  248 - #8b8588
       4  252 - #8a8487
     284  252 - #8a8487
     332  260 - #8a8487
     396  264 - #8a8487
     160  268 - #918888
      56  272 - #8e8687
     448  272 - #8a8487
     536  296 - #8a8588
     360  300 - #938886
     108  304 - #988b87
       4  308 - #948886
     188  316 - #a89a91
     200  316 - #a99a91
     412  316 - #948886
     212  320 - #ab9c92
     224  320 - #a99b91
     168  324 - #a99a90
     236  324 - #a99a90
     480  324 - #928887
      56  328 - #9a8b86
     180  328 - #ad9d92
     204  332 - #b1a195
     220  332 - #b0a094
     248  332 - #aa9a8f
     192  336 - #b2a295
     148  340 - #aa998f
     164  340 - #ae9e92
     312  340 - #9e8d87
     592  340 - #8e8788
     228  344 - #b2a194
     176  348 - #af9f93
     208  348 - #b3a395
     248  348 - #aa9a8f
     384  364 - #462c21
     216  372 - #462c21
     420  372 - #2b2b2f
     480  372 - #462b20
     152  376 - #462c21
     456  380 - #53545c
     512  380 - #53545c
     532  380 - #53545c
     496  388 - #53545c
     268  392 - #0b2147
     544  392 - #53545c
     208  396 - #462c21
     256  396 - #0b2147
     368  396 - #462c21
     528  396 - #53545c
     292  400 - #0b2147
     456  400 - #53545c
     512  400 - #53545c
     272  404 - #0d2550
       4  408 - #131b15
     496  408 - #53545c
     540  408 - #53545c
     164  412 - #462c21
     240  412 - #0b2147
     256  412 - #0e2650
     284  412 - #0e2651
     524  412 - #53545c
      84  424 - #131b15
     248  424 - #0d254f
     268  424 - #2f3e65
     272  424 - #2d3d64
     300  424 - #0c234b
     456  424 - #53545c
     548  424 - #53545c
     204  428 - #462c21
     268  428 - #2f3f66
     272  428 - #2d3d65
     516  428 - #53545c
     236  432 - #0b2146
     288  432 - #0e2752
     384  432 - #462c21
     500  436 - #53545c
     252  440 - #0d2650
     240  444 - #0b2147
     264  444 - #0e2752
     460  444 - #53545c
     528  444 - #53545c
     164  448 - #462c21
     276  448 - #0e2651
     296  448 - #0b2147
     372  452 - #462c21
     260  460 - #0b2147
     476  460 - #462b20
     496  472 - #462b20
     144  480 - #462c20
     476  484 - #462b20
     348  492 - #131b14
     496  496 - #462b1f
      44  500 - #131b14
     592  504 - #131b14
     212  516 - #131b14
     412  516 - #131b14
     100  520 - #131b14
     156  540 - #131b14
     300  544 - #131b14
     544  548 - #131b14
      56  560 - #131b14
     484  572 - #131b14
     112  584 - #131b14
       4  592 - #131b14
     224  592 - #121b14
     376  592 - #121b14
     592  592 - #131b14
";

const NIGHT_COLORS: &str = r"
     136    4 - #0a0a19
     392    4 - #0d0e1c
     592    4 - #0a0a19
     492   12 - #0b0b1a
       4   32 - #73747b
     300   72 - #1f202c
     220   92 - #4e4f56
     540  100 - #0e0e1c
     376  104 - #323540
     308  112 - #353742
      96  120 - #0b0b1a
     228  128 - #b0b2ba
     340  136 - #484c58
     436  140 - #333540
     296  148 - #464956
     388  152 - #4c4f5c
     272  172 - #474a56
     324  172 - #5b606d
     360  172 - #5d616f
     420  172 - #474a57
     236  184 - #353742
     292  196 - #5a5e6b
     372  196 - #646977
     464  196 - #343641
     592  196 - #0d0d1c
     316  200 - #656978
     400  200 - #5a5e6c
     344  204 - #7f8491
     348  204 - #7c808e
       4  208 - #0c0c1b
     340  208 - #afb3bf
     344  208 - #b5b9c6
     348  208 - #b3b7c3
     340  216 - #dee3ee
     348  216 - #dfe4ef
     260  220 - #4a4d5a
     332  220 - #d6dbe7
     352  220 - #e1e5f1
     360  220 - #c7cbd7
     296  224 - #606573
     324  224 - #9ca1ad
     340  224 - #e5e9f4
     344  224 - #e5eaf4
     348  224 - #e5e9f4
     356  224 - #dee3ee
     364  224 - #adb1bd
     368  224 - #6f7382
     324  228 - #9fa3b0
     328  228 - #c8ccd8
     340  228 - #e5e9f4
     344  228 - #e6eaf5
     348  228 - #e5eaf4
     364  228 - #aeb2bf
     368  228 - #717584
     396  228 - #5f6472
     436  228 - #474b57
     324  232 - #979ca8
     328  232 - #c2c7d3
     332  232 - #dadeea
     336  232 - #e1e6f1
     344  232 - #e5e9f4
     356  232 - #dde1ed
     336  236 - #dde1ed
     340  236 - #e1e5f1
     344  236 - #e2e7f2
     348  236 - #e1e6f1
     112  240 - #0d0d1c
     228  240 - #343641
     340  240 - #d9dee9
     344  240 - #dbe0ec
     348  240 - #dadfea
     340  244 - #c0c4d0
     344  244 - #c5c9d5
     348  244 - #c1c6d2
     288  248 - #5a5e6c
     340  248 - #9397a4
     344  248 - #9a9eab
     348  248 - #959aa6
     504  248 - #1f202c
     372  252 - #666a79
     400  252 - #5a5e6c
     260  256 - #474a56
     428  264 - #474a56
     332  268 - #636876
     356  268 - #636876
     312  272 - #5d616f
     376  276 - #5b5f6d
     456  276 - #333540
     244  280 - #363844
     344  288 - #5a5e6b
     284  292 - #474a57
     408  292 - #464a56
     316  312 - #474a56
     376  312 - #464956
     348  316 - #474a57
     408  328 - #333541
       4  352 - #1f2729
     100  352 - #252c30
     168  352 - #2b3238
     296  352 - #3f4652
     332  352 - #444b58
     484  352 - #30383f
     364  356 - #0d0605
     368  356 - #0d0605
     232  364 - #313840
     592  364 - #252d30
     428  372 - #484d5b
     268  380 - #333a42
     348  396 - #353d46
      52  400 - #1e2626
     308  412 - #30383f
     388  412 - #30383f
     192  428 - #242c2f
     116  444 - #1e2627
     500  448 - #232b2e
       4  452 - #192221
     564  476 - #1e2627
     268  484 - #232b2e
     352  500 - #232b2e
     452  500 - #21292b
     188  504 - #1e2627
     396  560 - #1e2626
       4  592 - #161f1c
     116  592 - #18201e
     216  592 - #1a2221
     312  592 - #1b2424
     492  592 - #1a2322
     592  592 - #18211f
";
