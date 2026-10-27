mod checks;
mod ground;
mod meshes;

use std::collections::HashMap;

use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Quat, Shape3, Vec3},
    refs::Weak,
    scene::{
        DayNightSky, Fog, Model, Motion, Node, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest,
        ThirdPerson, scene,
    },
    ui::{Color, Keys},
    ui_test::check_colors,
    window::KeyCode,
};

use self::{
    checks::CHECKS,
    ground::{Chunk, hills, tree_shapes},
};

const SHADOW_DISTANCE: f32 = 80.0;
/// Up from the horizon, in radians.
const SUN_HEIGHTS: [f32; 2] = [0.7, 0.35];
/// The jet cruises at this speed, and on the afterburner of shift at the
/// other, in m/s.
const CRUISE: f32 = 120.0;
const AFTERBURNER: f32 = 300.0;
/// The jet flies this high over the ground at the start of each stop.
const ALTITUDE: f32 = 25.0;
/// Where the test flies the jet to, far apart so new ground streams in,
/// with the way it looks, yaw and pitch.
const STOPS: [(f32, f32, f32, f32); 4] = [
    (900.0, 800.0, 4.0, -0.5),
    (0.0, 0.0, 4.0, -0.5),
    (3000.0, 2500.0, 4.0, -0.5),
    (15000.0, 12000.0, 4.0, -0.5),
];

/// The ground of a streaming world as heavy as geblings, without end:
/// chunks of steep bumpy ground come and go around a flying jet, 10 trees
/// of 32 shared shapes and 4 rocks in each, physics walls like the game,
/// over the coarse ground and a sea 2400 m wide that follow the jet, under
/// the sky, the sun with shadows, the light and the fog of geblings. In
/// presentation the jet flies: `w` where it looks, space up, ctrl down,
/// shift the afterburner, a drag turns it. The test flies it to 4 places
/// far apart under 2 suns. The ground and the trees show their shadows
/// with no dots and no stripes everywhere.
#[scene]
#[derive(Default)]
struct TerrainShadows {
    shapes:     Vec<Weak<Model>>,
    chunks:     HashMap<(i32, i32), Chunk>,
    far:        Option<(Weak<dyn Node>, Vec3)>,
    far_builds: u32,
    sea:        Option<Weak<dyn Node>>,
    jet:        Option<Weak<Prop>>,
}

fn sun(height: f32) -> Vec3 {
    Vec3::new(
        height.cos() * 0.6_f32.sin(),
        -height.sin(),
        height.cos() * -0.6_f32.cos(),
    )
}

impl TerrainShadows {
    /// The sun at `height` with the sky and the light of geblings by day.
    fn light(&mut self, height: f32) {
        let sun = sun(height);
        self.sun.direction = sun;
        self.day_night_sky = Some(DayNightSky {
            sun:       -sun,
            moon:      sun,
            night:     0.0,
            sunset:    (1.0 - height * 2.5).clamp(0.0, 1.0),
            twinkle:   0.0,
            star_seed: 0.0,
        });
    }
}

impl SceneSetup for TerrainShadows {
    fn needs_physics(&self) -> bool {
        true
    }

    fn setup(&mut self) {
        self.light(SUN_HEIGHTS[0]);
        self.sun.shadows = true;
        self.sun.shadow_map_size = 2048;
        self.sun.shadow_distance = SHADOW_DISTANCE;
        self.ambient = Color::rgb(0.42, 0.42, 0.42);
        self.fog = Some(Fog::new(Color::rgb(0.68, 0.78, 0.88), 300.0, 1100.0));
        self.camera.far = 1300.0;
        self.shapes = tree_shapes();

        let start = Vec3::new(0.0, hills(0.0, 0.0) + ALTITUDE, 0.0);
        self.stream(start);
        let player = self.add_player_sized(start, 0.4, 1.8);
        player.motion = Motion::Fly;
        player.speed = CRUISE;
        player.pitch = STOPS[0].3;
        player.third_person = Some(ThirdPerson {
            pivot_height: 1.5,
            distance: 18.0,
            shoulder: 0.0,
            ..ThirdPerson::default()
        });
        let mut jet = self.make_node::<Prop>(
            Shape3::Model(Model::from_mesh("Terrain shadows jet", meshes::jet())),
            start,
        );
        jet.set_color(Color::rgb(1.0, 1.0, 1.0)).set_roughness(0.4).set_metallic(0.6);
        self.jet = Some(jet);
    }

    fn update(&mut self, _: f32) {
        let Some(player) = self.player.as_mut() else {
            return;
        };
        player.speed = if Keys::held(KeyCode::ShiftLeft) || Keys::held(KeyCode::ShiftRight) {
            AFTERBURNER
        } else {
            CRUISE
        };
        let (at, yaw, pitch) = (player.position(), player.yaw, player.pitch);
        if let Some(mut jet) = self.jet {
            jet.set_position(at)
                .set_rotation(Quat::from_rotation_y(-yaw) * Quat::from_rotation_x(pitch));
        }
        self.stream(at);
    }
}

impl SceneTest for TerrainShadows {
    fn perform_test(mut scene: Weak<Self>) -> Result<()> {
        for (i, height) in SUN_HEIGHTS.into_iter().enumerate() {
            for (stop, (x, z, yaw, pitch)) in STOPS.into_iter().enumerate() {
                from_main(move || {
                    scene.light(height);
                    if let Some(player) = scene.player.as_mut() {
                        player.teleport(Vec3::new(x, hills(x, z) + ALTITUDE, z));
                        player.yaw = yaw;
                        player.pitch = pitch;
                    }
                });
                for _ in 0..3 {
                    wait_for_next_frame();
                }
                check_colors(CHECKS[i * STOPS.len() + stop])?;
            }
        }
        Ok(())
    }
}
