use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Shape3, Vec3},
    refs::{Weak, manage::DataManager},
    scene::{
        Camera, Model, NodeTemplates, Prop, SceneCreation, SceneManager, SceneSetup, SceneTest,
        SceneTestView, Sky, scene,
    },
    ui::{Color, Image, ImageView, Label, Size, ViewData, ViewFrame, ViewSubviews},
    ui_test::check_colors,
};

const PICTURE: u32 = 256;
const NAME: &str = "scene-picture-test";
/// Pictures a model grid makes when it opens.
const GRID: usize = 50;

/// What a picture is taken of: one model alone, the camera pulled back
/// until its bounds fit, the way a model grid frames every entry.
#[scene]
#[derive(Default)]
struct ModelShot {
    model: &'static str,
    color: Color,
}

impl SceneSetup for ModelShot {
    fn setup(&mut self) {
        let model = Model::get(self.model);
        let center = (model.bounds.min + model.bounds.max) / 2.0;
        let radius = (model.bounds.max - model.bounds.min).length() / 2.0;
        let camera = Camera::default();
        let distance = radius / (camera.fov_y / 2.0).tan() * 1.05;
        self.camera = Camera {
            position: center + Vec3::new(0.35, 0.25, 1.0).normalize() * distance,
            target: center,
            ..camera
        };
        let color = self.color;
        self.make_node::<Prop>(Shape3::Model(model), Vec3::ZERO)
            .set_color(color)
            .set_roughness(0.8);
    }
}

/// A tree in a running scene on the left and a picture of the same tree
/// in an `ImageView` on the right, drawn by `SceneManager::picture` while
/// the scene runs. The picture has its own camera and its own background,
/// and the running scene draws on as before. Then the same image is drawn
/// again with the monkey, so the view shows the new picture with nothing
/// set on it.
#[scene]
#[derive(Default)]
struct ScenePicture {
    image:  Weak<Image>,
    /// Says on screen what the current step shows.
    status: Weak<Label>,
}

impl SceneSetup for ScenePicture {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(2.4, 3.2, 8.5),
            target: Vec3::new(2.4, 1.6, 0.0),
            ..Camera::default()
        };
        self.sky = Some(Sky::gradient(
            Color::hex("#3a7bd5"),
            Color::hex("#d9e4f0"),
            Color::hex("#5a4a3a"),
        ));
        self.make_node::<Prop>(Shape3::Plane(40.0), Vec3::ZERO)
            .set_color(Color::hex("#6f8f5a"))
            .set_roughness(0.9);
        self.make_node::<Prop>(Shape3::Model(Model::get("tree.glb")), Vec3::ZERO)
            .set_roughness(0.8);
    }
}

/// A caption over the scene, white on dark.
fn caption(view: Weak<SceneTestView>, text: &str, frame: (f32, f32, f32, f32)) -> Weak<Label> {
    let label = view.add_view::<Label>();
    label.set_text(text).set_text_size(15).set_text_color(Color::hex("#ffffff"));
    label.set_color(Color::hex("#1c2530")).set_corner_radius(6).set_frame(frame);
    label
}

fn size() -> Size<u32> {
    Size::new(PICTURE, PICTURE)
}

impl SceneTest for ScenePicture {
    fn overlay(mut scene: Weak<Self>, view: Weak<SceneTestView>) {
        scene.image = SceneManager::picture(
            ModelShot {
                model: "tree.glb",
                color: Color::hex("#ffffff"),
                ..ModelShot::default()
            },
            NAME,
            size(),
            Color::hex("#22303c"),
        );

        let picture = view.add_view::<ImageView>();
        picture.set_image(scene.image);
        picture.set_frame((330, 80, PICTURE, PICTURE));
        picture.set_border_width(2).set_border_color(Color::hex("#ffffff"));

        scene.status = caption(view, "", (10.0, 10.0, 580.0, 30.0));
        caption(view, "the running scene", (40.0, 420.0, 200.0, 28.0));
        caption(view, "a picture in an image view", (330.0, 344.0, 256.0, 28.0));
    }

    fn perform_test(scene: Weak<Self>) -> Result<()> {
        let say = move |text: &'static str| {
            from_main(move || {
                scene.status.set_text(text);
            });
            wait_for_next_frame();
        };

        say("a picture of the same tree, its own camera and background");
        let image_size = from_main(move || scene.image.size);
        ensure!(image_size == size(), "the picture is {image_size:?}");
        check_colors(CHECK_1)?;

