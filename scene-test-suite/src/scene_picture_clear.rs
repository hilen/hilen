use anyhow::Result;
use hilen::{
    dispatch::wait_for_next_frame,
    gm::volume::{Shape3, Vec3},
    refs::{Weak, manage::DataManager},
    scene::{
        Camera, Model, NodeTemplates, Prop, SceneCreation, SceneManager, SceneSetup, SceneTest,
        SceneTestView, scene,
    },
    ui::{Color, Container, ImageView, Label, Size, ViewData, ViewFrame, ViewSubviews},
    ui_test::check_colors,
};

const PICTURE: u32 = 256;
/// Where the pictures sit, half over the light view and half over the
/// dark one.
const PICTURE_Y: f32 = 172.0;
const CLEAR_X: f32 = 30.0;
const FILLED_X: f32 = 314.0;

/// The monkey with a see through blue ball in front of its chin, over
/// nothing.
#[scene]
#[derive(Default)]
struct GlassShot {}

impl SceneSetup for GlassShot {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.9, 0.2, 3.4),
            target: Vec3::new(0.2, -0.25, 0.0),
            ..Camera::default()
        };
        self.make_node::<Prop>(Shape3::Model(Model::get("Monkey.glb")), Vec3::ZERO)
            .set_color(Color::hex("#e0a060"))
            .set_roughness(0.6);
        self.make_node::<Prop>(Shape3::Ball(0.55), Vec3::new(0.5, -0.5, 1.0))
            .set_color(Color::rgba(0.2, 0.5, 1.0, 0.5))
            .set_roughness(0.3);
    }
}

/// The same picture twice over a light view on top and a dark view below,
/// drawn while no scene runs. The left one has a clear background: both
/// views show around the monkey and tinted through the ball, and the
/// monkey's outline has no dark or light rim on either color, which is
/// what the divide by alpha in the picture pass is for. The right one has
/// a given background and covers the views. A white frame marks where
/// each image view is.
#[scene]
#[derive(Default)]
struct ScenePictureClear {}

fn caption(view: Weak<SceneTestView>, text: &str, frame: (f32, f32, f32, f32)) {
    let label = view.add_view::<Label>();
    label.set_text(text).set_text_size(15).set_text_color(Color::hex("#ffffff"));
    label.set_color(Color::hex("#1c2530")).set_corner_radius(6).set_frame(frame);
}

fn picture(view: Weak<SceneTestView>, name: &str, background: Color, x: f32) {
    let image = SceneManager::picture(
        GlassShot::default(),
        name,
        Size::new(PICTURE, PICTURE),
        background,
    );
    let picture = view.add_view::<ImageView>();
    picture.set_image(image);
    picture.set_frame((x, PICTURE_Y, 256.0, 256.0));
    picture.set_border_width(2).set_border_color(Color::hex("#ffffff"));
}

impl SceneTest for ScenePictureClear {
    fn overlay(_: Weak<Self>, view: Weak<SceneTestView>) {
        // The harness started this scene, a picture has to work without one.
        SceneManager::stop_scene();

        view.add_view::<Container>()
            .set_color(Color::hex("#e8e0c8"))
            .set_frame((0, 0, 600, 300));
        view.add_view::<Container>()
            .set_color(Color::hex("#203040"))
            .set_frame((0, 300, 600, 300));

        picture(
            view,
            "scene-picture-clear-test",
            Color::rgba(0.0, 0.0, 0.0, 0.0),
            CLEAR_X,
        );
        picture(view, "scene-picture-filled-test", Color::hex("#5a2d4a"), FILLED_X);

        caption(
            view,
            "2 pictures of one scene, no scene is running",
            (10.0, 10.0, 580.0, 30.0),
        );
        caption(view, "light view", (10.0, 60.0, 120.0, 28.0));
        caption(view, "dark view", (10.0, 560.0, 120.0, 28.0));
        caption(view, "clear background", (CLEAR_X, 130.0, 256.0, 28.0));
        caption(view, "both views show through", (CLEAR_X, 440.0, 256.0, 28.0));
        caption(view, "background #5a2d4a", (FILLED_X, 130.0, 256.0, 28.0));
        caption(view, "the views are covered", (FILLED_X, 440.0, 256.0, 28.0));
    }

    fn perform_test(_: Weak<Self>) -> Result<()> {
        wait_for_next_frame();
        check_colors(CHECK_1)
    }
}

const CHECK_1: &str = r"
         592    4 - #e8e0c8
         264   24 - #ecedee
         300   28 - #b2b5b9
         396   28 - #868b91
          44   76 - #d2d4d6
         412  144 - #9ca0a5
         432  144 - #3c444d
         112  148 - #1c2530
         188  224 - #d69b61
         436  224 - #db9f65
          80  248 - #b78452
         124  248 - #7b5838
         364  248 - #b78452
         516  252 - #c7915e
         228  268 - #ae7d4e
         440  268 - #8b643f
         392  288 - #7b5838
         120  292 - #7b5838
         188  292 - #93aedc
         176  296 - #95afdd
         176  316 - #60729c
         448  316 - #8e8aa4
         484  352 - #464c8e
         284  356 - #ffffff
         428  356 - #434682
         456  372 - #42427a
         140  376 - #223c66
         564  424 - #5a2d4a
          76  456 - #ffffff
         228  456 - #1c2530
         380  456 - #495059
           4  592 - #203040
";
