mod depth;
mod in_view;

use std::sync::mpsc::channel;

use anyhow::{Result, bail, ensure};
use hilen::{
    AppRunner,
    dispatch::{from_main, spawn, wait_for_next_frame},
    gm::{
        LossyConvert,
        color::U8Color,
        volume::{Shape3, Vec3},
    },
    refs::Weak,
    scene::{
        Camera, NodeTemplates, Prop, Scene, SceneCreation, SceneManager, ScenePhoto, SceneSetup, SceneTest,
        SceneTestView, scene,
    },
    ui::{Color, Image, ImageView, Label, Size, ViewData, ViewFrame, ViewSubviews},
    ui_test::check_colors,
};

/// What a phone on a rig takes, 12 megapixels, far over the 600 x 600
/// canvas.
const FULL: (u32, u32) = (4032, 3024);
/// The photos of a turn of the rig, a quarter of the full size each.
const TURN: (u32, u32) = (1008, 756);
/// Photos in 1 turn of the rig, 1 every 15 degrees.
const TURN_PHOTOS: u32 = 24;
const BACKGROUND: Color = Color::hex("#22303c");
/// How far a channel of a photo pixel may be from the recorded one. A
/// browser on WebGL lights with other float steps than a native GPU.
const CHANNEL_STEP: u8 = 3;

/// Where the photos are taken from, up and to the side of the rig, not
/// where the scene camera stands.
fn phone_at(degrees: f32) -> Camera {
    let turn = degrees.to_radians();
    Camera {
        position: Vec3::new(5.4 * turn.sin(), 4.0, 5.4 * turn.cos()),
        target: Vec3::new(0.0, 0.5, 0.0),
        ..Camera::default()
    }
}

/// The camera the scene runs with, low and in front of the rig.
fn scene_camera() -> Camera {
    Camera {
        position: Vec3::new(0.0, 2.0, 8.0),
        target: Vec3::new(0.0, 0.6, 0.0),
        ..Camera::default()
    }
}

/// The rig: a small floor, a red cube in the middle where the turntable
/// is, a blue ball on the left and a yellow crate on the right.
fn build_rig<S: Scene + ?Sized>(scene: &mut S) {
    scene.camera = scene_camera();
    scene
        .make_node::<Prop>(Shape3::Plane(6.0), Vec3::ZERO)
        .set_color(Color::hex("#c8ccd0"))
        .set_roughness(0.9);
    scene
        .make_node::<Prop>(Shape3::cube(1.2), Vec3::new(0.0, 0.6, 0.0))
        .set_color(Color::hex("#c0392b"));
    scene
        .make_node::<Prop>(Shape3::Ball(0.6), Vec3::new(-2.0, 0.6, 0.0))
        .set_color(Color::hex("#2980b9"));
    scene
        .make_node::<Prop>(Shape3::cube(0.8), Vec3::new(2.0, 0.4, 0.0))
        .set_color(Color::hex("#f1c40f"));
}

/// Takes the photos of `cameras` in a row on the main thread, each of
/// `size`, and waits for all of them.
fn photos(cameras: Vec<Camera>, size: (u32, u32)) -> Result<Vec<ScenePhoto>> {
    let count = cameras.len();
    let (sender, receiver) = channel();
    from_main(move || {
        for (index, camera) in cameras.into_iter().enumerate() {
            let photo = SceneManager::photo(camera, size, BACKGROUND);
            let sender = sender.clone();
            spawn(async move {
                sender.send((index, photo.await)).expect("the test waits for every photo");
            });
        }
    });
    let mut taken: Vec<Option<ScenePhoto>> = vec![None; count];
    for _ in 0..count {
        let (index, photo) = receiver.recv()?;
        taken[index] = Some(photo?);
    }
    Ok(taken.into_iter().flatten().collect())
}

