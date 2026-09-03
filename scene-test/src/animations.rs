use std::f32::consts::FRAC_PI_4;

use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::{
        LossyConvert,
        volume::{Quat, Shape3, Vec3},
    },
    refs::{Weak, manage::DataManager},
    scene::{Camera, Model, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, scene},
    ui::Color,
    ui_test::{capture_screenshot, check_colors, set_record_probe_count},
};

/// Frames the clips run free before the bar's clock is checked.
const PLAY_FRAMES: usize = 31;
/// Frames past the end of the longest clip, the two second spin, enough
/// for every single run to finish.
const PAST_END_FRAMES: usize = 130;
/// Frames of the 45 degree turn of the camera before the last check.
const TURN_FRAMES: usize = 20;
/// How far into its turn the windmill is frozen, off any symmetric
/// position of its four blades.
const WINDMILL_TURN: f32 = 0.3;
/// How far into its run the fox is frozen, legs apart mid stride.
const FOX_STRIDE: f32 = 0.3;
/// The fox is modeled in centimeters, this brings it to the bar's size.
const FOX_SCALE: f32 = 0.09;
const BEND: &str = "Bend";
const RUN: &str = "Run";
const SPIN: &str = "Spin";
/// How far the clock may sit from the frame count that drove it. A
/// wait for a frame can wake the main loop twice, so one step of slack.
const TIME_TOLERANCE: f32 = 1.5 / 60.0;

/// Three animated models. The skinned bar of `BoneTest.glb`, four bones
/// in a chain, twice: one at rest behind and one in front that plays its
/// bend. The Khronos sample fox of `Fox.glb`, 24 joints under a texture,
/// running on the spot, scaled from its centimeters down to the bar's
/// size. And the windmill of `windmill.glb`, whose blades ride on a hub
/// that spins from a plain node clip with no skin. The sun casts, so the
/// shadows come from the skinned shadow pass too. The first check pins
/// everything at rest, the second the bar bent, the fox mid stride and
/// the blades turned while the back bar has not moved, and the last every
/// clip played once and held on its last frame, seen after the camera
/// turned 45 degrees around the scene. The loop
/// runs free, so the frames between two waits vary by one, and the
/// middle frame is pinned by freezing each clip at a chosen time before
/// the check. The scene starts every clip in its setup, so a presented
/// scene moves, and the first check comes after `stop_animation`, which
/// puts every model back at rest.
#[scene]
#[derive(Default)]
struct Animations {
    bar:      Weak<Prop>,
    fox:      Weak<Prop>,
    windmill: Weak<Prop>,
}

impl SceneSetup for Animations {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(3.5, 15.0, 27.0),
            target: Vec3::new(3.5, 1.5, -1.0),
            ..Camera::default()
        };
        self.sun.direction = Vec3::new(-0.4, -1.0, -0.3);
        self.sun.shadows = true;

        self.make_node::<Prop>(Shape3::Plane(70.0), Vec3::new(3.5, -1.2, 0.0))
            .set_color(Color::hex("#c8ccd0"))
            .set_roughness(0.9);

        self.make_node::<Prop>(
            Shape3::Model(Model::get("BoneTest.glb")),
            Vec3::new(1.0, 0.0, -6.0),
        )
        .set_color(Color::hex("#7f8fa6"))
        .set_roughness(0.6);

        self.bar = self.make_node::<Prop>(
            Shape3::Model(Model::get("BoneTest.glb")),
            Vec3::new(1.0, 0.0, -1.0),
        );
        self.bar.set_color(Color::hex("#e67e22")).set_roughness(0.6);

        self.fox = self.make_node::<Prop>(Shape3::Model(Model::get("Fox.glb")), Vec3::new(-5.0, -1.2, 7.0));
        self.fox
            .set_scale(FOX_SCALE)
            .set_rotation(Quat::from_rotation_y(0.9))
            .set_roughness(0.7);

        self.windmill = self.make_node::<Prop>(
            Shape3::Model(Model::get("windmill.glb")),
            Vec3::new(15.0, -1.2, -4.0),
        );
        self.windmill.set_color(Color::hex("#8d6e4a")).set_roughness(0.8);

        // Everything plays from the start, so a presented scene moves.
        // The test stops it all for its first check.
        self.bar.play(BEND);
        self.fox.play(RUN);
        self.windmill.play(SPIN);
    }
}

