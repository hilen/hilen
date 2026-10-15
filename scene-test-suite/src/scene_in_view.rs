use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Shape3, Vec3},
    refs::Weak,
    scene::{
        Camera, Node, NodeTemplates, Prop, SceneCreation, SceneManager, SceneSetup, SceneTest, SceneTestView,
        SceneView, scene,
    },
    ui::{Color, Label, Point, Rect, ViewData, ViewFrame, ViewSubviews, WHITE},
    ui_test::{check_colors, inject_scroll, inject_touches, set_record_probe_count},
};

const BALL_AT: Vec3 = Vec3::new(-1.1, 0.8, 0.0);
const CRATE_AT: Vec3 = Vec3::new(1.4, 0.6, 0.0);
/// The scene view, in points of the canvas, taller than wide so a ball
/// that stays round shows the camera took the aspect of the frame.
const FRAME: (f32, f32, f32, f32) = (250.0, 60.0, 340.0, 400.0);
/// The same view after it got narrower.
const NARROW: (f32, f32, f32, f32) = (250.0, 60.0, 200.0, 400.0);

/// A scene inside a `SceneView` that shares the canvas with other views:
/// a panel next to it, a label over it and a status line under it. The
/// view is taller than wide. A tap in the view picks the ball through
/// the view's own pixels, a tap on the panel does not reach the scene, a
/// drag turns the camera and the wheel zooms it. Then the view gets
/// narrower, the picture follows with the new aspect and the pixels the
/// view left show the canvas again, and at the end the view is removed
/// and the scene stops with it.
#[scene]
#[derive(Default)]
struct SceneInView {
    status:  Weak<Label>,
    touched: usize,
    taps:    usize,
}

impl SceneSetup for SceneInView {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 3.0, 7.0),
            target: Vec3::new(0.0, 0.6, 0.0),
            ..Camera::default()
        };
        self.make_node::<Prop>(Shape3::Plane(12.0), Vec3::ZERO)
            .set_color(Color::hex("#c8ccd0"))
            .set_roughness(0.9);

        let mut ball = self.make_node::<Prop>(Shape3::Ball(0.8), BALL_AT);
        ball.set_color(Color::hex("#2980b9"));
        let mut weak = ball.weak_node();
        ball.on_touch.val(move |_| {
            SceneManager::downcast_scene::<SceneInView>().touched += 1;
            weak.set_color(WHITE);
        });

        self.make_node::<Prop>(Shape3::cube(1.2), CRATE_AT)
            .set_color(Color::hex("#d35400"));

        self.on_tap.val(|_| SceneManager::downcast_scene::<SceneInView>().taps += 1);
    }
}

fn caption(view: Weak<SceneTestView>, text: &str, frame: (f32, f32, f32, f32)) -> Weak<Label> {
    let label = view.add_view::<Label>();
    label.set_text(text).set_text_size(15).set_text_color(Color::hex("#ffffff"));
    label.set_color(Color::hex("#1c2530")).set_corner_radius(6).set_frame(frame);
    label
}

impl SceneTest for SceneInView {
    fn scene_view(view: Weak<SceneTestView>) -> Option<Weak<SceneView>> {
        caption(view, "a panel next to the scene", (10.0, 60.0, 220.0, 400.0));
        let mut scene_view = view.add_view::<SceneView>();
        scene_view.set_frame(FRAME);
        scene_view.set_background(Color::hex("#22303c"));
        Some(scene_view)
    }

    fn overlay(mut scene: Weak<Self>, view: Weak<SceneTestView>) {
        // A later sibling draws behind the image inside the scene view,
        // raised it draws over it.
        let badge = caption(view, "a label over the scene", (262.0, 72.0, 180.0, 30.0));
        badge.bump_z_position(0.000_1);
        scene.status = caption(view, "", (10.0, 480.0, 580.0, 40.0));
    }

