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
     340  124 - #3c537e
     252  128 - #4d6690
     272  128 - #72889f
     296  128 - #546e93
     384  128 - #e1e0ea
     236  132 - #435b86
     364  132 - #415985
       4  136 - #000000
     212  148 - #677e99
     380  152 - #4d6991
     420  152 - #0b1627
     168  160 - #091222
     408  176 - #49658f
     428  192 - #4d6992
     144  196 - #182540
     460  208 - #203050
     132  216 - #16233d
     328  220 - #3b557f
     240  236 - #375179
     128  240 - #314168
     472  240 - #2c3f66
     472  276 - #738ba6
     112  288 - #141f37
     168  304 - #364d71
     324  308 - #324b77
     472  312 - #8188a0
     120  336 - #263352
     216  336 - #354d73
     472  348 - #435c88
     460  368 - #6e87a3
     136  384 - #222e4b
     468  388 - #0c1728
     288  392 - #365076
     224  396 - #364d72
     436  408 - #6e86a1
     168  420 - #404e68
     396  444 - #71859c
     188  448 - #1b2745
     216  460 - #344467
     344  468 - #6b7f96
     368  468 - #2f436b
     248  476 - #243356
     296  484 - #213052
     568  580 - #e6e8f1
       4  592 - #000000
     152  592 - #000000
";
