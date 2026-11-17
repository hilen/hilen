use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Shape3, Vec3},
    refs::Weak,
    scene::{
        Camera, Node, NodeTemplates, Projection, Prop, SceneCreation, SceneManager, SceneSetup, SceneTest,
        SceneTestView, scene,
    },
    ui::{Color, Label, Point, ViewData, ViewFrame, ViewSubviews, WHITE},
    ui_test::{check_colors, checkpoint, inject_touches, set_record_probe_count},
};

/// Three cubes of one size, the nearest 6 units in front of the far one.
const CUBES: [(Vec3, &str); 3] = [
    (Vec3::new(-3.0, 1.0, 4.0), "#d35400"),
    (Vec3::new(0.0, 1.0, 0.0), "#27ae60"),
    (Vec3::new(3.0, 1.0, -2.0), "#2980b9"),
];
const FAR_CUBE: usize = 2;

/// Three cubes of one size at three depths, seen from the front by an
/// orthographic camera, so they come out as three squares of one size on
/// one row. The same camera in perspective draws the near one bigger.
/// Then a view from above and to the side, still orthographic, where the
/// sun casts shadows, a tap that picks the far cube along the parallel
/// ray of its pixel, and a zoom that halves the height the view shows,
/// so the cubes are twice as big and the camera stayed where it was.
#[scene]
#[derive(Default)]
struct OrthographicCamera {
    status:  Weak<Label>,
    touched: Vec<usize>,
}

impl SceneSetup for OrthographicCamera {
    fn setup(&mut self) {
        self.camera = Camera::orthographic(Vec3::new(0.0, 1.0, 20.0), Vec3::new(0.0, 1.0, 0.0), 8.0);
        self.sun.direction = Vec3::new(-0.6, -1.0, -0.35);
        // The default is 1024 on a phone and in the browser.
        self.sun.shadow_map_size = 2048;
        self.make_node::<Prop>(Shape3::Plane(16.0), Vec3::ZERO)
            .set_color(Color::hex("#c8ccd0"))
            .set_roughness(0.9);
        for (index, (at, color)) in CUBES.into_iter().enumerate() {
            let mut cube = self.make_node::<Prop>(Shape3::cube(2.0), at);
            cube.set_color(Color::hex(color)).set_roughness(0.6);
            let mut weak = cube.weak_node();
            cube.on_touch.val(move |_| {
                SceneManager::downcast_scene::<OrthographicCamera>().touched.push(index);
                weak.set_color(WHITE);
            });
        }
    }
}

impl SceneTest for OrthographicCamera {
    fn overlay(mut scene: Weak<Self>, view: Weak<SceneTestView>) {
        let label = view.add_view::<Label>();
        label.set_text_size(15).set_text_color(Color::hex("#ffffff"));
        label
            .set_color(Color::hex("#1c2530"))
            .set_corner_radius(6)
            .set_frame((10, 10, 580, 30));
        scene.status = label;
    }

    fn perform_test(mut scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);
        let say = move |text: &'static str| {
            from_main(move || {
                scene.status.set_text(text);
            });
            wait_for_next_frame();
            wait_for_next_frame();
        };

        say("orthographic from the front, three cubes at three depths, one size");
        let widths = from_main(move || {
            CUBES.map(|(at, _)| {
                let left = scene.view_point(at + Vec3::new(-1.0, 0.0, 1.0)).expect("in front");
                let right = scene.view_point(at + Vec3::new(1.0, 0.0, 1.0)).expect("in front");
                right.x - left.x
            })
        });
        // 8 units over 600 pixels, a cube of 2 is 150 pixels wide.
        ensure!(
            widths.iter().all(|width| (width - 150.0).abs() < 0.01),
            "the cubes are {widths:?} wide"
        );
        check_colors(FRONT)?;

        from_main(move || scene.camera.projection = Projection::Perspective);
        say("the same camera in perspective, the near cube is bigger");
        check_colors(PERSPECTIVE)?;

        from_main(move || {
            scene.camera = Camera::orthographic(Vec3::new(14.0, 16.0, 14.0), Vec3::new(0.0, 1.0, 0.0), 14.0);
            scene.sun.shadows = true;
        });
        say("orthographic from above and to the side, the sun casts shadows");
        // A shadow edge lands elsewhere on the software GPU of the browser
        // lane, so this state is only looked at, see docs/roadmap.md.
        checkpoint("orthographic from above and to the side, the sun casts shadows")?;

        from_main(move || scene.sun.shadows = false);
        let far = from_main(move || scene.view_point(CUBES[FAR_CUBE].0 + Vec3::Y)).expect("in front");
        tap(far);
        say("no shadows, a tap picked the far cube along the ray of its pixel");
        let touched = from_main(move || scene.touched.clone());
        ensure!(touched == [FAR_CUBE], "the tap touched {touched:?}");
        check_colors(PICKED)?;

        let before = from_main(move || scene.camera.position);
        from_main(move || scene.camera.zoom(0.5));
        say("zoomed in, the view shows half the height, the camera stayed");
        let (after, projection) = from_main(move || (scene.camera.position, scene.camera.projection));
        ensure!(after == before, "the zoom moved the camera to {after:?}");
        ensure!(
            projection == Projection::Orthographic { height: 7.0 },
            "the zoom left {projection:?}"
        );
        check_colors(ZOOMED)
    }
}

