use std::f32::consts::PI;

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

/// Frames of the half turn between the checks.
const TURN_FRAMES: usize = 30;

/// Three half transparent balls that overlap on screen, a glass pane in
/// front of an orange box, on a plane. Where two balls overlap the
/// nearer one has to tint the farther one and not hide it, the box has
/// to show through the pane, and after the half turn the same pixels
/// have to blend in the other order, since the list is sorted back to
/// front every frame.
#[scene]
#[derive(Default)]
struct Transparency {}

impl SceneSetup for Transparency {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 3.0, 7.0),
            target: Vec3::new(0.0, 0.8, 0.0),
            ..Camera::default()
        };

        self.make_node::<Prop>(Shape3::Plane(14.0), Vec3::ZERO)
            .set_color(Color::hex("#c8cdd2"))
            .set_roughness(0.9);
        self.make_node::<Prop>(Shape3::cube(1.2), Vec3::new(0.0, 0.6, -1.5))
            .set_color(Color::hex("#e67e22"))
            .set_roughness(0.6);
        self.make_node::<Prop>(Shape3::cuboid(3.0, 2.0, 0.1), Vec3::new(0.0, 1.0, -0.5))
            .set_color(Color::hex("#ffffff").with_alpha(0.35))
            .set_roughness(0.1);

        for (color, x, z) in [
            ("#e74c3c", -1.6, 0.8),
            ("#2ecc71", 0.0, 1.6),
            ("#3498db", 1.6, 0.8),
        ] {
            self.make_node::<Prop>(Shape3::Ball(0.9), Vec3::new(x, 0.9, z))
                .set_color(Color::hex(color).with_alpha(0.5))
                .set_roughness(0.3);
        }
    }
}

impl SceneTest for Transparency {
    fn perform_test(mut scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);

        wait_for_next_frame();
        check_colors(FRONT)?;

        let frames: f32 = TURN_FRAMES.lossy_convert();
        for _ in 0..TURN_FRAMES {
            from_main(move || scene.camera.orbit(PI / frames, 0.0));
            wait_for_next_frame();
        }

        capture_screenshot()?;
        check_colors(BEHIND)
    }
}

const FRONT: &str = r"
       4    4 - #597c95
     248    4 - #597c95
     592    4 - #597c95
     420   44 - #597c95
     136   96 - #597c95
     316  100 - #597c95
     484  136 - #597c95
       4  160 - #597c95
     592  160 - #597c95
     228  212 - #8299a9
     280  212 - #8299a9
     368  212 - #8299a9
     168  240 - #98646a
     148  244 - #966369
     288  244 - #d69867
     320  244 - #d69867
     192  248 - #9e666b
     356  248 - #8299a9
     436  248 - #4a89b6
     440  248 - #4a89b6
     444  248 - #4a8ab7
     448  248 - #4a8ab7
     452  248 - #4a8ab7
     456  248 - #4a8ab7
     460  248 - #4a8ab6
     216  256 - #cd8984
     392  256 - #7ca4c2
     472  256 - #7daed0
     144  264 - #ce8984
     264  264 - #7aa065
     436  268 - #7fadcf
     452  268 - #80afd1
     484  268 - #7cadd0
     112  272 - #be837f
     300  272 - #7dac6c
     204  276 - #d48c87
     332  276 - #7dae6d
     468  276 - #7eafd1
     176  280 - #d7a7a7
     180  280 - #d7a8a7
     172  284 - #d8abab
     176  284 - #dbbebf
     180  284 - #dbc0c1
     184  284 - #d8aeae
     448  284 - #c6d2e0
     176  288 - #dbc2c3
     180  288 - #dbc5c6
     184  288 - #d9b3b3
     236  288 - #7a916d
     276  288 - #7ca769
     360  288 - #55aa89
     448  288 - #c1cfdf
     492  288 - #7cacce
     172  292 - #d6a2a1
     180  292 - #d9b2b2
     184  292 - #d7a6a5
     256  292 - #7bbe98
     436  292 - #87aecd
     212  296 - #d08a85
     472  296 - #7dacce
     100  300 - #b17f7c
     312  300 - #a8ba87
     316  300 - #a7ba86
     136  304 - #c78682
     308  304 - #adbc8b
     312  304 - #bfc198
     316  304 - #c0c199
     356  304 - #55ab89
     284  308 - #7da769
     308  308 - #b2bd8e
     312  308 - #c7c29e
     316  308 - #c9c39f
     324  308 - #9db67f
     160  312 - #c98783
     264  312 - #7aa266
     312  312 - #b8bf92
     320  312 - #adbc8b
     368  312 - #54a989
     240  316 - #75936e
     316  316 - #a1b681
     356  316 - #55a988
     184  324 - #c68682
     216  324 - #c68782
     376  324 - #53a486
     288  328 - #7ca467
     360  328 - #54a787
     108  332 - #aa7c7a
     160  336 - #bf837f
     424  336 - #779db8
     496  336 - #779ebb
     228  340 - #6f8264
     260  340 - #78b794
     372  340 - #53a183
     136  344 - #b47f7d
     220  344 - #6c785f
     228  344 - #6a7e62
     232  344 - #698163
     336  344 - #79bf99
     192  348 - #b9817e
     224  348 - #69765e
     232  348 - #637e61
     112  352 - #9e7878
     228  352 - #63775d
     308  352 - #78bb97
     372  352 - #509b80
     172  356 - #b37f7c
     224  356 - #62745c
     280  360 - #76b492
     152  364 - #a67b79
     128  368 - #9e7878
     344  368 - #76b694
     368  368 - #75b090
     464  368 - #7390a6
     200  376 - #9e7878
     300  376 - #75b191
     168  380 - #9e7878
     324  380 - #75b191
     248  396 - #719984
     288  416 - #98aea6
       4  444 - #bec2c7
     592  444 - #bec2c7
     116  496 - #bec2c7
     472  496 - #bec2c7
     296  528 - #bec2c7
     196  588 - #bec2c7
       4  592 - #bec2c7
     388  592 - #bec2c7
     592  592 - #bec2c7
