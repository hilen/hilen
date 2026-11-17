use std::f32::consts::PI;

use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Quat, Shape3, Vec3},
    refs::Weak,
    scene::{
        Camera, HandleDrag, HandleMode, HandleSnap, Line3, Node, NodeTemplates, Prop, SceneCreation,
        SceneManager, SceneSetup, SceneTest, SceneTestView, SceneView, TransformHandle, scene,
    },
    ui::{Color, Label, Point, Size, ViewData, ViewFrame, ViewSubviews},
    ui_test::{check_colors, inject_touches, set_record_probe_count},
};

/// The scene view, under the status line.
const VIEW: (f32, f32, f32, f32) = (0.0, 50.0, 600.0, 550.0);
const CRATE_AT: Vec3 = Vec3::new(0.0, 0.8, 0.0);
/// A turn snaps to 15 degrees.
const TURN_STEP: f32 = PI / 12.0;

/// Where the crate was when a drag began, what each amount is counted
/// from.
#[derive(Clone, Copy, Debug, Default)]
struct Start {
    position: Vec3,
    rotation: Quat,
    scale:    f32,
}

/// A crate with a `TransformHandle` on it in a `SceneView`: 3 arrows, 3
/// rings and 3 boxes in the axis colors, drawn over the crate at one size
/// on the screen. The app moves, turns and scales the crate from what the
/// handle tells it. A drag on the x arrow moves it along x, a drag round
/// the y ring turns it about y in steps of 15 degrees, a drag out on the
/// z box scales it, and each drag ends once. The camera pulled back
/// leaves the handle as big as it was, and a drag off the handle turns
/// the camera and leaves the crate alone.
#[scene]
#[derive(Default)]
struct TransformHandleTest {
    crate_box: Weak<Prop>,
    start:     Option<Start>,
    drags:     Vec<HandleDrag>,
    status:    Weak<Label>,
}

impl SceneSetup for TransformHandleTest {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(3.5, 4.0, 6.5),
            target: Vec3::new(0.0, 0.8, 0.0),
            ..Camera::default()
        };
        self.make_node::<Prop>(Shape3::Plane(12.0), Vec3::ZERO)
            .set_color(Color::hex("#e4e6e8"))
            .set_roughness(0.9);
        self.lines = Line3::grid(10.0, 1.0, Color::hex("#b4b9be"));
        self.crate_box = self.make_node::<Prop>(Shape3::cube(1.2), CRATE_AT);
        self.crate_box.set_color(Color::hex("#8e6bb8")).set_roughness(0.6);

        let mut handle = TransformHandle::on(self.crate_box.weak_node());
        handle.snap = HandleSnap {
            turns: Some(TURN_STEP),
            ..HandleSnap::default()
        };
        handle
            .on_drag
            .val(|drag| SceneManager::downcast_scene::<TransformHandleTest>().apply(drag));
        self.handle = Some(handle);
    }
}

impl TransformHandleTest {
    /// What an app does with a drag: the change from where the crate was
    /// when the drag began.
    fn apply(&mut self, drag: HandleDrag) {
        let mut crate_box = self.crate_box;
        let start = *self.start.get_or_insert(Start {
            position: crate_box.position(),
            rotation: crate_box.rotation(),
            scale:    crate_box.world_scale(),
        });
        match drag.part.mode {
            HandleMode::Move => {
                crate_box.set_position(start.position + drag.axis * drag.amount);
            }
            HandleMode::Turn => {
                crate_box.set_rotation(Quat::from_axis_angle(drag.axis, drag.amount) * start.rotation);
            }
            // A node has 1 scale, the app grows it whole.
            HandleMode::Scale => {
                crate_box.set_scale(start.scale * drag.amount);
            }
        }
        if drag.ended {
            self.start = None;
        }
        self.drags.push(drag);
    }

    /// The world length of an arrow of the handle.
    fn arrow(&self) -> f32 {
        let size = self.handle.as_ref().expect("the handle is on").size;
        self.camera
            .units_per_pixel(self.crate_box.position(), Size::new(VIEW.2, VIEW.3))
            * size
    }

