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
       4    4 - #abc7ec
     160    4 - #a8c6ec
     440    4 - #a8c6ec
     592    4 - #abc7ec
     300   64 - #acc8ec
      92   80 - #afcaec
     200  104 - #b0cbec
     584  124 - #b4ceec
       4  132 - #b5ceec
     368  144 - #b3cdec
     472  172 - #b7d0ec
     124  180 - #b8d0ec
     280  224 - #bbd3ed
      32  244 - #bed5ed
     592  244 - #bed5ed
     392  248 - #bed4ed
     492  272 - #c0d6ed
       4  352 - #688259
     368  356 - #583421
     208  360 - #583421
     148  368 - #583421
     424  372 - #c9c1b1
     428  372 - #c9c1b1
     432  372 - #c9c1b1
     436  372 - #c9c1b1
     440  372 - #c9c1b1
     444  372 - #c9c1b1
     448  372 - #c9c1b1
     452  372 - #c9c1b2
     456  372 - #c9c1b2
     496  372 - #cac1b2
     500  372 - #cac1b2
     504  372 - #cac1b2
     384  376 - #583421
     452  376 - #c9c1b1
     456  376 - #c9c1b1
     496  376 - #cac1b2
     500  376 - #cac1b2
     508  376 - #b1a99d
     512  376 - #b1a99d
     516  376 - #b1a99d
     520  376 - #b1a99d
     524  376 - #b1a99d
     528  376 - #b1a99d
     532  376 - #999288
     536  376 - #999288
     540  376 - #999288
     264  384 - #2e609d
     272  384 - #3267a8
     360  384 - #605b3d
     252  388 - #285085
     280  388 - #366bae
     204  392 - #5c472f
     264  392 - #2c5992
     240  396 - #1d365c
     256  396 - #264d80
     272  396 - #325f9a
     284  396 - #577bb4
     288  396 - #5179b6
     292  396 - #4273b5
     168  400 - #583421
     280  400 - #5676ab
      72  404 - #688258
     264  404 - #274e81
     292  404 - #7691c2
     296  404 - #4874b2
     376  404 - #583421
     464  404 - #583421
     248  408 - #1c3457
     272  408 - #2c5488
     288  408 - #758dba
     296  408 - #4872ae
     232  412 - #1c3457
     492  412 - #583421
     592  412 - #6a845b
     148  416 - #583421
     276  416 - #294f82
     292  416 - #38629b
     256  420 - #1c3457
     284  420 - #2c5488
     304  420 - #3164a3
     296  424 - #2d5a94
     228  428 - #1c3457
     244  428 - #1c3457
     288  428 - #274e81
     264  432 - #1c3457
     276  432 - #1f3b62
     364  432 - #583421
     168  436 - #955a37
     304  436 - #2a558d
     468  436 - #583421
     168  440 - #955a37
     232  444 - #1c3457
     252  444 - #1c3457
     276  444 - #1c3457
     292  444 - #203f6a
     168  448 - #aa673e
     168  452 - #aa673e
     264  452 - #1c3457
     384  452 - #583421
     168  456 - #aa673e
     244  456 - #1c3457
     168  460 - #aa673e
     284  460 - #1c3457
     496  460 - #583421
     168  464 - #aa673e
     256  464 - #1c3457
     272  464 - #1c3457
     168  468 - #aa673e
     464  468 - #583421
      20  472 - #678157
     168  472 - #aa673e
     168  476 - #aa673e
     592  476 - #69835a
     144  480 - #583421
     480  496 - #583421
     336  504 - #678258
      80  512 - #678157
     216  524 - #678157
     396  532 - #678258
     280  544 - #678157
     520  564 - #688258
     112  580 - #678157
       4  592 - #678157
     216  592 - #678157
     344  592 - #678157
     448  592 - #678258
     592  592 - #688259
";

