use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::{
        LossyConvert,
        volume::{Shape3, Vec3},
    },
    refs::Weak,
    scene::{Light, NodeTemplates, SceneCreation, SceneSetup, SceneTest, Wall, scene},
    ui::{Color, Cursor, NamedKey},
    ui_test::{
        capture_screenshot, check_colors, inject_mouse_motion, inject_named_key, set_record_probe_count,
    },
};

/// Frames for the player to settle onto the floor.
const SETTLE_FRAMES: usize = 20;
/// A mouse moves a little every frame, so a turn arrives as a stream of
/// small motions: this many, one per frame, adding up to `MOTION`, a
/// quarter turn to the right and a little down at the default look
/// speed of 0.002 radians per unit.
const STEPS: usize = 60;
const MOTION: (f32, f32) = (785.0, 150.0);

/// A first person player between a blue post ahead and a red crate to
/// its right turns with the captured mouse: a stream of small motions to
/// the right and down sweeps the view off the post, past the empty floor
/// halfway, onto the crate, the yaw and the pitch add up to the whole
/// motion by the look speed and the camera follows the eyes. With the
/// mouse free, before the capture and after Escape, the same stream
/// turns nothing.
#[scene]
#[derive(Default)]
struct MouseLook {}

impl SceneSetup for MouseLook {
    fn needs_physics(&self) -> bool {
        true
    }

    fn setup(&mut self) {
        self.lights
            .push(Light::point(Vec3::new(0.0, 5.0, 0.0)).intensity(6.0).range(16.0));

        self.make_node::<Wall>(Shape3::Plane(30.0), Vec3::ZERO)
            .set_color(Color::hex("#8d9aa5"))
            .set_roughness(0.9);

        self.make_node::<Wall>(Shape3::cuboid(1.0, 4.0, 1.0), Vec3::new(0.0, 2.0, -5.0))
            .set_color(Color::hex("#3070d0"))
            .set_roughness(0.6);

        self.make_node::<Wall>(Shape3::cube(2.0), Vec3::new(5.0, 1.0, 0.0))
            .set_color(Color::hex("#d04030"))
            .set_roughness(0.6);

        self.add_player(Vec3::new(0.0, 1.0, 0.0));
    }
}

/// One frame of the stream, a `STEPS`th of the whole motion.
fn step() -> (f32, f32) {
    let steps: f32 = STEPS.lossy_convert();
    (MOTION.0 / steps, MOTION.1 / steps)
}

/// Streams `steps` frames of mouse motion, one motion per frame.
fn sweep(steps: usize) {
    for _ in 0..steps {
        inject_mouse_motion(step());
        wait_for_next_frame();
    }
    // The loop runs free, the last motion lands within one more frame.
    wait_for_next_frame();
}

impl SceneTest for MouseLook {
    fn perform_test(scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);

        let look = move || {
            from_main(move || {
                let player = scene.player.as_ref().expect("the scene has a player");
                (player.yaw, player.pitch, player.look_speed)
            })
        };
        // The stream adds up in floats, so the sum is a hair off.
        let close = |a: f32, b: f32| (a - b).abs() < 1e-4;

        for _ in 0..SETTLE_FRAMES {
            wait_for_next_frame();
        }
        sweep(STEPS);
        let (yaw, pitch, speed) = look();
        ensure!(
            yaw == 0.0 && pitch == 0.0,
            "a free mouse turned the player to {yaw} {pitch}"
        );
        check_colors(POST_AHEAD)?;

        from_main(Cursor::capture);
        ensure!(from_main(Cursor::captured), "the mouse is not captured");

        sweep(STEPS / 2);
        let (yaw, pitch, _) = look();
        let (half_yaw, half_pitch) = (MOTION.0 * speed / 2.0, -MOTION.1 * speed / 2.0);
        ensure!(
            close(yaw, half_yaw) && close(pitch, half_pitch),
            "half the stream turned the player to {yaw} {pitch}, wanted {half_yaw} {half_pitch}"
        );
        check_colors(HALFWAY)?;

