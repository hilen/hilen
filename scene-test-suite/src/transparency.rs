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
     240    4 - #597c95
     592    4 - #597c95
     416   44 - #597c95
     132  100 - #597c95
     308  100 - #597c95
     480  136 - #597c95
     592  160 - #597c95
       4  164 - #597c95
     248  212 - #8299a9
     304  212 - #8299a9
     344  212 - #8299a9
     396  212 - #8299a9
     168  240 - #98646a
     280  244 - #d69867
     140  248 - #966369
     184  248 - #9f666b
     204  248 - #ad7274
     372  248 - #8299a9
     436  248 - #4a89b6
     440  248 - #4a89b6
     444  248 - #4a8ab7
     448  248 - #4a8ab7
     452  248 - #4a8ab7
     456  248 - #4a8ab7
     460  248 - #4a8ab6
     312  252 - #d69867
     468  252 - #7cadd0
     480  264 - #7daed1
     124  268 - #c78682
     260  268 - #7abd97
     428  268 - #7dacce
     452  268 - #80afd2
     204  272 - #d58c87
     148  276 - #cf8984
     308  276 - #7dac6c
     332  276 - #7dad6d
     468  276 - #7eafd2
     492  276 - #7cabce
     176  280 - #d7a7a7
     180  280 - #d7a8a7
     172  284 - #d8abab
     176  284 - #dbbebf
     180  284 - #dbc0c1
     184  284 - #d8aeae
     224  284 - #cf8a85
     288  284 - #7daa6b
     448  284 - #c6d2e0
     176  288 - #dbc2c3
     180  288 - #dbc5c6
     184  288 - #d9b3b3
     360  288 - #55aa89
     448  288 - #c1cfdf
     100  292 - #b07e7c
     172  292 - #d6a2a1
     180  292 - #d9b2b2
     184  292 - #d7a6a5
     248  292 - #7abb96
     436  292 - #87aecd
     476  296 - #7dacce
     312  300 - #a8ba87
     316  300 - #a7ba86
     132  304 - #c58581
     308  304 - #adbc8a
     312  304 - #bfc198
     316  304 - #c0c199
     356  304 - #55ab89
     368  304 - #55ab8b
     308  308 - #b2bd8e
     312  308 - #c7c29d
     316  308 - #c9c39f
     324  308 - #9db67f
     112  312 - #ba817e
     156  312 - #c88782
     240  312 - #76946e
     264  312 - #7aa266
     312  312 - #b8bf92
     320  312 - #adbc8a
     316  316 - #a1b681
     356  316 - #55a988
     368  316 - #54a787
     188  320 - #c88782
     220  320 - #757c62
     216  324 - #c68782
     288  324 - #7ca568
     340  324 - #7ec69d
     132  328 - #bc827f
     220  328 - #737b61
     376  328 - #53a284
     108  332 - #aa7c7a
     224  332 - #728164
     360  332 - #54a686
     424  332 - #779eba
     156  336 - #be837f
     220  336 - #717a60
     496  336 - #779ebb
     228  340 - #6f8264
     220  344 - #6c785f
     256  344 - #75b391
     304  344 - #78be98
     376  344 - #519c80
     232  348 - #637e61
     112  352 - #9e7978
     188  352 - #b7807d
     276  352 - #76b693
     332  352 - #78bd97
     224  356 - #62755c
     152  360 - #ab7d7a
     312  364 - #77b894
     128  368 - #9e7978
     464  368 - #7390a6
     368  372 - #74ad8e
     144  376 - #9e7978
     200  376 - #9e7978
     292  376 - #75b090
     168  380 - #9e7978
     344  384 - #74ae8f
     320  388 - #74ad8e
     276  412 - #719984
       4  444 - #bec3c7
     592  444 - #bec3c7
     116  492 - #bec3c7
     476  500 - #bec3c7
     292  524 - #bec3c7
     196  584 - #bec3c7
       4  592 - #bec3c7
     384  592 - #bec3c7
     592  592 - #bec3c7
";

const BEHIND: &str = r"
       4    4 - #597c95
     232    4 - #597c95
     324    4 - #597c95
     592    4 - #597c95
     504   32 - #597c95
     412   36 - #597c95
     120   92 - #597c95
     308   96 - #597c95
     212  124 - #597c95
     484  128 - #597c95
     592  168 - #597c95
       4  176 - #597c95
     300  216 - #489a7f
     316  216 - #45987d
     208  220 - #8ca3b3
     236  220 - #8ca3b3
     260  220 - #8ca3b3
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
     424  236 - #845d64
     436  236 - #825c64
     420  240 - #815c64
     428  240 - #7f5b63
     448  240 - #7d5b63
     424  244 - #7b5a62
     440  244 - #785961
     420  248 - #785962
     432  248 - #755861
     444  248 - #725760
     452  248 - #735760
     148  252 - #799fbb
     200  252 - #7c8e9d
     260  252 - #7d9f8c
     276  252 - #7c9989
     288  252 - #7b9687
     300  252 - #7b9587
     328  252 - #7b9587
     360  252 - #ae8785
     368  252 - #ab8685
     380  252 - #a88584
     392  252 - #a58483
     180  256 - #7491a7
     356  256 - #ad8785
     376  256 - #a68583
     384  256 - #a48483
     316  260 - #7b9587
     364  260 - #a88583
     372  260 - #a58483
     164  264 - #7490a5
     264  264 - #7b9687
     356  264 - #a88583
     256  268 - #7b9788
     360  268 - #a58382
     460  268 - #9f7978
     308  272 - #7b9587
     328  272 - #7b9587
     284  276 - #7b9587
     440  276 - #9f7978
     140  280 - #7490a6
     344  280 - #7a9587
     412  280 - #987f7f
     268  284 - #7b9587
     328  284 - #7a9587
     380  284 - #987f7f
     572  284 - #bfc3c8
      24  288 - #c0c5ca
     292  288 - #7b9587
     316  288 - #7a9587
     336  288 - #7a9587
     252  296 - #dd8243
     356  296 - #987f7f
     280  300 - #dc8141
     300  300 - #dc8040
     428  300 - #9f7978
     456  300 - #9f7978
     264  304 - #dd8142
     328  304 - #dc7f3e
     312  308 - #dc803f
     404  308 - #987f7f
     344  312 - #db7f3d
     372  312 - #987f7f
     244  320 - #dd8244
     268  320 - #dd8142
     296  320 - #dc8040
     320  320 - #dc7f3e
     356  320 - #db7f3c
     440  320 - #9f7978
     392  332 - #987f7f
     420  332 - #9f7978
     360  336 - #adb0b3
     200  340 - #adb0b3
     280  344 - #7e471f
     224  348 - #adb0b3
     252  348 - #7e471f
     376  348 - #adb0b3
     328  356 - #7e471f
     192  364 - #adb0b3
     364  364 - #adb0b3
     400  364 - #adb0b3
     272  368 - #7e471f
     244  376 - #7e471f
     300  380 - #7e471f
     336  392 - #7e471f
     272  396 - #7e471f
       4  400 - #bfc4c9
     244  404 - #7e471f
     592  404 - #bfc3c8
     304  424 - #7e471f
     260  428 - #7e471f
     352  428 - #7e471f
     116  476 - #bfc3c8
     480  476 - #bec3c8
      20  496 - #bfc4c8
     300  548 - #bfc3c8
       4  592 - #bfc3c8
     184  592 - #bfc3c8
     416  592 - #bec3c8
     592  592 - #bec3c7
";