const AFTERNOON_COLORS: &str = r"
       4    4 - #abc7ec
     464    4 - #a9c6ec
     592    4 - #abc7ec
     220    8 - #a8c5ec
     344    8 - #a8c5ec
     112   44 - #acc8ec
     240  112 - #b1cbec
     408  116 - #ccdef1
     572  120 - #b4ceec
      32  128 - #b4ceec
     360  128 - #cddef2
     432  148 - #d5e4f3
     396  164 - #f0f5f3
     152  168 - #b6cfec
     328  168 - #ccdef1
     464  168 - #ccdef1
     384  172 - #f6f6ec
     400  172 - #f7f4e4
     392  176 - #f8f2dc
     380  180 - #f2f5f0
     388  180 - #f8f2df
     396  180 - #f8f2dc
     404  180 - #f7f5e8
     392  184 - #f8f2de
     384  188 - #f4f6ee
     400  188 - #f6f6ea
     352  192 - #d9e7f3
     392  192 - #f3f6ef
     436  200 - #d8e7f3
     332  216 - #cddff1
     388  228 - #d9e7f3
     444  236 - #ccdff0
     592  240 - #bed5ed
     228  244 - #bdd4ed
     364  252 - #ccdff0
       4  256 - #bfd6ed
     408  256 - #cddff0
      60  352 - #4e5c47
     248  352 - #606c5b
     280  352 - #667161
     312  352 - #6c7667
     372  352 - #757e71
     376  352 - #757f71
     380  352 - #767f71
     384  352 - #767f72
     392  352 - #767f72
     400  352 - #767f72
     408  352 - #767f72
     420  352 - #757e71
     564  352 - #606c5b
     364  356 - #583421
     368  356 - #583421
     212  360 - #583421
     364  360 - #735344
     368  360 - #735444
     372  360 - #745545
     148  368 - #583421
     424  372 - #aba7a1
     428  372 - #aaa6a1
     432  372 - #a9a5a0
     436  372 - #a9a59f
     440  372 - #a8a49e
     444  372 - #a7a39e
     448  372 - #a6a29d
     452  372 - #a5a19c
     456  372 - #a4a09b
     496  372 - #9c9791
     500  372 - #9b9690
     504  372 - #9a968f
     452  376 - #a4a09a
     456  376 - #a39f9a
     496  376 - #9b9790
     500  376 - #9a9690
     512  376 - #8c8881
     524  376 - #8a8680
     532  376 - #7e7974
     540  376 - #7d7973
     268  384 - #36486a
     316  384 - #636e5e
     372  392 - #583421
     204  396 - #573f2c
     244  396 - #1c3457
     168  404 - #583421
     264  404 - #1c3457
     292  404 - #1c3457
     344  404 - #616d5c
     472  408 - #583421
     532  408 - #67635e
     396  412 - #626d5c
     436  412 - #67635e
     248  416 - #1c3457
       4  420 - #485740
     228  420 - #1c3457
     368  420 - #583421
     268  424 - #1c3457
     504  424 - #67635e
      84  428 - #4a5942
     304  432 - #1c3457
     284  436 - #1c3457
     244  440 - #1c3457
     548  440 - #67635e
     264  444 - #1c3457
     460  444 - #67635e
     168  448 - #724b38
     168  452 - #724b37
     368  452 - #583421
     168  456 - #724b37
     284  456 - #1c3457
     168  460 - #724a37
     496  460 - #583421
     168  464 - #724a36
     256  464 - #1c3457
     168  468 - #724a36
     168  472 - #724a36
     168  476 - #714935
     144  480 - #583421
     464  480 - #583421
     496  496 - #583421
      52  508 - #46563e
     592  508 - #4d5c46
     396  520 - #505e49
     320  552 - #4c5b44
     504  584 - #4a5943
       4  592 - #44543c
     116  592 - #45563d
     228  592 - #485740
     412  592 - #4a5943
     592  592 - #495841
";