        sweep(STEPS / 2);
        let (yaw, pitch, _) = look();
        let (wanted_yaw, wanted_pitch) = (MOTION.0 * speed, -MOTION.1 * speed);
        ensure!(
            close(yaw, wanted_yaw) && close(pitch, wanted_pitch),
            "the stream turned the player to {yaw} {pitch}, wanted {wanted_yaw} {wanted_pitch}"
        );

        let (ahead, direction) = from_main(move || {
            let player = scene.player.as_ref().expect("the scene has a player");
            (scene.camera.target - scene.camera.position, player.direction())
        });
        ensure!(
            (ahead - direction).length() < 1e-4,
            "the camera looks along {ahead:?}, the player along {direction:?}"
        );
        check_colors(CRATE_RIGHT)?;

        inject_named_key(NamedKey::Escape);
        ensure!(!from_main(Cursor::captured), "Escape did not free the mouse");

        sweep(STEPS);
        let (still_yaw, still_pitch, _) = look();
        ensure!(
            close(still_yaw, yaw) && close(still_pitch, pitch),
            "a freed mouse turned the player to {still_yaw} {still_pitch}"
        );
        capture_screenshot()?;
        check_colors(CRATE_RIGHT)
    }
}

const POST_AHEAD: &str = r"
               4    4 - #597c95
             152    4 - #597c95
             484    4 - #597c95
             568    4 - #597c95
             272   40 - #4b78cc
             356   40 - #4d79cc
             424   40 - #597c95
              76   52 - #597c95
             316   56 - #4d79cc
             524   56 - #597c95
             200   60 - #597c95
             244   76 - #4976ca
             356   84 - #4c78ca
             592   84 - #597c95
             288   88 - #4b77ca
               4  104 - #597c95
             140  108 - #597c95
             444  108 - #597c95
              72  116 - #597c95
             244  120 - #4774c7
             328  120 - #4a76c9
             524  124 - #597c95
             288  132 - #4975c8
             188  144 - #597c95
             356  152 - #4774c6
             588  160 - #597c95
             244  168 - #4472c4
             304  172 - #4673c5
              68  180 - #597c95
             432  184 - #597c95
               4  192 - #597c95
             356  196 - #4371c3
             504  204 - #597c95
             164  208 - #597c95
             264  212 - #416fc2
             312  220 - #426fc2
             592  232 - #597c95
             352  244 - #3f6ec0
              72  248 - #597c95
             252  256 - #3e6dbf
               4  260 - #597c95
             296  268 - #3e6cbe
             180  272 - #597c95
             444  272 - #597c95
             524  284 - #597c95
             356  288 - #3c6bbd
             272  292 - #3c6bbd
             116  296 - #597c95
             240  300 - #597c95
             320  300 - #3c6bbc
              52  308 - #597c95
             292  324 - #3a69bb
             592  324 - #597c95
             188  332 - #597c95
             356  336 - #3a69ba
             252  344 - #3968ba
              12  360 - #89959f
             436  360 - #89959f
             324  372 - #3868b9
             520  376 - #89959f
             152  380 - #89959f
             284  380 - #3867b8
             356  384 - #3867b8
              80  388 - #89959f
             244  388 - #3767b8
             592  404 - #8a96a0
             292  420 - #3666b6
             356  420 - #3666b6
              24  424 - #8b96a0
             468  424 - #8b97a1
             252  428 - #3666b6
             324  440 - #3665b6
             180  444 - #8c98a2
             412  448 - #8c98a3
             592  456 - #8c98a2
             104  460 - #8d99a3
             540  460 - #8d98a3
             244  472 - #3565b4
             256  472 - #3565b4
             268  476 - #3564b4
             280  476 - #3565b4
             292  476 - #3565b4
             304  476 - #3565b4
             324  476 - #3565b4
             332  476 - #3565b4
             340  476 - #3565b4
             352  476 - #3565b4
             260  480 - #3564b4
             316  480 - #3564b4
             248  484 - #3564b4
             272  484 - #3564b4
             288  484 - #3564b4
             300  484 - #3564b4
             324  484 - #3564b4
             344  484 - #3564b4
             356  484 - #3564b4
               4  488 - #8d99a4
             264  488 - #3564b4
             280  488 - #3564b4
             308  488 - #3564b4
              60  492 - #8e9aa4
             296  492 - #3564b4
             336  492 - #3564b4
             348  492 - #3564b4
             244  496 - #3464b4
             256  496 - #3564b4
             268  496 - #3564b4
             288  496 - #3564b4
             304  496 - #3564b4
             320  496 - #3564b4
             356  496 - #3564b4
             452  496 - #8f9ba5
             116  512 - #8f9ba6
             168  512 - #8f9ba6
             592  512 - #8e9ba5
             404  524 - #909ca6
              20  540 - #8f9ca6
              76  544 - #909ca7
             524  544 - #909ca7
             328  548 - #919da8
             224  568 - #919ea8
             376  576 - #929ea9
             152  588 - #929ea9
               4  592 - #919da8
             100  592 - #929ea9
             296  592 - #929fa9
             456  592 - #929ea9
             592  592 - #919da8