    /// A point of the handle in arrow lengths from its center, on the
    /// screen.
    fn handle_point(&self, offset: Vec3) -> Point {
        let world = self.crate_box.position() + offset * self.arrow();
        self.view_point(world).expect("the handle is in front of the camera")
    }
}

impl SceneTest for TransformHandleTest {
    fn scene_view(view: Weak<SceneTestView>) -> Option<Weak<SceneView>> {
        let scene_view = view.add_view::<SceneView>();
        scene_view.set_frame(VIEW);
        Some(scene_view)
    }

    fn overlay(mut scene: Weak<Self>, view: Weak<SceneTestView>) {
        let label = view.add_view::<Label>();
        label.set_text_size(15).set_text_color(Color::hex("#ffffff"));
        label
            .set_color(Color::hex("#1c2530"))
            .set_corner_radius(6)
            .set_frame((10, 10, 580, 30));
        scene.status = label;
    }

    fn perform_test(scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);
        let say = move |text: &'static str| {
            from_main(move || {
                scene.status.set_text(text);
            });
            wait_for_next_frame();
            wait_for_next_frame();
        };
        let point = move |offset: Vec3| from_main(move || scene.handle_point(offset));

        say("a handle on the crate, arrows, rings and boxes over it");
        check_colors(SHOWN)?;

        // The x arrow, past its box.
        let from = point(Vec3::X * 0.85);
        let to = point(Vec3::X * 1.9);
        let middle = Point::new(f32::midpoint(from.x, to.x), f32::midpoint(from.y, to.y));
        inject_touches(format!(
            "{} {} b\n{} {} m\n{} {} m",
            from.x, from.y, middle.x, middle.y, to.x, to.y
        ));
        say("holding the x arrow, it is yellow and the crate follows");
        let held = from_main(move || scene.handle.as_ref().and_then(TransformHandle::active));
        ensure!(
            held.is_some_and(|part| part.mode == HandleMode::Move),
            "the press holds {held:?}"
        );
        check_colors(HOLDING)?;
        inject_touches(format!("{} {} e", to.x, to.y));
        wait_for_next_frame();
        let (position, drags) = from_main(move || (scene.crate_box.position(), scene.drags.clone()));
        let last = drags.last().copied().ok_or_else(|| anyhow::anyhow!("the move told nothing"))?;
        ensure!(
            last.part.mode == HandleMode::Move && last.part.axis == 0 && last.ended,
            "{last:?}"
        );
        ensure!(
            position.x > 0.5 && (position.y - CRATE_AT.y).abs() < 1e-4 && position.z.abs() < 1e-4,
            "the crate went to {position} on a drag along x"
        );
        ensure!(
            drags.iter().filter(|drag| drag.ended).count() == 1,
            "the move ended {drags:?}"
        );
        say("dragged the x arrow, the crate moved along x");
        check_colors(MOVED)?;

        // Round the y ring, from between z and x a third of a turn on.
        let ring = |angle: f32| Vec3::new(angle.sin(), 0.0, angle.cos()) * 0.8;
        let path: Vec<Point> = (0..=6_u8).map(|step| point(ring(PI / 4.0 + f32::from(step) * 0.2))).collect();
        drag(&path);
        let (rotation, last) = from_main(move || (scene.crate_box.rotation(), scene.drags.last().copied()));
        let last = last.ok_or_else(|| anyhow::anyhow!("the turn told nothing"))?;
        ensure!(
            last.part.mode == HandleMode::Turn && last.part.axis == 1,
            "{last:?}"
        );
        let steps = last.amount / TURN_STEP;
        ensure!(
            last.amount.abs() > TURN_STEP / 2.0 && (steps - steps.round()).abs() < 1e-4,
            "the turn is {} radians, not a step of 15 degrees",
            last.amount
        );
        let (axis, angle) = rotation.to_axis_angle();
        ensure!(
            axis.y.abs() > 0.999 && (angle - last.amount.abs()).abs() < 1e-3,
            "the crate turned {angle} about {axis}"
        );
        say("dragged round the y ring, the crate turned in steps of 15 degrees");
        check_colors(TURNED)?;

        // The box on the z arrow, out to twice as far.
        let from = point(Vec3::Z * 0.6);
        let to = point(Vec3::Z * 1.2);
        drag(&[from, to]);
        let (scale, last) = from_main(move || (scene.crate_box.world_scale(), scene.drags.last().copied()));
        let last = last.ok_or_else(|| anyhow::anyhow!("the scale told nothing"))?;
        ensure!(
            last.part.mode == HandleMode::Scale && last.part.axis == 2,
            "{last:?}"
        );
        ensure!(
            (scale - last.amount).abs() < 1e-5 && scale > 1.5,
            "the crate scaled to {scale}"
        );
        say("dragged the z box out, the crate grew");
        check_colors(SCALED)?;

        size_and_orbit(scene, say, point)
    }
}