const SUNSET_COLORS: &str = r"
      20    4 - #96afd0
     128    4 - #94aed0
     276    4 - #93add0
     368    4 - #93aed0
     468    4 - #94aed0
     592    4 - #96b0d0
     176   64 - #98b1d1
     552   72 - #9ab3d1
     248   92 - #9ab2d1
     480  100 - #9bb3d1
       4  108 - #a1b5cf
     116  116 - #9eb4d0
     368  116 - #9cb4d1
     188  140 - #a3b5ce
     592  140 - #a7b6cd
     520  164 - #abb7cb
     260  172 - #adb7ca
     440  184 - #b0b8c9
     192  216 - #bcbbc4
     324  220 - #bcbac4
      60  224 - #bcb9c2
     132  224 - #bcbac2
     392  228 - #bcb9c2
     468  236 - #bcbac2
     528  240 - #bcbac3
     592  256 - #bebbc3
     272  264 - #ddd1ca
       4  268 - #c4bbbd
     220  268 - #ded1c9
     360  272 - #c9bcbc
     424  276 - #c6bbbb
      72  280 - #c8bbba
     248  280 - #eadace
     204  284 - #e2d2c7
     128  292 - #cebdb8
     276  292 - #ecdacb
     548  292 - #c6bcbd
     224  296 - #ecdaca
     304  300 - #e5d2c5
     492  300 - #c9bcba
     192  304 - #e5d1c3
     244  312 - #f6ebd6
     248  312 - #f6eed8
     252  312 - #f6eed9
     256  312 - #f6ecd7
     244  316 - #f6eed8
     248  316 - #f7f0da
     252  316 - #f7f0db
     256  316 - #f7efd9
     260  316 - #f6ebd6
     212  320 - #eed8c4
     248  320 - #f7f0db
     252  320 - #f7f0db
     256  320 - #f7efda
     260  320 - #f6ecd6
     288  320 - #eed8c5
     176  324 - #e2cabb
     244  324 - #f6edd7
     248  324 - #f7efd9
     252  324 - #f7efda
     256  324 - #f6eed8
     312  324 - #e7cfbe
     592  324 - #c9bdbb
     252  328 - #f6ecd6
     392  328 - #d3bcb1
      80  336 - #d4bcb0
     332  340 - #e0c5b4
       4  348 - #cdb9b0
     180  348 - #dfc6b4
     216  348 - #ecd3bd
     240  348 - #efd7c0
     268  348 - #eed6bf
     292  348 - #ead1bc
     436  372 - #524e4c
     496  372 - #524e4c
     416  376 - #4f4c4b
     524  376 - #504d4b
     456  380 - #4f4c4b
     264  384 - #132746
     280  388 - #132746
     248  392 - #132746
     544  392 - #4f4c4b
     292  396 - #132746
     416  396 - #464641
     512  396 - #4f4c4b
     436  400 - #4f4c4b
     264  404 - #132746
     456  404 - #4f4c4b
     120  408 - #2b3425
     284  408 - #132746
     304  412 - #132746
     548  412 - #4f4c4b
     228  416 - #132746
     248  420 - #132746
     528  420 - #4f4c4b
     268  424 - #132746
     448  424 - #4f4c4b
     496  424 - #4c433e
     288  428 - #132746
     304  428 - #132746
     424  428 - #4f4c4b
      40  432 - #293324
     256  440 - #132746
     276  440 - #132746
     232  444 - #132746
     300  444 - #132746
     364  444 - #432719
     460  444 - #4f4c4b
     516  444 - #4f4c4b
     548  444 - #454641
     252  460 - #132746
     280  460 - #132746
     140  488 - #293324
      72  496 - #283324
       4  508 - #283324
     316  508 - #293324
     376  508 - #293324
     444  512 - #283324
     592  512 - #273223
     208  520 - #293324
     508  536 - #283223
     304  568 - #283324
      96  572 - #283324
     364  580 - #283324
       4  592 - #283223
     188  592 - #283324
     424  592 - #283223
     592  592 - #273223
