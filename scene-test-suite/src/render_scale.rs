use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Shape3, Vec3},
    refs::Weak,
    scene::{
        Camera, NodeTemplates, Prop, SceneCreation, SceneManager, SceneSetup, SceneTest, SceneTestView, scene,
    },
    ui::{Color, Label, ViewData, ViewFrame, ViewSubviews},
    ui_test::{check_colors, set_record_probe_count},
};

/// A flat box under a see through panel. At half the pixels the picture
/// is stretched over the whole window, so the box stays where it was and
/// the panel still blends over the scene. A picture drawn after the rects
/// of the views would leave the panel over an empty frame.
#[scene]
#[derive(Default)]
struct RenderScale {}

impl SceneSetup for RenderScale {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 0.0, 5.0),
            target: Vec3::ZERO,
            ..Camera::default()
        };
        self.ambient = Color::rgb(1.0, 1.0, 1.0);
        let mut node = self.make_node::<Prop>(Shape3::Box(Vec3::new(2.0, 2.0, 0.2)), Vec3::ZERO);
        node.set_color(Color::hex("#d35400"));
    }
}

impl SceneTest for RenderScale {
    fn overlay(_: Weak<Self>, view: Weak<SceneTestView>) {
        let panel = view.add_view::<Label>();
        panel
            .set_color(Color::rgba(0.0, 0.0, 1.0, 0.5))
            .set_frame((250.0, 100.0, 100.0, 400.0));
    }

    fn perform_test(_: Weak<Self>) -> Result<()> {
        set_record_probe_count(32);
        wait_for_next_frame();
        check_colors(WHOLE)?;
        from_main(|| SceneManager::set_render_scale(0.5));
        wait_for_next_frame();
        wait_for_next_frame();
        check_colors(HALF)
    }
}

const WHOLE: &str = r"
       4    4 - #597c95
     592    4 - #597c95
     252  100 - #2d3eca
     348  108 - #2d3eca
     276  156 - #2d3eca
     348  160 - #2d3eca
     404  196 - #ef7142
     320  200 - #7838a0
     236  204 - #ef7040
       4  212 - #597c95
     292  240 - #78389f
     196  244 - #ef6f3f
     380  260 - #ef7040
     252  268 - #78389f
     316  284 - #78389f
     192  300 - #597c95
     404  324 - #ef6f3f
     260  332 - #78389f
     320  348 - #78389f
     196  360 - #ef6f3e
     280  380 - #78389e
     404  384 - #ef6f3f
     592  392 - #597c95
      12  400 - #597c95
     236  404 - #ef6f3e
     348  404 - #78389e
     292  432 - #2d3eca
     252  464 - #2d3eca
     348  468 - #2d3eca
     292  496 - #2d3eca
       4  592 - #597c95
     592  592 - #597c95
";

const HALF: &str = r"
       4    4 - #597c95
     592    4 - #597c95
     252  100 - #2d3eca
     348  108 - #2d3eca
     276  156 - #2d3eca
     348  160 - #2d3eca
     404  196 - #ef7142
     320  200 - #7838a0
     236  204 - #ef7040
       4  212 - #597c95
     292  240 - #78389f
     196  244 - #ef6f3f
     380  260 - #ef7040
     252  268 - #78389f
     316  284 - #78389f
     192  300 - #597c95
     404  324 - #ef6f3f
     260  332 - #78389f
     320  348 - #78389f
     196  360 - #ef6f3e
     280  380 - #78389e
     404  384 - #ef6f3e
     592  392 - #597c95
      12  400 - #597c95
     236  404 - #ef6f3e
     348  404 - #78389e
     292  432 - #2d3eca
     252  464 - #2d3eca
     348  468 - #2d3eca
     292  496 - #2d3eca
       4  592 - #597c95
     592  592 - #597c95
";
