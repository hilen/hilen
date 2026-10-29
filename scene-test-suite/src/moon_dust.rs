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
     152   60 - #597c95
       4  164 - #597c95
     388  212 - #7a94a7
     188  216 - #707070
     220  216 - #646464
     160  224 - #707070
     244  228 - #696969
     336  244 - #a4a4a4
     136  252 - #858585
     376  252 - #a2a2a2
     420  252 - #a6a6a6
     248  256 - #898989
     184  260 - #9c9c9c
     276  260 - #5f5f5f
     472  260 - #b8b8b8
     340  280 - #a0a0a0
     288  284 - #424242
     124  288 - #8c8c8c
     376  292 - #a0a0a0
     208  296 - #a0a0a0
     260  296 - #8b8b8b
     308  300 - #435d70
     432  304 - #a5a5a5
     288  312 - #484848
     484  312 - #c4c4c4
     288  316 - #404040
     168  320 - #a0a0a0
     120  328 - #7b7b7b
     252  332 - #8b8b8b
     348  332 - #a0a0a0
     220  336 - #999999
     280  336 - #525252
     408  348 - #a5a5a5
     460  356 - #b7b7b7
     140  360 - #757575
     228  360 - #8a8a8a
     252  364 - #6d6d6d
     172  380 - #6f6f6f
     212  380 - #767676
     368  380 - #afafaf
     192  384 - #6e6e6e
     444  524 - #597c95
       4  592 - #597c95
     292  592 - #597c95
     592  592 - #597c95
";