";

const DUSK_COLORS: &str = r"
     144    4 - #607189
     244    4 - #5f7089
     368    4 - #5f7189
     488    4 - #607189
     592    4 - #617289
       4    8 - #617289
     316   80 - #637389
     420   80 - #637389
     208   88 - #637489
     120  100 - #657489
     532  104 - #667589
       4  132 - #707989
     252  164 - #727a89
     348  172 - #747a89
     448  180 - #757b89
     592  200 - #787d89
     108  248 - #8a8487
     212  248 - #8b8488
       4  252 - #8a8487
     292  256 - #8a8487
     380  264 - #8a8487
      56  268 - #8e8587
     160  268 - #918888
     476  280 - #8a8487
       4  308 - #948886
      92  308 - #978a87
     360  308 - #948886
     588  308 - #8a8588
     420  312 - #938887
     528  312 - #8d8687
     188  316 - #a89a91
     212  320 - #ab9c92
     168  324 - #a99a90
     236  324 - #a99a90
     312  324 - #9a8c87
     180  328 - #ad9d92
     204  332 - #b1a195
     220  332 - #b0a094
     248  332 - #aa9a8f
     192  336 - #b2a295
     148  340 - #aa998f
     164  340 - #ae9e92
     228  344 - #b2a194
      44  348 - #988983
     176  348 - #af9f93
     208  348 - #b3a395
     248  348 - #aa9a8f
     372  356 - #462c21
     424  372 - #2c2c30
     472  372 - #462b20
     500  372 - #2c2c30
     152  376 - #462c21
     532  376 - #404046
     536  376 - #404046
     540  376 - #404046
     368  380 - #462c21
     516  380 - #53545c
     496  388 - #53545c
     548  388 - #333738
     208  392 - #462c21
     268  392 - #0b2147
     456  392 - #53545c
     528  392 - #53545c
     548  392 - #43464a
     256  396 - #0b2147
     384  396 - #462c21
     512  396 - #53545c
     548  396 - #43464a
       4  400 - #131b15
     292  400 - #0b2147
     272  404 - #0d2550
     496  404 - #53545c
     536  404 - #53545c
     244  408 - #0b2248
     164  412 - #462c21
     260  412 - #0f2751
     284  412 - #0e2651
     368  412 - #462c21
     460  412 - #474348
     548  412 - #53545c
     460  416 - #474348
     528  416 - #53545c
     496  420 - #504a4d
      80  424 - #131b15
     236  424 - #0b2146
     268  424 - #2e3e65
     272  424 - #2c3c64
     300  424 - #0c234b
     496  424 - #504a4d
     512  424 - #53545c
     264  428 - #21335c
     268  428 - #304067
     272  428 - #2d3d65
     456  428 - #53545c
     548  428 - #53545c
     204  432 - #462c21
     268  432 - #21335d
     288  432 - #0e2752
     376  436 - #462c21
     500  436 - #53545c
     532  436 - #53545c
     244  444 - #0c234a
     260  444 - #0e2752
     456  444 - #53545c
     516  444 - #53545c
     544  444 - #43464a
     548  444 - #43464a
     164  448 - #462c21
     276  448 - #0e2651
     296  448 - #0b2147
     260  460 - #0b2148
     484  464 - #462b20
      12  468 - #131b14
     144  480 - #462c20
     484  496 - #462b20
     404  512 - #121b14
     592  512 - #121b14
     212  520 - #121b14
      72  524 - #121b14
     308  536 - #121b14
     532  540 - #121b14
     436  564 - #121b14
       4  592 - #121b14
     120  592 - #121b14
     236  592 - #121b14
     380  592 - #121b14
     492  592 - #121b14
     592  592 - #121b14
";