fn tap(point: Point) {
    inject_touches(format!("{} {} b\n{} {} e", point.x, point.y, point.x, point.y));
    wait_for_next_frame();
}

const FRONT: &str = r"
      56   12 - #1c2530
     508   12 - #1c2530
     556   12 - #1c2530
     588   16 - #1c2530
     104   20 - #f7f7f8
     224   20 - #1c2530
     272   20 - #1c2530
     328   20 - #5d636b
     376   20 - #868b91
     440   20 - #8a8f95
      88   24 - #1c2530
     160   24 - #787d84
     192   24 - #454c55
     240   24 - #ffffff
     296   24 - #ecedee
     304   24 - #ecedee
     324   24 - #f2f2f3
     376   24 - #868b91
     384   24 - #595f67
     412   24 - #1c2530
     108   28 - #313a43
     156   28 - #e0e1e3
     160   28 - #787d84
     176   28 - #3f4650
     180   28 - #b8bbbe
     192   28 - #454c55
     220   28 - #e0e1e3
     236   28 - #9a9ea2
     240   28 - #ffffff
     260   28 - #1c2530
     284   28 - #585e66
     288   28 - #1c2530
     376   28 - #868b91
     384   28 - #595f67
     444   28 - #fefefe
     476   28 - #9ca0a5
     132   32 - #1c2530
      12   36 - #1c2530
     532   36 - #1c2530
     428  152 - #597c95
       4  228 - #99421f
     112  228 - #99421f
     240  228 - #2a7f4a
     528  228 - #2b5f86
     372  264 - #2a7f4a
     460  284 - #2b5f86
     592  284 - #2b5f86
     300  288 - #2a7f4a
      60  300 - #99421f
     228  312 - #2a7f4a
     148  328 - #99421f
     524  332 - #2b5f86
     368  352 - #2a7f4a
       8  372 - #99421f
      84  372 - #99421f
     284  372 - #2a7f4a
     456  372 - #2b5f86
     592  372 - #2b5f86
      60  488 - #597c95
     456  512 - #597c95
     176  520 - #597c95
       4  592 - #597c95
     344  592 - #597c95
     568  592 - #597c95
";

const PERSPECTIVE: &str = r"
      64   12 - #1c2530
     520   12 - #1c2530
     120   20 - #414952
     340   20 - #fbfcfc
     420   20 - #1c2530
     444   20 - #e3e4e5
     448   20 - #1c2530
     120   24 - #414952
     156   24 - #5b626a
     236   24 - #d2d4d6
     360   24 - #ffffff
     400   24 - #767b82
     452   24 - #6d737a
     120   28 - #414952
     156   28 - #5b626a
     176   28 - #1e2732
     200   28 - #1c2530
     208   28 - #acafb4
     220   28 - #1c2530
     236   28 - #d2d4d6
     240   28 - #5d636b
     244   28 - #ffffff
     268   28 - #b8bbbe
     312   28 - #1c2530
     340   28 - #fbfcfc
     344   28 - #8c9196
     364   28 - #8c9196
     384   28 - #f7f7f8
     452   28 - #6d737a
     588   32 - #1c2530
      12   36 - #1c2530
     488   36 - #1c2530
     520  180 - #597c95
     164  268 - #99421e
     220  272 - #99421e
     276  276 - #2a7f4a
     324  276 - #2a7f4a
     376  276 - #2c5f87
     348  280 - #214865
     300  288 - #2a7f4a
     364  288 - #2b5f87
     348  292 - #214865
     396  292 - #2b5f87
     280  300 - #2a7f4a
     348  304 - #214865
     372  304 - #2b5f87
     236  308 - #ad4a20
     164  312 - #99421d
     308  312 - #2a7f4a
     348  312 - #214865
     348  320 - #214865
     396  320 - #2b5f86
     444  328 - #bfc3c7
     200  332 - #99421e
       4  340 - #bfc3c7
     100  340 - #bfc3c7
     492  340 - #bfc3c7
     592  340 - #bfc3c7
       4  512 - #597c95
     372  544 - #597c95
     152  592 - #597c95
     592  592 - #597c95
