use anyhow::Result;
use hilen::{
    dispatch::wait_for_next_frame,
    gm::volume::{Shape3, Vec3},
    refs::Weak,
    scene::{Atmosphere, Camera, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, SpaceSky, scene},
    ui::Color,
    ui_test::{check_colors, set_record_probe_count},
};

const PLANET: f32 = 1000.0;
const AIR: f32 = 50.0;

/// A white ball far past the air of a planet, like a moon, with the sun at
/// its side, so its line to the sun never meets the air. It is lit a clean
/// white. Read from the table of the air, which holds only points inside it,
/// it came out orange.
#[scene]
#[derive(Default)]
struct LightPastTheAir {}

impl SceneSetup for LightPastTheAir {
    fn setup(&mut self) {
        let at = Vec3::new(0.0, 0.0, PLANET * 15.0);
        self.camera = Camera {
            position: at + Vec3::new(0.0, 0.0, 300.0),
            target: at,
            ..Camera::default()
        };
        self.sun.direction = Vec3::new(-1.0, 0.0, 0.0);
        self.ambient = Color::rgb(0.0, 0.0, 0.0);
        self.space_sky = Some(SpaceSky {
            atmosphere: Some(Atmosphere::scaled(Vec3::ZERO, PLANET, AIR)),
            ..SpaceSky::default()
        });
        self.make_node::<Prop>(Shape3::Ball(60.0), at)
            .set_color(Color::rgb(1.0, 1.0, 1.0))
            .set_roughness(1.0);
    }
}

impl SceneTest for LightPastTheAir {
    fn perform_test(_: Weak<Self>) -> Result<()> {
        set_record_probe_count(24);
        wait_for_next_frame();
        check_colors(CLEAN)
    }
}

const CLEAN: &str = r"
       4    4 - #000000
     592    4 - #000000
     312  196 - #565656
     352  212 - #a2a2a2
     304  240 - #303030
     380  240 - #c2c2c2
     340  244 - #898989
       4  256 - #e6e8f1
     400  272 - #d9d9d9
     348  276 - #939393
     324  300 - #6b6b6b
     404  300 - #dddddd
     356  316 - #9f9f9f
     392  328 - #cdcdcd
     324  336 - #6c6c6c
     304  364 - #303030
     336  364 - #848484
     364  380 - #b2b2b2
     440  380 - #e6e8f1
     340  396 - #939393
     312  400 - #555555
     300  404 - #151515
      52  576 - #e6e8f1
     592  592 - #000000
";