const NIGHT_COLORS: &str = r"
       4    4 - #0a0a19
     168    4 - #0a0a19
     300    4 - #0d0d1b
     436    4 - #0c0c1b
     592    4 - #0a0a19
     292   76 - #1f212c
     444   96 - #1f212c
      32  100 - #0b0b1a
     372  104 - #333540
     300  112 - #333540
     124  132 - #0c0c1b
     328  136 - #474a57
     368  136 - #474a56
     256  144 - #333540
     292  152 - #474a56
     400  152 - #474b57
     492  156 - #1f202c
     364  168 - #5a5e6c
     328  172 - #5c606e
     592  172 - #0c0d1c
     444  176 - #3b3e4a
     300  188 - #5b5f6d
       4  192 - #0b0b1b
     396  192 - #5a5e6c
     260  196 - #474a56
     320  196 - #656978
     372  200 - #656a78
     344  204 - #7f8491
     348  204 - #7c808e
     340  208 - #afb3bf
     344  208 - #b5b9c6
     348  208 - #b3b7c3
     304  212 - #636776
     336  216 - #d9dde9
     344  216 - #e0e4f0
     352  216 - #dbe0ec
     356  220 - #dbdfeb
     256  224 - #474b57
     324  224 - #9ca1ad
     332  224 - #dce0ec
     340  224 - #e5e9f4
     344  224 - #e5eaf4
     348  224 - #e5e9f4
     360  224 - #cfd4e0
     364  224 - #adb1bd
     368  224 - #6f7382
     436  224 - #474b57
     228  228 - #343742
     284  228 - #5a5e6b
     324  228 - #9fa3b0
     328  228 - #c8ccd8
     340  228 - #e5e9f4
     344  228 - #e6eaf5
     348  228 - #e5eaf4
     364  228 - #aeb2bf
     368  228 - #717584
     404  228 - #5b5f6d
     324  232 - #979ca8
     328  232 - #c2c7d3
     332  232 - #dadeea
     336  232 - #e1e6f1
     344  232 - #e5e9f4
     356  232 - #dde1ed
     336  236 - #dde1ed
     340  236 - #e1e5f1
     344  236 - #e2e6f2
     352  236 - #dee3ee
     104  240 - #0d0d1c
     340  240 - #d9dee9
     344  240 - #dbe0ec
     348  240 - #dadfea
     304  244 - #626775
     340  244 - #c0c4d0
     344  244 - #c4c9d5
     348  244 - #c1c6d2
     340  248 - #9397a4
     344  248 - #9a9eab
     348  248 - #959aa6
     464  248 - #343641
     264  256 - #494d59
     372  256 - #646977
     428  256 - #494c58
     396  260 - #5a5e6b
     548  260 - #111220
     300  268 - #5a5e6c
     328  276 - #5f6371
     368  280 - #5b5f6d
     244  284 - #343742
     276  284 - #474a57
     348  284 - #5c606e
     416  284 - #464956
     452  284 - #323540
     308  304 - #4a4d5a
     344  304 - #4f5360
     392  304 - #464a56
     284  328 - #333541
     100  352 - #252d30
     320  352 - #434b57
     468  352 - #333b43
     592  352 - #272e33
     364  356 - #0d0605
     368  356 - #0d0605
     364  360 - #28242a
     368  360 - #272329
     372  360 - #272328
     232  368 - #303840
       4  372 - #1e2627
     436  372 - #474b5a
     500  372 - #3c3f4b
     280  376 - #363d47
     348  400 - #353c45
     556  408 - #232b2e
     308  412 - #313840
     392  412 - #30383f
     184  436 - #232b2e
     104  440 - #1e2627
     504  460 - #222a2c
     592  464 - #1e2627
     376  472 - #272f33
       4  484 - #192120
     248  488 - #222a2c
     152  516 - #1c2424
     464  544 - #1e2626
     388  552 - #1f2728
     312  564 - #1e2626
       4  592 - #161f1c
     216  592 - #1a2322
     584  592 - #192120
";