/// Seconds the clip called `clip` of the model file `model` runs.
fn duration(model: &str, clip: &str) -> f32 {
    let model = Model::get(model);
    model.clips()[model.clip(clip).expect("the model has the clip")].duration
}

impl SceneTest for Animations {
    fn perform_test(mut scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);

        from_main(move || {
            let (mut bar, mut fox, mut windmill) = (scene.bar, scene.fox, scene.windmill);
            bar.stop_animation();
            fox.stop_animation();
            windmill.stop_animation();
        });
        wait_for_next_frame();
        wait_for_next_frame();
        check_colors(REST)?;

        from_main(move || {
            let (mut bar, mut fox, mut windmill) = (scene.bar, scene.fox, scene.windmill);
            bar.play(BEND);
            fox.play(RUN);
            windmill.play(SPIN);
        });
        let started = from_main(move || scene.bar.animation_time().expect("the clip plays"));
        for _ in 0..PLAY_FRAMES {
            wait_for_next_frame();
        }

        let time = from_main(move || scene.bar.animation_time().expect("the clip still plays"));
        let frames: f32 = PLAY_FRAMES.lossy_convert();
        let expected = frames / 60.0;
        anyhow::ensure!(
            (time - started - expected).abs() < TIME_TOLERANCE,
            "the clip moved from {started} to {time} over {PLAY_FRAMES} frames, expected {expected}"
        );

        // Frozen at chosen times, so the check reads the same poses
        // whatever the frame count until it.
        let middle = from_main(move || {
            let (mut bar, mut fox, mut windmill) = (scene.bar, scene.fox, scene.windmill);
            let middle = duration("BoneTest.glb", BEND) / 2.0;
            bar.set_animation_speed(0.0).set_animation_time(middle);
            fox.set_animation_speed(0.0)
                .set_animation_time(duration("Fox.glb", RUN) * FOX_STRIDE);
            windmill
                .set_animation_speed(0.0)
                .set_animation_time(duration("windmill.glb", SPIN) * WINDMILL_TURN);
            middle
        });
        wait_for_next_frame();
        wait_for_next_frame();
        let frozen = from_main(move || scene.bar.animation_time());
        anyhow::ensure!(frozen == Some(middle), "frozen at {frozen:?}, not at {middle}");
        capture_screenshot()?;
        check_colors(MIDDLE)?;

        from_main(move || {
            let (mut bar, mut fox, mut windmill) = (scene.bar, scene.fox, scene.windmill);
            bar.play_once(BEND);
            fox.play_once(RUN);
            windmill.play_once(SPIN);
        });
        let turn: f32 = TURN_FRAMES.lossy_convert();
        for _ in 0..TURN_FRAMES {
            from_main(move || scene.camera.orbit(FRAC_PI_4 / turn, 0.0));
            wait_for_next_frame();
        }
        for _ in 0..PAST_END_FRAMES - TURN_FRAMES {
            wait_for_next_frame();
        }

        let (time, animating, bend) = from_main(move || {
            (
                scene.bar.animation_time(),
                scene.bar.is_animating() || scene.fox.is_animating() || scene.windmill.is_animating(),
                duration("BoneTest.glb", BEND),
            )
        });
        anyhow::ensure!(
            time == Some(bend) && !animating,
            "a single run holds its end: the bar at {time:?} of {bend}, still animating {animating}"
        );
        check_colors(HELD)
    }
}

