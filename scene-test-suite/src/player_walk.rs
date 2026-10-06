use anyhow::Result;
use hilen::{
    dispatch::from_main,
    gm::volume::{Shape3, Vec3},
    refs::{Weak, manage::DataManager},
    scene::{
        Body, Light, Material, Node, NodeTemplates, Player, SceneCreation, SceneSetup, SceneTest, Wall,
        scene, step_scene,
    },
    ui::{Color, Image},
    ui_test::{capture_screenshot, checkpoint, hold_key, release_key},
    window::KeyCode,
};

/// Frames of walking, two seconds, more than the wall is away.
const WALK_FRAMES: u32 = 120;
const SETTLE_FRAMES: u32 = 20;
const CRATE_START: Vec3 = Vec3::new(0.6, 0.5, 1.5);

/// A first person player walks forward into a crate and on into a brick
/// wall. The crate has to be shoved aside, the wall has to stop the
/// capsule at its surface, a jump has to lift the player and gravity
/// bring it back, and the camera has to look out of the player's eyes,
/// so the last frame is the wall up close with its bricks in relief
/// under the grazing lamp.
#[scene]
#[derive(Default)]
struct PlayerWalk {
    crate_box: Weak<Body>,
}

impl SceneSetup for PlayerWalk {
    fn needs_physics(&self) -> bool {
        true
    }

    fn setup(&mut self) {
        self.lights
            .push(Light::point(Vec3::new(-2.5, 1.6, -2.3)).intensity(3.0).range(8.0));

        self.make_node::<Wall>(Shape3::Plane(20.0), Vec3::ZERO)
            .set_color(Color::hex("#8d9aa5"))
            .set_roughness(0.9);

        let bricks = Material {
            color:        Color::hex("#ffffff"),
            metallic:     0.0,
            roughness:    0.8,
            texture:      Some(Image::get("bricks.jpg")),
            normal_map:   Some(Image::get("bricks_normal.jpg")),
            normal_scale: 1.5,
            emissive:     0.0,
            cutout:       None,
        };
        for x in [-2.0, 0.0, 2.0] {
            self.make_node::<Wall>(Shape3::cuboid(2.0, 2.0, 0.5), Vec3::new(x, 1.0, -3.0))
                .set_material(bricks);
        }

        let mut crate_box = self.make_node::<Body>(Shape3::cube(1.0), CRATE_START);
        crate_box.set_color(Color::hex("#ffffff")).set_roughness(0.7);
        crate_box.material.texture = Some(Image::get("crate_box.png"));
        self.crate_box = crate_box;

        self.add_player(Vec3::new(0.0, 1.0, 4.0));
    }
}

impl SceneTest for PlayerWalk {
    fn stepped() -> bool {
        true
    }

    fn perform_test(scene: Weak<Self>) -> Result<()> {
        let player = |scene: Weak<Self>| from_main(move || scene.player.as_ref().map(Player::position));

        // Spawned a little above the floor, the player settles onto it
        // first.
        step_scene(SETTLE_FRAMES);
        let start = player(scene).expect("the scene has a player");

        hold_key(KeyCode::KeyW);
        step_scene(WALK_FRAMES);
        release_key(KeyCode::KeyW);

        let stopped = player(scene).expect("the scene has a player");
        let crate_box = from_main(move || scene.crate_box.position());
        // The shove deflects the capsule sideways a little, the wall is
        // what stops it.
        anyhow::ensure!(
            stopped.z > -2.5 && stopped.z < -2.2 && stopped.x.abs() < 0.8,
            "the wall did not stop the player, it is at {stopped:?}"
        );
        anyhow::ensure!(
            (stopped.y - start.y).abs() < 0.05,
            "the player left the floor while walking, it is at {stopped:?}"
        );
        anyhow::ensure!(
            crate_box.z < 1.0 && crate_box.x > CRATE_START.x,
            "the player did not shove the crate, it is at {crate_box:?}"
        );

        hold_key(KeyCode::Space);
        step_scene(1);
        release_key(KeyCode::Space);
        step_scene(6);
        let airborne = player(scene).expect("the scene has a player");
        anyhow::ensure!(
            airborne.y > stopped.y + 0.15,
            "the jump did not lift the player, it is at {airborne:?}"
        );
        step_scene(90);
        let landed = player(scene).expect("the scene has a player");
        anyhow::ensure!(
            (landed.y - stopped.y).abs() < 0.05,
            "the player did not land, it is at {landed:?}"
        );

        let (camera, eye) = from_main(move || {
            let player = scene.player.as_ref().expect("the scene has a player");
            (
                scene.camera.position,
                player.position() + Vec3::Y * player.eye_height,
            )
        });
        anyhow::ensure!(
            (camera - eye).length() < 1e-4,
            "the camera is at {camera:?}, the eye at {eye:?}"
        );

        capture_screenshot()?;
        checkpoint("the player stands at the brick wall and looks out of its eyes")
    }
}