";

const HALFWAY: &str = r"
               4    4 - #597c95
             220    4 - #597c95
             444    4 - #597c95
             592    4 - #597c95
              76    8 - #597c95
             296    8 - #597c95
             516    8 - #597c95
             148   12 - #597c95
             372   16 - #597c95
             188   44 - #597c95
             468   44 - #597c95
              48   48 - #597c95
             264   48 - #597c95
             120   52 - #597c95
             320   52 - #597c95
             572   72 - #597c95
               8   76 - #597c95
             428   76 - #597c95
             500   80 - #597c95
              84   84 - #597c95
             156   84 - #597c95
             228   84 - #597c95
             356   88 - #597c95
             292   96 - #597c95
             456  112 - #597c95
             536  112 - #597c95
             396  116 - #597c95
              52  120 - #597c95
             120  120 - #597c95
             196  124 - #597c95
             328  124 - #597c95
             252  128 - #597c95
             492  140 - #597c95
             572  144 - #597c95
              12  148 - #597c95
             432  152 - #597c95
              84  156 - #597c95
             156  156 - #597c95
             360  160 - #597c95
             292  164 - #597c95
             224  168 - #597c95
              44  188 - #597c95
             120  192 - #597c95
             184  192 - #597c95
             328  196 - #597c95
             260  200 - #597c95
             504  200 - #597c95
             592  212 - #597c95
             428  216 - #597c95
               4  220 - #597c95
             544  224 - #597c95
              84  228 - #597c95
             156  228 - #597c95
             368  228 - #597c95
             296  232 - #597c95
             228  236 - #597c95
              48  264 - #597c95
             124  264 - #597c95
             332  264 - #597c95
             520  264 - #597c95
             192  268 - #597c95
             404  268 - #597c95
             456  272 - #597c95
             580  284 - #597c95
               8  296 - #89959f
              88  296 - #89959f
             156  300 - #89959f
             300  300 - #8a959f
             232  304 - #8a959f
             360  304 - #8a959f
             536  316 - #8a96a0
              48  328 - #8a96a0
             120  336 - #8b97a1
             192  336 - #8b97a1
             452  340 - #8b97a1
             272  344 - #8c97a1
             388  356 - #8c98a2
             584  356 - #8c98a2
             512  364 - #8c98a2
              80  368 - #8c98a2
               8  372 - #8c98a2
             156  372 - #8d99a3
             228  372 - #8d99a3
             316  376 - #8d99a3
             432  392 - #8e9aa4
             552  396 - #8e9aa4
             116  404 - #8e9aa4
             268  404 - #8f9ba5
             480  404 - #8e9aa5
             592  428 - #8f9ba5
             372  432 - #909ca7
             520  436 - #909ca6
              76  440 - #8f9ca6
               4  444 - #8f9ba6
             152  444 - #909ca7
             224  444 - #909da7
             448  444 - #909ca7
             296  448 - #919da7
             404  472 - #919ea8
             340  476 - #929ea9
             488  476 - #919ea8
             572  476 - #919da8
              44  480 - #919da7
             116  480 - #919da8
             188  480 - #919ea8
             260  484 - #929ea9
             528  508 - #929ea9
              80  516 - #929ea9
             152  516 - #929faa
             372  516 - #939faa
             456  516 - #939faa
               8  520 - #929ea9
             224  520 - #939faa
             300  520 - #939faa
             592  524 - #929fa9
             412  548 - #94a0ab
             116  552 - #93a0aa
             188  556 - #94a0ab
             264  556 - #94a0ab
             516  580 - #94a0ab
              76  588 - #94a0ab
             372  588 - #94a1ac
             444  588 - #94a1ac
               4  592 - #93a0ab
             152  592 - #94a1ab
             228  592 - #94a1ac
             300  592 - #94a1ac
             592  592 - #94a0ab
