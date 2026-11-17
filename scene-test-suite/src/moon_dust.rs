use anyhow::Result;
use hilen::{
    dispatch::wait_for_next_frame,
    gm::volume::{Shape3, Vec3},
    refs::Weak,
    scene::{Camera, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, Sun, scene},
    ui::Color,
    ui_test::{check_colors, set_record_probe_count},
};

/// 2 grey balls in space under a sun straight behind the camera, 2 full
/// moons. The left one reflects by Lambert's law and darkens toward its
/// edge. The right one is `dust` and stays flat and bright to its edge,
/// the way the real full moon looks. Their middles, facing the light and
/// the eye, are equally bright.
#[scene]
#[derive(Default)]
struct MoonDust {}

impl SceneSetup for MoonDust {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 0.0, 12.0),
            target: Vec3::ZERO,
            ..Camera::default()
        };
        self.ambient = Color::hex("#000000");
        self.sun = Sun {
            direction: Vec3::NEG_Z,
            intensity: 1.0,
            ..Sun::default()
        };
        for (x, dust) in [(-2.2, false), (2.2, true)] {
            self.make_node::<Prop>(Shape3::Ball(2.0), Vec3::new(x, 0.0, 0.0))
                .set_color(Color::hex("#a0a0a0"))
                .set_roughness(1.0)
                .set_dust(dust);
        }
    }
}

impl SceneTest for MoonDust {
    fn perform_test(_: Weak<Self>) -> Result<()> {
        set_record_probe_count(48);
        wait_for_next_frame();
        check_colors(FULL_MOONS)
    }
}

/// The Lambert ball dark at its edge, the dusty one bright to its edge.
const FULL_MOONS: &str = r"
       4    4 - #597c95
     300    4 - #597c95
     592    4 - #597c95
     152   64 - #597c95
     208  216 - #6f6f6f
     388  216 - #afafaf
     176  228 - #868686
     448  232 - #b6b6b6
     144  236 - #767676
     248  244 - #808080
     216  248 - #949494
     164  252 - #979797
     188  260 - #9c9c9c
     124  264 - #7e7e7e
     348  264 - #a1a1a1
     260  272 - #878787
     152  276 - #9c9c9c
     228  284 - #9b9b9b
     288  284 - #424242
     484  284 - #c7c7c7
     484  288 - #c3c3c3
     116  296 - #7f7f7f
     288  296 - #525252
     260  300 - #8c8c8c
     288  300 - #545454
     300  300 - #597c95
     400  300 - #a2a2a2
     288  308 - #4f4f4f
     152  316 - #9d9d9d
     288  316 - #404040
     484  316 - #c7c7c7
     244  320 - #949494
     216  324 - #9d9d9d
     284  324 - #545454
     120  328 - #7b7b7b
     348  336 - #a1a1a1
     172  344 - #9a9a9a
     276  344 - #505050
     208  352 - #959595
     140  356 - #7f7f7f
     448  368 - #b8b8b8
     244  372 - #666666
     176  380 - #737373
     212  384 - #696969
     388  384 - #b1b1b1
       4  592 - #597c95
     300  592 - #597c95
     592  592 - #597c95
";