/// Points of the phone photo as parts of its width and height, with the
/// color each has, the same in a photo of any size.
const PHONE_PROBES: &[(f32, f32, &str)] = &[
    (0.5, 0.1, "#22303c"),
    (0.29, 0.49, "#4586b9"),
    (0.5, 0.53, "#9d362d"),
    (0.71, 0.54, "#c4a020"),
    (0.5, 0.78, "#bec1c5"),
];

/// Checks the known pixels of a photo through `phone_at(0.0)`.
fn check_phone_photo(photo: &ScenePhoto, size: (u32, u32)) -> Result<()> {
    ensure!(
        photo.size == Size::new(size.0, size.1),
        "the photo is {:?}, not {size:?}",
        photo.size
    );
    let mut wrong = vec![];
    let mut seen = vec![];
    for &(x, y, hex) in PHONE_PROBES.iter().chain(&[(0.01, 0.01, "#22303c")]) {
        let (px, py) = (fraction(x, size.0), fraction(y, size.1));
        let Some(pixel) = photo.pixel(px, py) else {
            bail!("no pixel at {px} {py}");
        };
        let want = U8Color::from(Color::hex(hex));
        seen.push(format!("{x} {y} {}", pixel.as_hex()));
        let off = |a: u8, b: u8| a.abs_diff(b) > CHANNEL_STEP;
        if off(pixel.r, want.r) || off(pixel.g, want.g) || off(pixel.b, want.b) || pixel.a != 255 {
            wrong.push(format!(
                "{x} {y} is {} with alpha {}, want {hex}",
                pixel.as_hex(),
                pixel.a
            ));
        }
    }
    ensure!(
        wrong.is_empty(),
        "the photo of {size:?} differs:\n{}\nseen:\n{}",
        wrong.join("\n"),
        seen.join("\n")
    );
    Ok(())
}

fn fraction(part: f32, of: u32) -> u32 {
    let of: f32 = of.lossy_convert();
    (part * of).lossy_convert()
}

/// The `check_colors` blocks of a test, the rig alone and with the photo
/// shown.
struct Blocks {
    rig:   &'static str,
    shown: &'static str,
}

/// Takes the full photo and a turn of photos while the scene runs, checks
/// them and that the frame on screen did not change, then shows the full
/// photo in `image_frame`.
fn shoot_and_show(
    root: Weak<SceneTestView>,
    status: Weak<Label>,
    image_frame: (f32, f32, f32, f32),
    blocks: &Blocks,
    scene_camera_now: impl Fn() -> Camera,
) -> Result<()> {
    let say = move |text: &'static str| {
        from_main(move || {
            status.set_text(text);
        });
        wait_for_next_frame();
        wait_for_next_frame();
    };

    say("the rig through the scene camera, photos are taken from above");
    check_colors(blocks.rig)?;
    let before = AppRunner::take_screenshot()?;

    let full = photos(vec![phone_at(0.0)], FULL)?.remove(0);
    check_phone_photo(&full, FULL)?;

    let turn: Vec<Camera> = (0..TURN_PHOTOS)
        .map(|step| {
            let step: f32 = step.lossy_convert();
            phone_at(15.0 * step)
        })
        .collect();
    let turn = photos(turn, TURN)?;
    ensure!(
        turn.len() == usize::try_from(TURN_PHOTOS)?,
        "{} photos of the turn came back",
        turn.len()
    );
    check_phone_photo(&turn[0], TURN)?;
    for (step, photo) in turn.iter().enumerate() {
        let center = photo.pixel(TURN.0 / 2, TURN.1 / 2);
        ensure!(
            center.is_some_and(|pixel| pixel.r > 100 && pixel.g < 90 && pixel.b < 90),
            "photo {step} of the turn has no red cube in its middle: {center:?}"
        );
    }

    wait_for_next_frame();
    let after = AppRunner::take_screenshot()?;
    ensure!(
        before.data == after.data,
        "the photos changed the frame on screen"
    );
    ensure!(
        scene_camera_now() == scene_camera(),
        "the photos moved the camera of the scene"
    );

    from_main(move || {
        let image = Image::from_raw_data(full.rgba, "scene photo test", full.size, 4);
        let view = root.add_view::<ImageView>();
        view.set_image(image);
        view.set_frame(image_frame);
        view.set_border_width(2).set_border_color(Color::hex("#ffffff"));
    });
    say("the 4032 x 3024 photo from above, shown small");
    check_colors(blocks.shown)
}