";

const CRATE_RIGHT: &str = r"
               4    4 - #597c95
             124    4 - #597c95
             168    4 - #597c95
             288    4 - #597c95
             332    4 - #597c95
             400    4 - #597c95
             480    4 - #597c95
             592    4 - #597c95
             208   20 - #597c95
              64   36 - #597c95
             360   36 - #597c95
             440   36 - #597c95
             520   36 - #597c95
              20   44 - #597c95
             568   44 - #597c95
             256   48 - #597c95
             304   48 - #597c95
             180   56 - #597c95
             400   64 - #597c95
             136   68 - #597c95
              48   80 - #597c95
             532   80 - #597c95
             464   84 - #597c95
               4   88 - #597c95
             328   88 - #597c95
             580   88 - #597c95
             192  100 - #92372f
             264  100 - #933830
             424  100 - #92362f
              84  104 - #597c95
             300  108 - #933830
             148  112 - #597c95
             228  112 - #92372f
             388  112 - #92372f
             504  116 - #597c95
              16  132 - #597c95
             444  132 - #597c95
             200  136 - #90352d
             300  144 - #90362e
             352  144 - #90352e
             116  148 - #597c95
             236  148 - #8f352d
             544  148 - #597c95
              56  152 - #597c95
             168  152 - #8e342c
             396  160 - #8e342d
             592  168 - #597c95
             324  172 - #8e342d
             476  172 - #597c95
               4  176 - #597c95
             272  176 - #8d342c
             212  188 - #8c332b
             132  192 - #597c95
             356  192 - #8c332c
             432  192 - #725860
              88  196 - #597c95
              44  200 - #597c95
             516  200 - #597c95
             560  200 - #597c95
             304  204 - #8b332b
             392  208 - #8b322b
             172  212 - #8a322a
             252  212 - #8b332b
             352  232 - #89322a
             428  236 - #88312a
             592  236 - #8d98a2
             240  248 - #88312a
             276  248 - #88312a
             388  248 - #88312a
             204  256 - #873129
             320  256 - #88312a
             476  256 - #8d98a2
               4  260 - #8b96a0
             132  260 - #8b97a1
             292  280 - #863129
             544  280 - #8e99a3
             248  284 - #863029
             392  284 - #863029
             212  292 - #853029
              80  296 - #8d99a3
             360  296 - #853029
             424  300 - #896566
             180  308 - #842f28
             308  312 - #853028
             272  316 - #843028
             484  316 - #8f9ba5
             236  324 - #842f28
             380  328 - #832f28
             592  328 - #909ba5
             132  344 - #8f9ca6
             208  352 - #822f28
             264  352 - #822f28
             304  352 - #822f28
             352  352 - #822f28
             408  352 - #822f27
               4  364 - #8f9ba6
              76  368 - #909ca7
             492  376 - #929ea8
             592  396 - #929ea8
             436  404 - #939fa9
             136  408 - #929ea9
             284  412 - #939faa
             380  424 - #93a0aa
               4  432 - #929ea9
             224  436 - #93a0aa
             508  440 - #93a0aa
             444  464 - #94a1ab
             316  468 - #94a1ac
              52  476 - #93a0aa
             548  492 - #94a1ac
             172  512 - #95a1ac
             252  512 - #95a2ac
             104  520 - #95a1ac
             476  520 - #95a2ac
               4  524 - #94a1ab
             392  528 - #95a2ad
             316  532 - #95a2ad
             592  532 - #95a1ac
              56  560 - #95a2ac
             528  560 - #96a2ad
             172  580 - #96a2ad
             400  588 - #96a3ae
             464  588 - #96a3ae
               4  592 - #95a2ac
             108  592 - #96a2ad
             236  592 - #96a3ae
             340  592 - #96a3ae
             592  592 - #96a2ad
";