    fn perform_test(scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);
        let status = from_main(move || scene.status);
        let say = move |text: &'static str| {
            from_main(move || {
                status.set_text(text);
            });
            wait_for_next_frame();
            wait_for_next_frame();
        };

        say("the scene in its view, a panel next to it, a label over it");
        let ball = from_main(move || scene.view_point(BALL_AT)).expect("the ball is in front of the camera");
        ensure!(
            Rect::from(FRAME).contains(ball),
            "the ball lands at {ball:?}, outside the view"
        );
        check_colors(IN_VIEW)?;

        tap(ball);
        say("a tap in the view picked the ball, it turned white");
        let counts = from_main(move || (scene.touched, scene.taps));
        ensure!(counts == (1, 1), "touched and taps after the tap: {counts:?}");
        check_colors(BALL_PICKED)?;

        tap(Point::new(120.0, 300.0));
        let counts = from_main(move || (scene.touched, scene.taps));
        ensure!(
            counts == (1, 1),
            "a tap on the panel reached the scene: {counts:?}"
        );

        let before = from_main(move || scene.camera.position);
        inject_touches("420 300 b\n430 300 m\n450 300 m\n470 300 m\n490 300 m\n510 300 m\n520 300 e");
        wait_for_next_frame();
        let after = from_main(move || scene.camera.position);
        let counts = from_main(move || (scene.touched, scene.taps));
        ensure!(
            after.distance(before) > 1.0,
            "the drag left the camera at {after:?}"
        );
        ensure!(counts == (1, 1), "a drag is no tap, got {counts:?}");
        say("a drag in the view turned the camera around the scene");
        check_colors(TURNED)?;

        let distance = from_main(move || scene.camera.position.distance(scene.camera.target));
        inject_touches("420 300 m");
        inject_scroll(100);
        let closer = from_main(move || scene.camera.position.distance(scene.camera.target));
        ensure!(
            closer < distance * 0.9,
            "the wheel zoomed from {distance} to {closer}"
        );
        say("the wheel in the view zoomed the camera in");
        check_colors(ZOOMED)?;

        from_main(move || {
            let host = SceneManager::host().expect("the scene runs in a view");
            host.set_frame(NARROW);
        });
        say("the view got narrower, the picture follows its new aspect");
        let ball = from_main(move || scene.view_point(BALL_AT)).expect("the ball is in front of the camera");
        ensure!(
            Rect::from(NARROW).contains(ball),
            "the ball lands at {ball:?}, outside the narrow view"
        );
        check_colors(NARROWED)?;

        from_main(move || {
            let mut host = SceneManager::host().expect("the scene runs in a view");
            host.remove_from_superview();
        });
        say("the view is gone and its scene stopped with it");
        ensure!(
            from_main(move || !scene.is_ok() && SceneManager::host().is_none()),
            "the scene runs on after its view was removed"
        );
        check_colors(GONE)
    }
}

fn tap(point: Point) {
    inject_touches(format!("{} {} b\n{} {} e", point.x, point.y, point.x, point.y));
    wait_for_next_frame();
}

const IN_VIEW: &str = r"
     200    4 - #597c95
     592    4 - #597c95
      12   64 - #1c2530
     292   84 - #404750
     304   84 - #888c92
     292   88 - #404750
     316   88 - #92969b
     352   88 - #fefefe
     376   88 - #92969b
     396   88 - #1c2530
     420   88 - #92969b
     148  116 - #1c2530
     592  132 - #597c95
     348  220 - #3075a7
     508  228 - #c7541e
     456  232 - #733219
     364  236 - #4f8bbd
     372  236 - #6d9dcb
     380  236 - #508dc0
     368  240 - #5e92c1
     336  244 - #2a6691
     376  244 - #508abb
     404  252 - #2b6894
      84  256 - #4a515a
     460  256 - #733219
      52  260 - #1c2530
      84  260 - #4a515a
     124  260 - #596068
     140  260 - #ffffff
     176  260 - #1c2530
     188  260 - #ffffff
     332  260 - #214865
     492  264 - #ac491d
     360  268 - #2a638d
     384  268 - #2b6792
     456  268 - #733219
     344  272 - #224967
     456  280 - #733219
     368  288 - #214865
     484  296 - #ac491d
     516  296 - #ac491d
     256  308 - #bec2c5
     592  340 - #597c95
     156  380 - #1c2530
     316  380 - #bec1c5
     484  400 - #bec1c5
      12  412 - #1c2530
     572  428 - #bec1c5
     408  496 - #434b54
     144  500 - #a1a4a9
     172  500 - #a1a4a9
     188  500 - #a1a4a9
     212  500 - #6d737a
     216  500 - #5c636a
     240  500 - #ffffff
     308  500 - #a1a4a9
     320  500 - #868b91
     344  500 - #5c636a
     360  500 - #1c2530
     436  500 - #1c2530
     280  504 - #b5b8bc
     412  504 - #f8f8f9
     584  516 - #1c2530
       4  592 - #597c95
";