";

const PICKED: &str = r"
     548   12 - #1c2530
      12   16 - #1c2530
     124   20 - #464d56
     240   20 - #5f656d
     260   20 - #b8bbbe
     396   20 - #ced0d2
     504   20 - #f5f6f6
     144   24 - #ffffff
     272   24 - #ffffff
     304   24 - #595f67
     456   24 - #6d737a
     100   28 - #caccce
     124   28 - #464d56
     160   28 - #1c2530
     192   28 - #1c2530
     228   28 - #e9eaeb
     292   28 - #e8e9ea
     304   28 - #595f67
     308   28 - #1c2530
     372   28 - #1c2530
     376   28 - #6f757b
     400   28 - #babcc0
     416   28 - #ced0d2
     456   28 - #6d737a
     476   28 - #888c92
     212   32 - #1c2530
     588   32 - #1c2530
      56   36 - #1c2530
     180  156 - #bec2c6
       4  168 - #597c95
     300  232 - #32a660
     456  252 - #ececec
      68  260 - #c85520
     112  264 - #c85520
     240  268 - #287e49
     320  268 - #32a660
     400  280 - #ececec
     508  284 - #ececec
     288  292 - #32a660
      56  300 - #c85520
     432  304 - #ececec
     240  308 - #287e49
      28  316 - #98411c
     336  316 - #2c8f52
      96  332 - #ad4a1d
     260  344 - #287e49
      32  352 - #98411c
      68  352 - #98411c
     296  364 - #287e49
      84  384 - #98411c
     592  452 - #597c95
     408  492 - #bec2c6
     128  520 - #bec2c6
       4  592 - #597c95
     248  592 - #bec2c6
     528  592 - #597c95
";

const ZOOMED: &str = r"
      44   12 - #1c2530
      96   12 - #1c2530
     512   12 - #1c2530
      12   16 - #1c2530
     588   16 - #1c2530
     148   20 - #f8f8f9
     240   20 - #caccce
     280   20 - #ffffff
     296   20 - #b1b4b8
     324   20 - #1c2530
     356   20 - #1c2530
     380   20 - #1c2530
     156   24 - #bbbec1
     296   24 - #b1b4b8
     444   24 - #ffffff
     128   28 - #3e454f
     132   28 - #ffffff
     156   28 - #bbbec1
     160   28 - #464d56
     164   28 - #ffffff
     184   28 - #f7f7f8
     204   28 - #1c2530
     240   28 - #caccce
     256   28 - #1c2530
     284   28 - #5f656d
     296   28 - #b1b4b8
     300   28 - #777c83
     308   28 - #1c2530
     336   28 - #1c2530
     368   28 - #414952
     392   28 - #f7f7f8
     428   28 - #dddfe0
     432   28 - #aaadb1
     480   28 - #1c2530
      68   36 - #1c2530
     536   36 - #1c2530
     288  168 - #32a660
     592  204 - #ececec
     400  220 - #32a660
     328  228 - #32a660
     180  236 - #287e49
     544  248 - #ececec
     512  252 - #ececec
     592  256 - #ececec
     484  268 - #ececec
     268  288 - #287e49
     528  292 - #ececec
     352  296 - #2c8f52
       4  300 - #bec2c6
     204  300 - #287e49
     576  304 - #ececec
     252  344 - #287e49
     420  348 - #2c8f52
     180  360 - #287e49
     300  388 - #2c8f52
     368  396 - #2c8f52
     236  400 - #287e49
     296  436 - #287e49
      56  448 - #bec2c6
     532  452 - #bec2c6
     408  544 - #bec2c6
       4  592 - #bec2c6
     224  592 - #bec2c6
     592  592 - #bec2c6
";