        // The same name and size draws into the same image, the view over
        // it shows the monkey without being told.
        let same = from_main(move || {
            let again = SceneManager::picture(
                ModelShot {
                    model: "Monkey.glb",
                    color: Color::hex("#e0a060"),
                    ..ModelShot::default()
                },
                NAME,
                size(),
                Color::hex("#5a2d4a"),
            );
            again.raw() == scene.image.raw()
        });
        ensure!(same, "the second picture went into another image");
        say("the same image drawn again with the monkey, the view was not touched");
        check_colors(CHECK_2)?;

        // The start page of a model grid: 50 pictures in a row, each its own
        // image. The frame on screen is the same after them.
        let made = from_main(move || {
            (0..GRID)
                .map(|index| {
                    SceneManager::picture(
                        ModelShot {
                            model: "tree.glb",
                            color: Color::hex("#ffffff"),
                            ..ModelShot::default()
                        },
                        &format!("{NAME}-{index}"),
                        size(),
                        Color::hex("#22303c"),
                    )
                })
                .filter(|image| image.raw() != scene.image.raw() && image.size == size())
                .count()
        });
        ensure!(made == GRID, "{made} of {GRID} pictures were made");
        say("50 more pictures were made, nothing on screen changed");
        check_colors(CHECK_3)
    }
}

const CHECK_1: &str = r"
          12   16 - #1c2530
         132   20 - #1c2530
         256   24 - #ffffff
         284   24 - #8e9298
         396   24 - #414952
         196   28 - #414952
         284   28 - #8e9298
         328   28 - #fefefe
         352   28 - #5b626a
         480   28 - #b8bbbe
         484   28 - #d1d3d5
         584   96 - #ffffff
         116  112 - #29512a
         184  120 - #29512a
          56  136 - #d3deeb
         448  164 - #3a6837
         332  208 - #22303c
         584  216 - #ffffff
         132  232 - #4b9a50
           4  256 - #d4dfeb
         456  284 - #614834
         456  288 - #614834
         240  292 - #3e7844
         164  348 - #6b513c
         404  360 - #1c2530
         500  360 - #ffffff
         160  396 - #6b513c
          84  436 - #1c2530
         136  436 - #6d737a
         184  436 - #e1e2e3
         356  592 - #6c926f
         592  592 - #6d9370
";

const CHECK_2: &str = r"
         584   12 - #1c2530
         176   20 - #596068
         376   20 - #6f757b
         516   20 - #caccce
         112   28 - #f0f0f1
         204   28 - #9ca0a5
         244   28 - #414952
         324   28 - #95999e
         464   28 - #1c2530
         528   80 - #ffffff
         100  112 - #29512a
         184  120 - #29512a
         468  144 - #dea166
         544  176 - #c9925d
         452  188 - #c8915c
         408  204 - #996e46
         404  212 - #89633f
         520  212 - #e2a469
         480  220 - #a5774b
           4  240 - #d5dfeb
         436  240 - #8e6641
         440  280 - #dfa267
         224  304 - #3e7844
         580  332 - #5a2d4a
         404  360 - #1c2530
         444  360 - #414952
         464  360 - #6f757b
         500  360 - #ffffff
         176  388 - #674f3b
          92  436 - #e0e1e3
         136  436 - #6d737a
         592  592 - #6d9370
";

const CHECK_3: &str = r"
         116   20 - #b8bbbe
         332   20 - #464d56
         172   24 - #ffffff
         144   28 - #3a424c
         264   28 - #5b626a
         372   28 - #fefefe
         420   28 - #caccce
         456   28 - #9ca0a5
          12   36 - #1c2530
         584   80 - #ffffff
         100  112 - #29512a
         184  120 - #29512a
         468  144 - #dea166
         544  176 - #c9925d
         452  188 - #c8915c
         408  204 - #996e46
         404  212 - #89633f
         520  212 - #e2a469
         480  220 - #a5774b
         436  240 - #8e6641
           4  256 - #d4dfeb
         228  280 - #3e7844
         440  280 - #dfa267
         580  332 - #5a2d4a
         160  348 - #6b513c
         404  360 - #1c2530
         464  360 - #6f757b
         508  360 - #1c2530
          92  436 - #e0e1e3
         136  436 - #6d737a
         184  436 - #e1e2e3
         592  592 - #6d9370
";