const BALL_PICKED: &str = r"
     200    4 - #597c95
     592    4 - #597c95
      12   64 - #1c2530
     292   84 - #404750
     304   84 - #888c92
     316   88 - #92969b
     348   88 - #92969b
     376   88 - #92969b
     396   88 - #1c2530
     420   88 - #92969b
     148  112 - #1c2530
     588  136 - #22303c
     356  216 - #eaeaea
     384  220 - #f0f0f0
     496  228 - #c7541e
     340  232 - #dadada
     456  236 - #733219
     400  240 - #e7e7e7
     368  244 - #efefef
     456  248 - #733219
     332  252 - #a5a5a5
     520  252 - #ac491d
      84  256 - #4a515a
     144  256 - #b3b6ba
      52  260 - #1c2530
     124  260 - #596068
     172  260 - #fefefe
     188  260 - #ffffff
     460  260 - #733219
     336  268 - #8b8b8b
     456  268 - #733219
     492  268 - #ac491d
     352  272 - #a7a7a7
     364  276 - #a9a9a9
     384  276 - #aaaaaa
     348  280 - #8b8b8b
     456  280 - #733219
     356  284 - #8b8b8b
     368  288 - #8b8b8b
     388  296 - #bec2c5
     484  296 - #ac491d
     516  296 - #ac491d
     592  316 - #597c95
     256  372 - #bec1c5
     128  384 - #1c2530
     356  392 - #bec1c5
     460  392 - #bec1c5
      12  428 - #1c2530
     580  456 - #bec1c5
     348  496 - #4a515a
     168  500 - #ffffff
     184  500 - #e9eaeb
     200  500 - #f8f8f9
     232  500 - #bbbec1
     272  500 - #1c2530
     288  500 - #a1a4a9
     308  500 - #1c2530
     360  500 - #404750
     376  500 - #1c2530
     384  500 - #3c444d
     440  500 - #bbbec1
     444  500 - #b4b7bb
      28  592 - #597c95
     548  592 - #597c95
";

const TURNED: &str = r"
       4    4 - #597c95
     204    4 - #597c95
     588   60 - #22303c
     292   84 - #404750
     292   88 - #404750
     316   88 - #92969b
     348   88 - #92969b
     376   88 - #92969b
     404   88 - #92969b
     420   88 - #92969b
     132  100 - #1c2530
      12  140 - #1c2530
     480  220 - #c7551f
     384  224 - #f0f0f0
     448  224 - #c7541f
     356  236 - #d9d9d9
     464  236 - #733219
     504  236 - #ac491d
     340  244 - #a8a8a8
     432  244 - #733219
     588  244 - #22303c
     404  248 - #efefef
     456  252 - #733219
     484  252 - #ac491d
      84  256 - #4a515a
     144  256 - #b3b6ba
     376  256 - #dedede
      52  260 - #1c2530
      84  260 - #4a515a
     124  260 - #596068
     140  260 - #ffffff
     176  260 - #1c2530
     188  260 - #ffffff
     336  260 - #8b8b8b
     392  268 - #d7d7d7
     432  268 - #733219
     500  272 - #ac491c
     360  276 - #9b9b9b
     464  280 - #733219
     340  284 - #8b8b8b
     376  284 - #9f9f9f
     404  284 - #adadad
     356  296 - #8b8b8b
     392  296 - #8b8b8b
     588  348 - #bec2c5
     260  368 - #bec1c5
     132  380 - #1c2530
     496  388 - #bec2c5
     360  396 - #bec1c5
       4  400 - #597c95
     588  452 - #bec2c5
     136  500 - #1c2530
     184  500 - #1c2530
     220  500 - #a1a4a9
     264  500 - #ffffff
     288  500 - #ffffff
     316  500 - #1c2530
     344  500 - #a1a4a9
     388  500 - #ffffff
     432  500 - #a1a4a9
     464  500 - #414952
     476  500 - #a1a4a9
       4  592 - #597c95
     584  592 - #597c95
";

const ZOOMED: &str = r"
       4    4 - #597c95
     204    4 - #597c95
     492    4 - #597c95
     588   60 - #22303c
     292   84 - #404750
     292   88 - #404750
     316   88 - #92969b
     332   88 - #1c2530
     348   88 - #92969b
     376   88 - #92969b
     396   88 - #1c2530
     420   88 - #92969b
     132   96 - #1c2530
      12  140 - #1c2530
     448  208 - #bec2c6
     348  212 - #e5e5e5
     496  216 - #c75520
     396  224 - #f1f1f1
     436  232 - #733219
     472  240 - #733219
     340  244 - #d1d1d1
      84  256 - #4a515a
     144  256 - #b3b6ba
     492  256 - #ac491d
      52  260 - #1c2530
      84  260 - #4a515a
     124  260 - #596068
     172  260 - #fefefe
     188  260 - #ffffff
     436  260 - #733219
     368  268 - #d1d1d1
     312  276 - #8b8b8b
     452  276 - #733219
     476  276 - #9e431b
     504  280 - #ac491c
     340  284 - #8c8c8c
     476  288 - #ac491c
     400  292 - #a6a6a6
     356  312 - #8b8b8b
     252  376 - #bec1c5
     128  380 - #1c2530
     388  384 - #bec1c5
      12  420 - #1c2530
     484  424 - #bec2c5
     588  428 - #bec2c5
     160  496 - #414952
     192  496 - #f7f7f8
     216  496 - #c8cacd
     244  496 - #e1e2e3
     156  500 - #a0a4a8
     172  500 - #a1a4a9
     216  500 - #c8cacd
     224  500 - #ffffff
     272  500 - #d2d4d6
     308  500 - #1c2530
     356  500 - #e4e5e7
     384  500 - #1c2530
     412  500 - #a1a4a9
     300  504 - #fefefe
       4  592 - #597c95
     552  592 - #597c95
