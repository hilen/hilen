use std::f32::consts::FRAC_PI_2;

use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    gm::volume::{Quat, Shape3, Vec3},
    refs::Weak,
    scene::{
        GravityWell, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTemplates, SceneTest, Sky, Sun,
        Wall, scene, step_scene,
    },
    ui::Color,
    ui_test::{checkpoint, hold_key, release_key},
    window::KeyCode,
};

const PLANET: f32 = 30.0;
/// The middle of the capsule over the ground, half its height.
const STAND: f32 = 0.9;
/// Frames of walking a quarter round: 47 m at 4 m/s, 11.8 s.
const QUARTER_FRAMES: u32 = 707;

/// A small round planet that pulls 9.81 on its ground, posts standing out
/// of it along a great circle and a player on its top. A held `w` walks
/// the player a quarter of the way round. Its up turns to stay against
/// the pull and the camera's up turns along, so the ground stays under
/// its feet on the screen the whole way.
#[scene]
#[derive(Default)]
struct PlanetWalk {}

impl SceneSetup for PlanetWalk {
    fn needs_physics(&self) -> bool {
        true
    }

    fn setup(&mut self) {
        self.sky = Some(Sky::gradient(
            Color::hex("#3a7bd5"),
            Color::hex("#d9e4f0"),
            Color::hex("#5a4a3a"),
        ));
        self.sun = Sun {
            direction: Vec3::new(-0.3, -1.0, -0.4),
            ..Sun::default()
        };
        self.set_gravity(Vec3::ZERO);
        self.set_wells(vec![GravityWell::with_surface_gravity(Vec3::ZERO, PLANET, 9.81)]);
        self.make_node::<Wall>(Shape3::Ball(PLANET), Vec3::ZERO)
            .set_color(Color::hex("#6f8f5a"))
            .set_roughness(0.9);
        // Posts every 0.2 radians along the way, standing out of the
        // ground, off to the side of the path.
        for step in 1..10_u8 {
            let angle = f32::from(step) * 0.2;
            let turn = Quat::from_rotation_x(-angle);
            let foot = turn * Vec3::new(1.5, PLANET, 0.0);
            self.make_node::<Prop>(Shape3::cuboid(0.3, 2.0, 0.3), foot + turn * Vec3::Y)
                .set_rotation(turn)
                .set_color(Color::hex("#c0703a"))
                .set_roughness(0.7);
        }
        let player = self.add_player(Vec3::new(0.0, PLANET + STAND, 0.0));
        player.mouse = false;
        player.look(0.0, -0.15);
    }
}

impl SceneTest for PlanetWalk {
    fn stepped() -> bool {
        true
    }

    fn perform_test(scene: Weak<Self>) -> Result<()> {
        step_scene(30);
        checkpoint("the player stands on top of the planet")?;

        hold_key(KeyCode::KeyW);
        step_scene(QUARTER_FRAMES);
        release_key(KeyCode::KeyW);
        step_scene(5);

        let (at, up, camera_up, grounded) = from_main(move || {
            let player = scene.player.as_ref().expect("a player");
            (player.position(), player.up(), scene.camera.up, player.grounded())
        });
        let round = at.angle_between(Vec3::Y);
        ensure!(
            (round - FRAC_PI_2).abs() < 0.1,
            "walked {round} radians round, not a quarter"
        );
        ensure!(
            (at.length() - PLANET - STAND).abs() < 0.1,
            "the capsule's middle is {} from the center",
            at.length()
        );
        ensure!(grounded, "the player stands on the ground");
        ensure!(up.angle_between(at) < 0.02, "the up {up} is not against the pull");
        ensure!(
            camera_up.angle_between(up) < 1e-3,
            "the camera's up {camera_up} did not turn with the player"
        );
        checkpoint("a quarter round, the ground still under the player's feet")
    }
}