";

const BEHIND: &str = r"
       4    4 - #597c95
     232    4 - #597c95
     592    4 - #597c95
     504   32 - #597c95
     412   36 - #597c95
     120   88 - #597c95
     308   96 - #597c95
     212  120 - #597c95
     484  128 - #597c95
       4  168 - #597c95
     592  168 - #597c95
     300  216 - #489a7f
     316  216 - #45987d
     228  220 - #8ca3b3
     256  220 - #8ca3b3
     352  220 - #8ca3b3
     404  220 - #b69498
     284  224 - #9babaf
     288  224 - #92a9a8
     184  228 - #8fa1b1
     284  228 - #85a39e
     180  232 - #99a7b3
     184  232 - #96a5b2
     384  232 - #9c9299
     388  232 - #9d959c
     384  236 - #9fa3ab
     388  236 - #9e9aa2
     428  236 - #835d64
     440  236 - #825c64
     420  240 - #815c64
     432  240 - #7e5b63
     448  240 - #7d5b63
     424  244 - #7b5a62
     436  244 - #785962
     420  248 - #785962
     432  248 - #755861
     444  248 - #725760
     452  248 - #735760
     144  252 - #79a1be
     184  252 - #7d919f
     200  252 - #7c8e9d
     268  252 - #7c9c8b
     292  252 - #7b9587
     320  252 - #7b9587
     360  252 - #ae8785
     372  252 - #aa8684
     392  252 - #a48483
     252  256 - #7da18e
     348  256 - #7a9587
     368  256 - #a88583
     376  256 - #a68583
     384  256 - #a48482
     456  256 - #9f7978
     172  260 - #7490a6
     308  260 - #7b9587
     356  260 - #aa8583
     372  260 - #a58482
     280  264 - #7b9587
     328  264 - #7a9587
     364  264 - #a58482
     156  268 - #7490a5
     260  268 - #7b9587
     356  268 - #a68482
     404  268 - #987f7f
     312  272 - #7a9587
     300  276 - #7a9587
     340  276 - #7a9587
     140  280 - #7490a6
     268  280 - #7b9587
     284  280 - #7a9587
     424  280 - #9f7978
      20  284 - #c0c5c9
     316  284 - #7a9587
     392  284 - #987f7f
     460  284 - #9f7978
     576  284 - #bfc3c8
     252  292 - #dd8243
     288  292 - #dc8041
     340  292 - #db7f3d
     368  292 - #987f7f
     304  296 - #dc803f
     272  300 - #dc8142
     324  308 - #db7f3e
     412  308 - #987f7f
     448  308 - #9f7978
     256  312 - #dd8143
     288  312 - #dc8040
     348  312 - #db7f3c
     392  312 - #987f7f
     304  316 - #dc803f
     372  316 - #987f7f
     276  324 - #c57339
     240  328 - #8a6144
     332  328 - #7e471f
     428  328 - #9f7978
     240  332 - #967c69
     208  336 - #adb0b3
     404  336 - #a39899
     240  340 - #a1968e
     360  340 - #adb0b3
     308  348 - #7e471f
     384  348 - #adb0b3
     268  356 - #7e471f
     192  364 - #adb0b3
     224  364 - #adb0b3
     368  364 - #adb0b3
     400  364 - #adb0b3
     332  368 - #7e471f
     296  376 - #7e471f
     252  384 - #7e471f
     320  396 - #7e471f
     592  396 - #bec3c7
       4  400 - #bfc4c8
     288  408 - #7e471f
     356  408 - #8e6649
     244  412 - #7e471f
     356  412 - #9f8574
     356  416 - #9f8574
     356  420 - #aea49e
     264  428 - #7e471f
     312  428 - #7e471f
     116  476 - #bfc3c8
     480  476 - #bec3c7
     296  544 - #bec3c7
       4  592 - #bfc3c8
     188  592 - #bec3c7
     408  592 - #bec3c7
     592  592 - #bec3c7
";