/// The handle keeps its size when the camera pulls back, and a drag off
/// the handle turns the camera and leaves the crate.
fn size_and_orbit(
    scene: Weak<TransformHandleTest>,
    say: impl Fn(&'static str),
    point: impl Fn(Vec3) -> Point,
) -> Result<()> {
    let tip = point(Vec3::X * 1.22);
    let center = point(Vec3::ZERO);
    from_main(move || {
        let mut scene = scene;
        scene.camera.zoom(1.6);
    });
    let far_tip = point(Vec3::X * 1.22);
    let far_center = point(Vec3::ZERO);
    let length = |gap: Point| (gap.x * gap.x + gap.y * gap.y).sqrt();
    let (near, far) = (length(tip - center), length(far_tip - far_center));
    // The size holds at the center. The tip is nearer to the eye than
    // the center, and a near camera draws it a few percent longer.
    ensure!(
        (near - far).abs() < near * 0.06,
        "the arrow was {near} pixels long and is {far}"
    );
    say("the camera pulled back, the crate is smaller, the handle is not");
    check_colors(FAR)?;

    let before = from_main(move || {
        (
            scene.camera.position,
            scene.crate_box.position(),
            scene.drags.len(),
        )
    });
    drag(&[
        Point::new(80.0, 520.0),
        Point::new(130.0, 520.0),
        Point::new(180.0, 520.0),
    ]);
    let after = from_main(move || {
        (
            scene.camera.position,
            scene.crate_box.position(),
            scene.drags.len(),
        )
    });
    ensure!(
        after.0.distance(before.0) > 0.5,
        "a drag off the handle left the camera"
    );
    ensure!(
        after.1 == before.1 && after.2 == before.2,
        "a drag off the handle reached it"
    );
    say("a drag off the handle turned the camera, the crate stayed");
    check_colors(ORBITED)
}

/// A press at the first point, moves through the rest, and a release at
/// the last.
fn drag(points: &[Point]) {
    let mut lines = vec![];
    for (index, point) in points.iter().enumerate() {
        let event = match index {
            0 => "b",
            _ => "m",
        };
        lines.push(format!("{} {} {event}", point.x, point.y));
    }
    let last = points.last().expect("a drag has points");
    lines.push(format!("{} {} e", last.x, last.y));
    inject_touches(lines.join("\n"));
    wait_for_next_frame();
}

const SHOWN: &str = r"
      12   16 - #1c2530
     588   16 - #1c2530
     172   20 - #ffffff
     384   20 - #fcfcfd
     392   20 - #464d56
     240   24 - #9ca0a5
     468   24 - #4a515a
     156   28 - #737980
     200   28 - #767b82
     240   28 - #9ca0a5
     288   28 - #1c2530
     304   28 - #1c2530
     340   28 - #585e66
     376   28 - #9ca0a5
     384   28 - #ffffff
     460   28 - #1c2530
      84   36 - #1c2530
     592  140 - #597c95
     132  172 - #597c95
     300  188 - #46a758
     292  212 - #46a758
     312  236 - #e5484d
       4  240 - #597c95
     260  244 - #3e8ed0
     288  252 - #597c95
     348  260 - #3e8ed0
     232  264 - #d8dadb
     268  276 - #e5484d
     296  276 - #d8dadb
     336  284 - #e5484d
     376  288 - #d8dadb
     228  292 - #3e8ed0
     272  296 - #46a758
     252  304 - #755a97
     288  312 - #755a97
     316  312 - #715690
     212  324 - #46a758
     316  336 - #6b5289
     336  336 - #6b5289
     268  344 - #755a97
     352  344 - #e5484d
     412  344 - #e5484d
     228  352 - #46a758
     296  352 - #755a97
     432  352 - #e5484d
     264  356 - #755a97
     328  364 - #46a758
     260  376 - #3e8ed0
     304  376 - #755a97
     372  376 - #3e8ed0
     220  384 - #3e8ed0
     276  392 - #d8d9db
     556  396 - #bdc1c5
      40  416 - #d8d9db
     548  488 - #b4b9be
     544  536 - #b4b9be
       4  592 - #d7d9db
     216  592 - #d7d9db
";

const HOLDING: &str = r"
      16   12 - #1c2530
     516   12 - #1c2530
     264   20 - #1c2530
     368   20 - #ffffff
     440   20 - #787d84
     396   24 - #ffffff
     132   28 - #1c2530
     192   28 - #858a90
     228   28 - #1c2530
     308   28 - #8e9298
     356   28 - #ffffff
     372   28 - #5f656d
     588   32 - #1c2530
       4  132 - #597c95
     592  188 - #597c95
     428  216 - #46a758
     424  252 - #46a758
     400  264 - #3e8ed0
     380  268 - #3e8ed0
     448  276 - #597c95
     220  284 - #b4b9be
     428  284 - #46a758
       4  296 - #597c95
     436  296 - #e5484d
     484  296 - #3e8ed0
     332  300 - #d8d9db
     412  304 - #8869ae
     424  304 - #8869ae
     404  308 - #8869ae
     436  312 - #e5484d
     500  312 - #d8dadb
     384  316 - #46a758
     368  324 - #765b97
     468  328 - #46a758
     420  336 - #765b97
     452  336 - #6b5389
     392  340 - #e5484d
     504  340 - #d8d9db
     444  356 - #ffd23f
     384  364 - #765a97
     536  368 - #ffd23f
     520  372 - #ffd23f
     548  376 - #ffd23f
     504  380 - #3e8ed0
     532  380 - #ffd23f
     564  380 - #ffd23f
     452  384 - #6b5389
     392  388 - #3e8ed0
     420  388 - #e5484d
     364  392 - #755a97
     472  392 - #46a758
     556  396 - #bdc1c5
     440  408 - #705790
     484  408 - #3e8ed0
     384  420 - #e5484d
     120  444 - #d8d9db
     548  488 - #b4b9be
     544  536 - #b4b9be
       4  592 - #d7d9db
     232  592 - #d7d9db
";

const MOVED: &str = r"
      52   12 - #1c2530
     588   16 - #1c2530
     216   20 - #e5e6e7
     404   20 - #1c2530
     160   24 - #595f67
     260   24 - #ffffff
     296   24 - #ffffff
     352   24 - #72787f
     392   24 - #1c2530
     160   28 - #595f67
     164   28 - #1c2530
     188   28 - #8c9196
     216   28 - #e5e6e7
     256   28 - #464d56
     260   28 - #ffffff
     324   28 - #8a8f95
     428   28 - #414952
     508   36 - #1c2530
       4  108 - #597c95
     592  176 - #597c95
     428  216 - #46a758
     424  248 - #46a758
     380  268 - #3e8ed0
     436  276 - #e5484d
     412  280 - #46a758
     220  284 - #b4b9be
       4  288 - #597c95
     436  292 - #e5484d
     484  296 - #3e8ed0
     332  300 - #d8d9db
     412  304 - #8869ae
     424  304 - #8869ae
     404  308 - #8869ae
     384  316 - #46a758
     464  316 - #6b5389
     360  320 - #46a758
     440  320 - #46a758
     460  336 - #6b5389
     392  340 - #e5484d
     504  340 - #d8d9db
     364  352 - #765a97
     444  352 - #765b97
     412  368 - #765b97
     544  372 - #e5484d
     448  376 - #6b5389
     500  376 - #d8d9db
     564  380 - #e5484d
     392  384 - #755a97
     556  396 - #bdc1c5
     444  400 - #6b5389
     368  404 - #3e8ed0
     484  408 - #3e8ed0
     384  420 - #e5484d
     412  424 - #3e8ed0
     452  424 - #3e8ed0
     120  444 - #d8d9db
     548  488 - #b4b9be
     544  536 - #b4b9be
       4  592 - #d7d9db
     244  592 - #d7d9db
";

const TURNED: &str = r"
      12   16 - #1c2530
     140   20 - #ffffff
     452   20 - #b8bbbe
     140   24 - #ffffff
     168   24 - #595f67
     248   24 - #495059
     324   24 - #e7e8e9
     100   28 - #1c2530
     140   28 - #ffffff
     200   28 - #6f757b
     224   28 - #414952
     248   28 - #495059
     388   28 - #1c2530
     432   28 - #9a9ea2
     504   28 - #1e2732
     588   32 - #1c2530
      56   36 - #1c2530
     592  188 - #597c95
     168  212 - #597c95
     428  216 - #46a758
     416  240 - #46a758
     380  268 - #3e8ed0
     424  268 - #597c95
     444  272 - #597c95
     432  276 - #597c95
     220  284 - #b4b9be
     476  288 - #3e8ed0
     436  292 - #e5484d
       4  300 - #597c95
     332  300 - #d8d9db
     408  304 - #8869ae
     424  308 - #8869ae
     368  312 - #5a4673
     496  312 - #3e8ed0
     440  320 - #46a758
     468  328 - #46a758
     396  332 - #5a4673
     420  340 - #7b5f9e
     504  340 - #d8d9db
     384  352 - #5a4673
     444  352 - #7b5f9e
     412  368 - #7b5f9e
     504  368 - #e5484d
     380  372 - #5a4673
     544  372 - #e5484d
     532  376 - #e5484d
     564  380 - #e5484d
     400  392 - #46a758
     432  396 - #46a758
     464  396 - #7b5f9e
     556  396 - #bdc1c5
     368  400 - #3e8ed0
     484  408 - #3e8ed0
     384  420 - #e5484d
     412  424 - #3e8ed0
     124  444 - #d8d9db
     544  536 - #b4b9be
       4  592 - #d7d9db
     236  592 - #d7d9db
";

const SCALED: &str = r"
      76   12 - #1c2530
     588   16 - #1c2530
     272   20 - #71767d
     396   24 - #ffffff
     400   24 - #ffffff
     188   28 - #303842
     236   28 - #1c2530
     316   28 - #1c2530
     340   28 - #414952
     400   28 - #fbfcfc
     496   36 - #1c2530
       4  100 - #597c95
     592  172 - #597c95
     428  216 - #46a758
     416  240 - #46a758
     332  256 - #7c6ea8
     380  268 - #3e8ed0
     428  268 - #8869ae
     220  284 - #b4b9be
     532  284 - #8869ae
     436  288 - #e5484d
     416  292 - #8869ae
     344  296 - #3e8ed0
     484  296 - #3e8ed0
       4  300 - #597c95
     316  300 - #5a4673
     436  312 - #e5484d
     356  320 - #46a758
     500  320 - #3e8ed0
     396  324 - #e5484d
     456  324 - #46a758
     556  324 - #d8dadb
     476  332 - #46a758
     432  336 - #e5484d
     512  340 - #7b5f9e
     328  348 - #5a4673
     428  356 - #e5484d
     488  364 - #e5484d
     388  368 - #3e8ed0
     520  372 - #e5484d
     552  372 - #d8d9db
     324  380 - #5a4673
     356  384 - #5a4673
     500  388 - #3e8ed0
     368  392 - #5a4673
     400  392 - #46a758
     444  396 - #46a758
     556  396 - #bdc1c5
     340  408 - #5a4673
     484  408 - #3e8ed0
     396  428 - #7b5f9e
     372  436 - #735993
     460  440 - #d8d9db
       4  472 - #d8d9db
     548  488 - #b4b9be
     544  536 - #b4b9be
     124  592 - #d7d9db
     296  592 - #d7d9db
";

const FAR: &str = r"
     588   16 - #1c2530
     104   20 - #ffffff
     252   20 - #1c2530
     368   20 - #61676f
     408   20 - #1c2530
     140   24 - #ffffff
     192   24 - #535a62
     288   24 - #1c2530
     324   24 - #333b45
     104   28 - #ffffff
     148   28 - #1c2530
     324   28 - #333b45
     356   28 - #acafb4
     452   28 - #787d84
      12   36 - #1c2530
     520   36 - #1c2530
     592  200 - #597c95
     376  204 - #46a758
     372  232 - #46a758
     352  252 - #468ac1
     328  260 - #3e8ed0
     392  264 - #e5484d
     368  276 - #46a758
     428  280 - #3e8ed0
     252  284 - #d8dadb
     396  288 - #e5484d
     304  296 - #b4b9be
     304  300 - #d8d9db
     360  300 - #8869ae
     332  312 - #5a4673
     404  312 - #46a758
     232  316 - #d8d9db
     452  316 - #3e8ed0
     428  320 - #46a758
     312  332 - #5a4673
     364  332 - #7b5f9e
     468  336 - #d8d9db
     280  344 - #46a758
     412  348 - #e5484d
      60  352 - #d8d9db
     388  352 - #e5484d
     440  356 - #e5484d
     320  368 - #5a4673
     508  368 - #e5484d
     484  372 - #e5484d
     416  376 - #7b5f9e
     428  380 - #7b5f9e
     332  384 - #7d476a
     340  384 - #735993
     364  384 - #46a758
     388  388 - #7b5f9e
     308  404 - #3e8ed0
     424  408 - #3e8ed0
     360  412 - #3e8ed0
     180  504 - #d7d9db
       4  592 - #597c95
     404  592 - #d7d9db
     592  592 - #597c95
";

const ORBITED: &str = r"
      12   16 - #1c2530
     588   16 - #1c2530
     236   20 - #ffffff
     300   20 - #1c2530
     388   20 - #fbfcfc
     460   24 - #1c2530
     136   28 - #1c2530
     172   28 - #323a44
     212   28 - #414952
     236   28 - #ffffff
     272   28 - #595f67
     348   28 - #545b63
     416   28 - #ced0d2
     592  184 - #597c95
     376  208 - #46a758
       4  248 - #597c95
     376  256 - #46a758
     372  264 - #e5484d
     376  276 - #46a758
     332  280 - #8869ae
     416  280 - #8869ae
     452  280 - #3e8ed0
     300  296 - #d8dadb
     360  300 - #8869ae
     400  300 - #46a758
     376  304 - #8869ae
     336  320 - #5a4573
     408  328 - #e5484d
     492  328 - #e5484d
     508  332 - #e5484d
     460  340 - #3e8ed0
     292  348 - #d8d9db
     320  356 - #5a4673
     384  360 - #3e8ed0
     432  364 - #46a758
     344  368 - #5a4573
     368  372 - #7d466a
     368  376 - #7d466a
     392  376 - #7b5f9e
     316  384 - #3e8ed0
     416  392 - #3e8ed0
     368  412 - #e5484d
     504  424 - #bdc1c5
      72  428 - #b4b9be
     276  440 - #b4b9be
     588  452 - #d8d9db
     172  468 - #bdc1c5
     384  472 - #bdc1c5
";
