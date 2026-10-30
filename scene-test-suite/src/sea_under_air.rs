use anyhow::Result;
use hilen::{
    dispatch::wait_for_next_frame,
    gm::volume::{Quat, Shape3, Vec3},
    refs::Weak,
    scene::{
        Atmosphere, Camera, NodeTemplates, Occluder, Prop, SceneCreation, SceneSetup, SceneTest, SpaceSky,
        Sun, scene,
    },
    ui::Color,
    ui_test::{check_colors, set_record_probe_count},
};

const PLANET: f32 = 20_000.0;
const AIR: f32 = 5000.0;
const CENTER: Vec3 = Vec3::new(0.0, -PLANET, 0.0);
const SEA: &str = "#1c3f73";

/// A dark blue sea round a whole planet of 20 km under 5 km of air, seen
/// from 60 km out. The sea is a ball at the ground radius of the air,
/// drawn as flat triangles about 4 km wide that sag up to 100 m under
/// the true ball. A raft of the same blue floats on the sea where the
/// camera looks, its top over the ball. The haze over both is the air
/// between the camera and them, so the sea round the raft has the same
/// dark blue as the raft. A sea lifted to pale lilac by the light of the
/// air's own ground, which the sky draws where a ray ends on the ball,
/// is the bug this test pins.
#[scene]
#[derive(Default)]
struct SeaUnderAir {}

impl SceneSetup for SeaUnderAir {
    fn setup(&mut self) {
        let eye = Vec3::new(0.0, 0.0, 60_000.0);
        self.camera = Camera {
            position: eye,
            target: CENTER,
            far: f32::INFINITY,
            ..Camera::default()
        };
        self.space_sky = Some(SpaceSky {
            atmosphere: Some(Atmosphere::scaled(CENTER, PLANET, AIR)),
            ..SpaceSky::default()
        });
        self.occluders = vec![Occluder::new(CENTER, PLANET)];
        self.sun = Sun {
            direction: -Vec3::new(0.5, 0.6, 1.0).normalize(),
            intensity: 1.0,
            ..Sun::default()
        };
        self.make_node::<Prop>(Shape3::Ball(PLANET), CENTER)
            .set_color(Color::hex(SEA))
            .set_roughness(0.9);
        let up = (eye - CENTER).normalize();
        self.make_node::<Prop>(
            Shape3::cuboid(5000.0, 40.0, 5000.0),
            CENTER + up * (PLANET + 120.0),
        )
        .set_rotation(Quat::from_rotation_arc(Vec3::Y, up))
        .set_color(Color::hex(SEA))
        .set_roughness(0.9);
    }
}

impl SceneTest for SeaUnderAir {
    fn perform_test(_: Weak<Self>) -> Result<()> {
        set_record_probe_count(48);
        wait_for_next_frame();
        check_colors(SEA_AND_RAFT)
    }
}

/// The sea and the raft on it the same dark blue under the haze.
const SEA_AND_RAFT: &str = r"
      592    4 - #000000
      140   52 - #e6e8f1
      296  108 - #091322
      340  124 - #3c537e
      228  128 - #1b2a48
      252  128 - #4d6690
      272  128 - #72889f
      316  128 - #657795
      384  128 - #e1e0ea
      388  132 - #0e1a2d
        4  136 - #000000
      208  136 - #15233c
      360  136 - #7089a4
      592  152 - #000000
      436  168 - #0e192c
      180  172 - #677b94
      400  184 - #354c75
      448  208 - #708aa6
      148  212 - #63738a
      352  224 - #304670
      140  228 - #627186
      128  240 - #314168
      472  240 - #2c3f66
      468  268 - #4b668e
      488  292 - #111d32
      124  300 - #505e75
      292  300 - #2d436d
      472  312 - #8188a0
      168  316 - #273858
      380  316 - #30466f
      472  344 - #4c6691
      192  360 - #273858
      128  364 - #263250
      468  368 - #2f446c
      448  376 - #3c547c
      136  388 - #1a2640
      208  396 - #273858
      436  408 - #6e86a1
      164  416 - #3e4c66
      192  436 - #505f76
      408  436 - #6d849e
      376  456 - #6d829a
      216  460 - #344467
      344  468 - #6b7f96
      244  476 - #1e2c4c
      296  476 - #576c8b
      568  580 - #e6e8f1
        4  592 - #000000
";