/// A rig in a scene over the whole canvas, seen from the front, while a
/// phone above it takes a photo of 4032 x 3024 and 24 more on a turn
/// around it. The scene and the frame on screen do not change, then the
/// big photo shows small in the corner.
#[scene]
#[derive(Default)]
struct PhotoOfScene {
    root:   Weak<SceneTestView>,
    status: Weak<Label>,
}

impl SceneSetup for PhotoOfScene {
    fn setup(&mut self) {
        build_rig(self);
    }
}

/// A caption over the scene, white on dark.
fn caption(view: Weak<SceneTestView>, text: &str, frame: (f32, f32, f32, f32)) -> Weak<Label> {
    let label = view.add_view::<Label>();
    label.set_text(text).set_text_size(15).set_text_color(Color::hex("#ffffff"));
    label.set_color(Color::hex("#1c2530")).set_corner_radius(6).set_frame(frame);
    label
}

impl SceneTest for PhotoOfScene {
    fn overlay(mut scene: Weak<Self>, view: Weak<SceneTestView>) {
        scene.root = view;
        scene.status = caption(view, "", (10.0, 10.0, 580.0, 30.0));
    }

    fn perform_test(scene: Weak<Self>) -> Result<()> {
        let (root, status) = from_main(move || (scene.root, scene.status));
        shoot_and_show(root, status, (180.0, 415.0, 240.0, 180.0), &BLOCKS, move || {
            from_main(move || scene.camera)
        })
    }
}

const BLOCKS: Blocks = Blocks {
    rig:   RIG,
    shown: SHOWN,
};

const RIG: &str = r"
         588   16 - #1c2530
         132   24 - #767b82
         376   24 - #595f67
         144   28 - #737980
         152   28 - #b8bbbe
         200   28 - #1c2530
         248   28 - #414952
         284   28 - #272f3a
         288   28 - #f0f0f1
         332   28 - #fefefe
         440   28 - #8a8f95
         452   28 - #898d93
          12   36 - #1c2530
         296  260 - #b63f34
         168  280 - #508cbe
         176  284 - #6f9eca
         424  284 - #e3b924
         140  292 - #2b6793
         208  292 - #2d6c9a
         400  296 - #846b1b
         200  320 - #234d6d
         156  332 - #214865
         432  340 - #c5a022
         312  344 - #9d372e
          20  400 - #bec2c5
         592  408 - #bec2c5
           4  592 - #597c95
         592  592 - #597c95
";

const SHOWN: &str = r"
         588   16 - #1c2530
         204   20 - #1c2530
         244   24 - #dcdedf
         300   24 - #ffffff
         412   24 - #868b91
         152   28 - #585e66
         300   28 - #e5e6e7
         316   28 - #1c2530
         324   28 - #3a424c
         384   28 - #9ca0a5
         432   28 - #b6b9bc
         296  260 - #b63f34
         168  280 - #508cbe
         176  284 - #6f9eca
         424  284 - #e3b924
         140  292 - #2b6793
         400  296 - #846b1b
         200  320 - #234d6d
         156  332 - #214865
          20  400 - #bec2c5
         180  416 - #ffffff
         244  500 - #387fb4
         264  504 - #2c6895
         340  504 - #846b1b
         336  508 - #887127
         252  516 - #28506e
         340  516 - #c39f20
         348  520 - #c2ad5c
         356  520 - #c2ad5c
           4  592 - #597c95
";
