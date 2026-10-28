use anyhow::Result;
use hilen::{
    dispatch::wait_for_next_frame,
    gm::volume::{Shape3, Vec3},
    refs::Weak,
    scene::{Camera, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, scene},
    ui::Color,
    ui_test::{check_colors, set_record_probe_count},
};

/// A cube floating behind the camera, out of its view, under a sun that
/// throws its shadow onto the floor in the middle of the view. Each pass
/// leaves out the nodes it cannot see, and a cascade that left out what
/// the camera cannot see would lose this shadow.
#[scene]
#[derive(Default)]
struct ShadowOutOfView {}

impl SceneSetup for ShadowOutOfView {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 5.0, 4.0),
            target: Vec3::new(0.0, 0.0, -1.0),
            ..Camera::default()
        };
        self.sun.direction = Vec3::new(0.0, -1.0, -1.5);
        self.sun.shadows = true;
        // The default is 1024 on a phone and in the browser.
        self.sun.shadow_map_size = 2048;

        self.make_node::<Prop>(Shape3::Plane(40.0), Vec3::ZERO)
            .set_color(Color::hex("#c8ccd0"))
            .set_roughness(0.9);
        self.make_node::<Prop>(Shape3::cube(1.5), Vec3::new(0.0, 7.0, 9.0))
            .set_color(Color::hex("#d03a3a"))
            .set_roughness(0.7);
    }
}

impl SceneTest for ShadowOutOfView {
    fn perform_test(_: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);
        wait_for_next_frame();
        check_colors(SHADOW)
    }
}

/// The floor and the shadow of the cube in the middle of it, with no
/// red of the cube itself.
const SHADOW: &str = r"
       4    4 - #aaadb1
     216    4 - #aaadb1
     592    4 - #aaadb1
     112   20 - #aaadb1
     404   28 - #aaadb1
     500   44 - #aaadb0
     300   92 - #aaadb0
     208  120 - #aaadb0
     112  128 - #aaadb0
     412  132 - #aaadb0
       4  140 - #aaadb0
     512  140 - #aaadb0
     280  204 - #6e7072
     320  204 - #6e7072
     344  208 - #6e7072
     256  212 - #6e7072
     300  216 - #6e7072
     332  228 - #6e7072
     276  232 - #6e7072
     312  240 - #6e7072
       4  252 - #aaadb0
     252  252 - #6e7072
     336  252 - #6e7072
     128  256 - #aaadb0
     288  260 - #6e7072
     464  260 - #aaadb0
     312  264 - #6e7072
     592  272 - #aaadb0
     248  276 - #6e7072
     348  276 - #6e7072
     272  288 - #6e7072
     304  288 - #6e7072
     332  296 - #6e7072
     244  300 - #aaadb0
     352  308 - #6e7072
     288  312 - #6e7072
     316  320 - #6e7072
     264  324 - #6e7072
     356  332 - #6e7072
     244  344 - #6e7072
     280  344 - #6e7072
     148  348 - #aaadb0
     316  348 - #6e7072
       4  356 - #aaadb0
     344  356 - #6e7072
     296  360 - #6e7072
     460  360 - #aaadb0
     240  376 - #6e7072
     272  380 - #6e7072
     308  380 - #6e7072
     332  380 - #6e7072
     360  380 - #6e7072
      88  420 - #aaadb0
     528  432 - #aaadb0
     428  456 - #aaadb0
     200  476 - #aaadb0
       4  484 - #aaadb0
     304  500 - #aaadb0
     508  532 - #aaadb0
     116  540 - #aaadb0
     408  560 - #aaadb0
       4  592 - #aaadb0
     224  592 - #aaadb0
     592  592 - #aaadb0
";