const REST: &str = r"
     252    4 - #597c95
     592    4 - #597c95
     420   52 - #597c95
       4   64 - #597c95
     148  120 - #597c95
     304  132 - #597c95
      12  192 - #597c95
     492  212 - #a03e3e
     484  236 - #a03e3e
     484  240 - #a03e3e
     484  248 - #505054
     488  248 - #505054
     440  252 - #b28385
     448  252 - #b28385
     460  252 - #b28385
     480  252 - #3e3e41
     484  252 - #505054
     488  252 - #505054
     496  252 - #b28385
     508  252 - #b28385
     528  252 - #b28385
     484  256 - #7b3030
     480  260 - #7b3030
     476  264 - #4e3f2e
     484  264 - #a03e3e
     184  272 - #7f8ea3
     240  272 - #7f8ea3
     272  272 - #7f8ea3
     316  272 - #7f8ea3
     352  272 - #7f8ea3
     384  272 - #7f8ea3
     416  272 - #7f8ea3
     476  272 - #4e3f2e
     476  288 - #a03e3e
     472  292 - #4e3f2e
     476  296 - #6e573f
     136  300 - #d99158
     140  300 - #d99158
     172  300 - #bf7843
     224  300 - #bf7843
     348  300 - #bf7843
     424  300 - #d99159
     428  300 - #d99159
     432  300 - #d99159
     476  300 - #725b42
     268  308 - #e07e31
     440  308 - #6e7072
     472  308 - #5e4b37
     388  312 - #e07e32
     448  312 - #6e7072
     452  312 - #6e7072
     464  312 - #c4c8cc
      48  316 - #b5aca2
      96  316 - #ca7f32
     308  316 - #a45c26
     472  316 - #68533c
      44  324 - #aa6b2c
     124  324 - #c67c31
     184  328 - #b2afa9
      52  336 - #724921
      80  336 - #8e5926
     148  340 - #b1aea8
     404  340 - #a45c26
      32  344 - #8b8883
     100  344 - #905a26
     156  344 - #b1aea8
     220  344 - #976038
     260  344 - #976038
     292  344 - #976138
     332  344 - #976138
     368  344 - #976138
     436  344 - #ac7750
     152  348 - #b1aea8
      48  356 - #784b21
     120  356 - #9b6128
     140  360 - #784b21
      64  364 - #714721
      88  364 - #784b21
     184  364 - #e1944f
     184  368 - #e1944f
     188  368 - #e29654
      36  372 - #6e7072
      80  372 - #c4c8cb
     188  372 - #e39755
      64  376 - #4f3f32
      88  376 - #c4c8cb
     124  376 - #784b21
     160  376 - #8b8984
     144  380 - #8b8984
     148  380 - #8b8984
     172  380 - #8b8984
     176  380 - #8b8984
     184  380 - #a7a49e
     192  380 - #efebe3
      96  384 - #32251c
     104  384 - #c4c8cb
      76  388 - #32251c
     108  388 - #c4c8cb
     152  388 - #784b21
     100  392 - #32251c
     144  392 - #34271d
      56  396 - #6e7072
     136  400 - #c4c8cb
     156  400 - #a3a2a0
      76  404 - #32251c
      96  404 - #6e7072
      80  408 - #32251c
     132  408 - #32251c
     144  408 - #c4c8cb
     124  416 - #32251c
     156  416 - #32251c
     132  420 - #32251c
     104  424 - #6e7072
     152  424 - #32251c
     592  424 - #c4c8cc
     160  428 - #32251c
     128  432 - #32251c
     136  440 - #32251c
     116  444 - #6e7072
     152  444 - #6e7072
     132  448 - #6e7072
     168  452 - #6e7072
     296  472 - #c4c8cb
     460  496 - #c4c8cb
       4  592 - #c4c8cb
     200  592 - #c4c8cb
     344  592 - #c4c8cb
     592  592 - #c4c8cb
";

