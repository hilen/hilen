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
     284    4 - #776a52
     564    4 - #b9c9e7
     100   16 - #786b52
     192   24 - #355030
     392   52 - #a4c0c0
     160   96 - #786b52
     284   96 - #776a51
      72  116 - #599544
     404  144 - #6d9569
       4  152 - #34502d
     416  168 - #94b797
     580  180 - #d3ddeb
     188  184 - #406638
     492  188 - #d3deeb
     100  200 - #786b51
     292  204 - #375434
     416  228 - #b2c5c0
     424  260 - #b1c5c0
     524  260 - #d5e0ec
      40  264 - #786a50
     140  288 - #36542d
     420  288 - #77a46b
     300  308 - #457035
     204  316 - #5c9b47
     592  328 - #d3dee9
     368  352 - #436b38
     496  372 - #cfd9e4
     288  392 - #518740
      52  400 - #776a4f
     188  408 - #638f47
     404  424 - #75a668
     572  460 - #c1c9d2
     112  464 - #48773a
     396  464 - #809781
     280  484 - #385632
       4  524 - #5d9d47
     408  536 - #abb0b7
     136  548 - #5b9944
     476  556 - #a9aeb4
     360  560 - #a5a8ae
     220  588 - #3e6334
      64  592 - #3f6432
     288  592 - #538a40
     356  592 - #9da0a4
     360  592 - #9ea0a5
     364  592 - #9ea0a5
     592  592 - #a7acb2
";