";

const NARROWED: &str = r"
       4    4 - #597c95
     592    4 - #597c95
     292   84 - #404750
     292   88 - #404750
     316   88 - #92969b
     352   88 - #fefefe
     376   88 - #92969b
     420   88 - #92969b
     132   96 - #1c2530
      12  140 - #1c2530
     592  176 - #597c95
     376  208 - #bec2c6
     268  216 - #d8d8d8
     316  216 - #f1f1f1
     428  216 - #c75520
     284  244 - #e1e1e1
     252  248 - #acacac
     404  248 - #733219
      84  256 - #4a515a
     144  256 - #b3b6ba
     312  256 - #e8e8e8
      52  260 - #1c2530
      84  260 - #4a515a
     124  260 - #596068
     176  260 - #1c2530
     188  260 - #ffffff
     448  264 - #ac491c
     260  268 - #9f9f9f
     368  268 - #733219
     420  280 - #ac491c
     288  284 - #aaaaaa
     392  284 - #733219
     328  296 - #969696
     260  300 - #8b8b8b
     296  312 - #8b8b8b
      12  344 - #1c2530
     448  356 - #bec2c5
     132  380 - #1c2530
     356  396 - #bec2c5
     592  396 - #597c95
     252  400 - #bec1c5
     448  432 - #bec2c5
     352  496 - #83888e
     116  500 - #9da1a6
     148  500 - #61676f
     176  500 - #1c2530
     192  500 - #596068
     204  500 - #ffffff
     248  500 - #a1a4a9
     264  500 - #72787f
     280  500 - #a1a4a9
     312  500 - #1c2530
     340  500 - #c7c9cc
     356  500 - #c8cacd
     396  500 - #9da1a6
     416  500 - #ffffff
     484  500 - #596068
";

const GONE: &str = r"
       4    4 - #597c95
     204    4 - #597c95
     592    4 - #597c95
      56   68 - #1c2530
     312   80 - #1c2530
     208   84 - #1c2530
     292   84 - #404750
     304   84 - #888c92
     292   88 - #404750
     316   88 - #92969b
     348   88 - #92969b
     352   88 - #fefefe
     376   88 - #92969b
     404   88 - #92969b
     420   88 - #92969b
     336   92 - #1c2530
     360   92 - #1c2530
     412   92 - #1c2530
     132  100 - #1c2530
      12  140 - #1c2530
     556  144 - #597c95
     216  160 - #1c2530
      88  172 - #1c2530
     456  200 - #597c95
     316  204 - #597c95
      84  256 - #4a515a
     144  256 - #b3b6ba
      52  260 - #1c2530
      84  260 - #4a515a
     124  260 - #596068
     140  260 - #ffffff
     172  260 - #fefefe
     176  260 - #1c2530
     180  260 - #ffffff
     188  260 - #ffffff
     592  284 - #597c95
     384  292 - #597c95
      60  336 - #1c2530
     492  372 - #597c95
     228  376 - #1c2530
     120  384 - #1c2530
     336  392 - #597c95
      12  396 - #1c2530
     588  484 - #1c2530
      48  496 - #1c2530
     156  496 - #b8bbbe
     404  496 - #e2e3e4
     152  500 - #596068
     228  500 - #1c2530
     252  500 - #a1a4a9
     280  500 - #1c2530
     296  500 - #1c2530
     320  500 - #1c2530
     328  500 - #a1a4a9
     344  500 - #a1a4a9
     364  500 - #1c2530
     404  500 - #ffffff
     424  500 - #333b45
     436  500 - #fefefe
     444  500 - #a5a9ad
     448  500 - #9da1a6
     376  508 - #1c2530
       4  592 - #597c95
     528  592 - #597c95
";