const MIDDLE: &str = r"
       4    4 - #597c95
     176    4 - #597c95
     292    4 - #597c95
     432    4 - #597c95
     592    4 - #597c95
     304  116 - #e68235
     308  116 - #e68235
     332  128 - #e68235
     592  128 - #597c95
      84  136 - #597c95
     336  164 - #975424
     288  168 - #7e471f
     324  196 - #975524
     288  208 - #985524
     476  208 - #a03e3e
     528  236 - #a03e3e
     512  240 - #a03e3e
     276  244 - #7e471f
     500  244 - #a03e3e
     484  248 - #505054
     488  248 - #505054
     312  252 - #a25b25
     472  252 - #a03e3e
     484  252 - #505054
     488  252 - #505054
     460  256 - #a03e3e
     484  260 - #7b3030
     440  264 - #a03e3e
     476  264 - #4e3f2e
     188  272 - #7f8ea3
     232  272 - #7f8ea3
     364  272 - #7f8ea3
     400  272 - #7f8ea3
     152  276 - #7f8ea3
     260  276 - #7f8ea3
     288  276 - #a85e26
     476  276 - #4e3f2e
      36  284 - #cc8033
     324  284 - #a75e26
     492  284 - #a03e3e
     332  288 - #e17f31
     304  292 - #a85e26
     476  292 - #6a543d
      60  296 - #ce8133
     476  296 - #6e573f
     388  300 - #9f7355
     392  300 - #9f7355
     396  300 - #9e7355
     404  300 - #bf7843
     408  300 - #bf7843
     412  300 - #bf7843
     424  300 - #d99159
     428  300 - #d99159
     432  300 - #d99159
     476  300 - #705941
      20  304 - #8b8883
     176  308 - #6e7072
     220  308 - #6e7072
     260  308 - #6e7072
     440  308 - #6e7072
     464  308 - #6e7072
     312  312 - #a75d26
     452  312 - #6e7072
     456  312 - #6e7072
     464  312 - #c4c8cc
      96  316 - #cc8033
     344  316 - #a65d26
     472  316 - #68533c
     120  328 - #cd8133
     376  332 - #a45c26
     408  332 - #a45c26
      60  336 - #8c5826
     352  336 - #a55c26
     144  340 - #b7722e
     200  340 - #b6b2ad
     280  340 - #6e7072
      92  344 - #cb8033
     320  344 - #6e7072
     436  344 - #ac7750
     196  352 - #b6b2ad
      20  356 - #6e7072
     116  356 - #bd772f
     160  360 - #b5b1ab
     164  360 - #b5b1ab
     172  364 - #b5b1ab
     168  368 - #b5b1ab
     136  372 - #a06429
     164  372 - #b3aca2
      52  376 - #32251c
      60  380 - #32251c
     108  388 - #784b21
      60  392 - #32251c
     160  392 - #8b8984
     200  396 - #c4c0ba
     156  400 - #8b8984
     168  400 - #8b8984
     184  400 - #8d8a85
     188  400 - #8d8a85
     196  400 - #a8a5a0
     204  400 - #efebe3
     208  400 - #efebe3
     180  404 - #32251c
     192  404 - #8b8984
     200  404 - #a8a5a0
     196  408 - #33261c
     196  412 - #32251c
     156  416 - #32251c
     208  416 - #4a3523
     212  416 - #513a26
     220  420 - #58412d
     212  424 - #33261c
     592  424 - #c4c8cc
     104  428 - #6e7072
     164  428 - #4e3724
     164  436 - #32251c
     172  436 - #4d3624
     176  444 - #493322
     180  452 - #513926
     192  452 - #6e7072
     200  456 - #6e7072
     184  460 - #32251c
     204  460 - #6e7072
     336  472 - #c4c8cb
     464  484 - #c4c8cb
       4  592 - #c4c8cb
     144  592 - #c4c8cb
     376  592 - #c4c8cb
     592  592 - #c4c8cb
";

