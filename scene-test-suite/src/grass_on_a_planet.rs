use std::f32::consts::FRAC_PI_2;

use anyhow::Result;
use hilen::{
    dispatch::wait_for_next_frame,
    gm::volume::{Quat, Shape3, Vec3},
    refs::Weak,
    scene::{
        Camera, Grass, GrassSpot, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, Sky, Wind, scene,
    },
    ui::Color,
    ui_test::{check_colors, set_record_probe_count},
};

const PLANET: f32 = 30.0;

/// The ground of a ball planet seen from a field whose y points out of
/// the planet at its origin on the surface: the curve as heights, and
/// the normal, both in the field's axes.
fn planet_ground(x: f32, z: f32) -> Option<GrassSpot> {
    let level = PLANET * PLANET - x * x - z * z;
    (level > 0.0).then(|| {
        let up = level.sqrt();
        GrassSpot {
            height: up - PLANET,
            normal: Vec3::new(x, up, z) / PLANET,
            tint:   Color::hex("#ffffff"),
        }
    })
}

/// A field of grass on the side of a small planet, where the planet's up
/// points along the world's x. The field's own axes sit there, turned so
/// its y points out of the planet, and `ground` gives the curve as
/// heights. The blades stand straight out of the side, sideways in the
/// world, and follow the curve away from the camera. No wind, so the
/// picture holds still.
#[scene]
#[derive(Default)]
struct GrassOnAPlanetSide {}

impl SceneSetup for GrassOnAPlanetSide {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(PLANET + 6.0, 1.0, 9.0),
            target: Vec3::new(PLANET, 0.0, 0.0),
            ..Camera::default()
        };
        self.sky = Some(Sky::gradient(
            Color::hex("#3a7bd5"),
            Color::hex("#d9e4f0"),
            Color::hex("#5a4a3a"),
        ));
        self.sun.direction = Vec3::new(-1.0, -0.3, -0.4);
        self.make_node::<Prop>(Shape3::Ball(PLANET), Vec3::ZERO)
            .set_color(Color::hex("#6b5a3a"))
            .set_roughness(0.9);
        let mut grass = Grass::new(Grass::blade("planet_grass_blade", 0.6, 0.16), planet_ground);
        grass.material.color = Color::hex("#4f8a2e");
        grass.spots_per_frame = u32::MAX;
        grass.sway_distance = 0.0;
        grass.origin = Vec3::new(PLANET, 0.0, 0.0);
        grass.turn = Quat::from_rotation_z(-FRAC_PI_2);
        self.grass.push(grass);
        self.wind = Wind {
            direction: Vec3::Z,
            strength:  0.0,
        };
    }
}

impl SceneTest for GrassOnAPlanetSide {
    fn perform_test(_: Weak<Self>) -> Result<()> {
        set_record_probe_count(48);
        for _ in 0..3 {
            wait_for_next_frame();
        }
        check_colors(ON_THE_SIDE)
    }
}

/// The blades standing out of the planet's side along the world's x.
const ON_THE_SIDE: &str = r"
       8    4 - #786b52
     592    4 - #b9cae7
     240   12 - #4b7b3c
     340   16 - #756951
      84   32 - #599444
     468   52 - #bfcee8
     160   72 - #385533
     296   76 - #776a51
     372   96 - #548c42
     584  124 - #cdd9ea
     224  136 - #3a5933
       4  152 - #34502d
     308  152 - #599543
     104  164 - #786b51
     480  188 - #d3ddeb
     552  188 - #d3deeb
     384  196 - #385634
     256  200 - #344f2d
     184  224 - #776a50
     592  244 - #d5e0ec
      44  260 - #4d7f3e
     120  276 - #786a50
     416  300 - #569043
     300  308 - #457035
     516  308 - #d4deea
     164  340 - #395934
     100  356 - #5b9844
     244  368 - #76694e
     592  372 - #d0dae5
      44  388 - #4f833c
     400  412 - #5c9b46
     272  444 - #5d9d46
      92  448 - #36522f
     168  460 - #36522f
     336  488 - #3f6434
     120  520 - #5c9b46
       4  524 - #5d9d47
     476  528 - #afb5bc
     272  536 - #74664c
     360  560 - #a5a8ae
      92  588 - #75674c
     512  588 - #a4a8ad
     200  592 - #74664b
     356  592 - #9ea0a4
     360  592 - #9ea0a5
     364  592 - #9ea0a5
     440  592 - #a0a3a8
     580  592 - #a7abb1
";