const HELD: &str = r"
       4    4 - #597c95
     316    4 - #597c95
     592    4 - #597c95
     160   72 - #597c95
     452  104 - #597c95
     276  128 - #597c95
       4  144 - #597c95
     592  144 - #597c95
     128  240 - #eae5de
     264  248 - #7f8ea3
     128  256 - #c77d31
     292  256 - #7f8ea3
     520  260 - #a03e3e
      92  264 - #a2662d
     136  264 - #906d4e
     212  264 - #e07e31
     244  264 - #767f8c
     524  268 - #a33f3f
      60  272 - #c17930
      64  272 - #c17930
     312  272 - #7f8ea3
     340  272 - #7f8ea3
      92  276 - #bab7b1
     516  276 - #a03e3e
      88  280 - #bab7b1
      92  280 - #bab7b1
      76  284 - #df9658
      80  284 - #df9658
     140  284 - #6e7072
     224  288 - #a45b25
      76  292 - #df9658
     196  292 - #a45b25
     272  292 - #e07e31
     364  292 - #7f8ea3
     512  292 - #9f3e3e
      76  296 - #df9557
     456  296 - #9f3e3e
      76  300 - #e1934d
     100  300 - #898178
     116  300 - #8b8984
     396  300 - #7f8ea3
     468  300 - #9f3e3e
      68  304 - #e09148
      76  304 - #e1934d
     128  304 - #514b47
     132  304 - #32251c
     480  304 - #9f3e3e
     516  304 - #9f3e3e
      72  308 - #e0924a
     112  308 - #32251c
     132  308 - #32251c
     492  308 - #9f3e3e
      92  312 - #8b8984
     104  312 - #c4c8cb
     132  312 - #32251c
     236  312 - #a45b25
     292  312 - #e07e31
     508  312 - #4f4f54
      80  316 - #aeaba5
     108  316 - #32251c
     112  316 - #32251c
     132  316 - #32251c
     320  316 - #e07e31
     428  316 - #7f8ea3
     504  316 - #4f4f54
     516  316 - #555559
      84  320 - #784b21
     348  320 - #e07e31
     504  320 - #4f4f54
     512  320 - #4f4f54
     532  320 - #a03e3e
      92  324 - #32261c
     112  324 - #32251c
     508  324 - #7b3030
     516  324 - #735c42
      88  328 - #32251c
      92  328 - #32251c
     128  328 - #6e7072
     456  328 - #7f8ea3
     504  328 - #7b3030
     508  328 - #7b3030
     544  328 - #a03e3e
     560  328 - #a03e3e
      88  332 - #32251c
     272  332 - #a35b25
      84  336 - #32251c
      92  336 - #c4c8cb
     108  336 - #6e7072
     120  336 - #6e7072
     372  336 - #e07e31
     572  336 - #a03e3e
      92  340 - #c4c8cb
     396  340 - #e07e31
     504  340 - #a03e3e
      84  344 - #32251c
     512  344 - #725b41
     120  348 - #6e7072
     304  348 - #a35b25
     512  348 - #715a41
     432  352 - #474f5b
     436  352 - #4b5461
      84  356 - #4d3725
     104  356 - #6e7072
     440  356 - #474f5b
     444  356 - #4a525f
     500  356 - #a03e3e
     412  360 - #e07e31
     452  360 - #474f5b
     508  360 - #735c42
     336  364 - #a35b25
     456  364 - #c4c8cb
     468  368 - #6e7072
      96  372 - #6e7072
     500  372 - #50402f
     476  380 - #6e7072
     484  380 - #6e7072
     376  384 - #a35b25
     500  384 - #6b553e
     496  392 - #5b4935
     408  396 - #ae6127
     248  460 - #c4c8cb
       4  464 - #c4c8cb
     592  468 - #c4c8cb
     132  516 - #c4c8cb
     424  552 - #c4c8cb
       4  592 - #c4c8cb
     256  592 - #c4c8cb
     592  592 - #c4c8cb
";
