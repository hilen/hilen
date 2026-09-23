mod colors;

use std::f32::consts::{FRAC_PI_4, PI, TAU};

use anyhow::{Result, bail, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Quat, Shape3, Vec3},
    refs::Weak,
    scene::{
        Model, Node, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, Sky, ThirdPerson, Wall, scene,
    },
    ui::Color,
    ui_test::{check_colors, checkpoint, set_record_probe_count},
};

use self::colors::{BEHIND, PULLED_IN, SLID_OUT, THROUGH_WALL, ZOOMED};
use crate::geometry::{WHITE, capsule};

const TOLERANCE: f32 = 1e-3;
/// Frames for the player to land on the floor and the camera to follow.
const SETTLE_FRAMES: usize = 30;
/// Frames an animated move may take before the test gives up on it.
const MOVE_FRAMES: usize = 2000;
/// How fast the figure turns, radians per second of scene time, a half
/// turn in about a second.
const TURN_SPEED: f32 = 3.5;
/// How fast the camera zooms, units per second of scene time.
const ZOOM_SPEED: f32 = 4.0;
const PLAYER: Vec3 = Vec3::new(0.0, 0.9, 1.0);
/// The stone wall stands across the way in front, its near face at z -2.8.
const WALL: Vec3 = Vec3::new(0.0, 1.5, -3.0);
const WALL_FACE: f32 = -2.8;
/// The body sits this far below the capsule's middle, the head above.
const FIGURE_DROP: f32 = 0.25;

/// A player seen over its shoulder, turning slowly so the camera is
/// watched swinging round it. A figure with a face in front and hair
/// behind stands in the player's capsule and faces where it looks, a
/// half clear stone wall ahead and posts around. The figure turns half
/// round, and as the wall comes behind it the camera slides in to stay
/// in front of the stone. It turns back, and the camera slides out again. Then
/// the wall is skipped, the way a game skips a swing's hitbox, and on the same
/// half turn the camera keeps its full distance, passes through the wall
/// and looks back at the figure through it. Last the camera zooms in over
/// the shoulder while the figure turns a little.
#[scene]
#[derive(Default)]
struct ThirdPersonCamera {
    figure:  Weak<Prop>,
    wall:    Weak<Wall>,
    /// The yaw the figure is turning to, none while it stands.
    turn_to: Option<f32>,
    /// The camera distance the zoom is heading for, none while it holds.
    zoom_to: Option<f32>,
}

impl SceneSetup for ThirdPersonCamera {
    fn needs_physics(&self) -> bool {
        true
    }

    fn setup(&mut self) {
        self.sky = Some(Sky::gradient(
            Color::hex("#3a7bd5"),
            Color::hex("#d9e4f0"),
            Color::hex("#5a4a3a"),
        ));
        self.make_node::<Wall>(Shape3::Plane(40.0), Vec3::ZERO)
            .set_color(Color::hex("#6f8f5a"))
            .set_roughness(0.9);

        self.wall = self.make_node::<Wall>(Shape3::cuboid(6.0, 3.0, 0.4), WALL);
        // Half see through, so a camera behind it still shows what it hides.
        self.wall.set_color(Color::hex("#9a938a").with_alpha(0.5)).set_roughness(0.9);

        let posts = [
            (-3.0, 3.0),
            (3.0, 3.0),
            (-3.5, -1.0),
            (3.5, -1.0),
            (-2.0, 6.0),
            (2.5, -6.5),
        ];
        for (x, z) in posts {
            self.make_node::<Prop>(Shape3::cuboid(0.4, 2.0, 0.4), Vec3::new(x, 1.0, z))
                .set_color(Color::hex("#c0703a"))
                .set_roughness(0.7);
        }

        // A body with a head on it, the eyes looking down -z the way a
        // player with no yaw looks.
        let body = Model::from_mesh("Third person figure", capsule(0.33, 1.25, WHITE));
        self.figure = self.make_node::<Prop>(Shape3::Model(body), PLAYER - Vec3::Y * FIGURE_DROP);
        self.figure.set_color(Color::hex("#2f5fa7")).set_roughness(0.6);
        let mut head = self.make_node::<Prop>(Shape3::Ball(0.26), Vec3::new(0.0, 0.85, 0.0));
        head.set_color(Color::hex("#f0c8a0")).attach_to(self.figure.weak_node());
        for x in [-0.09, 0.09] {
            self.make_node::<Prop>(Shape3::Ball(0.05), Vec3::new(x, 0.04, -0.23))
                .set_color(Color::hex("#1a1a1a"))
                .attach_to(head.weak_node());
        }
        // Hair over the back of the head, so from behind it reads as the
        // back and from the front the face shows.
        self.make_node::<Prop>(Shape3::Ball(0.25), Vec3::new(0.0, 0.1, 0.06))
            .set_color(Color::hex("#a8753c"))
            .set_roughness(0.9)
            .attach_to(head.weak_node());

        let player = self.add_player(PLAYER);
        player.keyboard = false;
        player.mouse = false;
        player.third_person = Some(ThirdPerson::default());
    }

    /// Moves the turn and the zoom on, then stands the figure in the
    /// player's capsule facing its yaw.
    fn update(&mut self, dt: f32) {
        let (turn_to, zoom_to) = (self.turn_to, self.zoom_to);
        let Some(player) = self.player.as_mut() else {
            return;
        };
        let mut turned = false;
        if let Some(target) = turn_to {
            player.yaw = step_toward(player.yaw, target, TURN_SPEED * dt);
            turned = player.yaw.to_bits() == target.to_bits();
        }
        let mut zoomed = false;
        if let (Some(target), Some(rig)) = (zoom_to, player.third_person.as_mut()) {
            rig.distance = step_toward(rig.distance, target, ZOOM_SPEED * dt);
            zoomed = rig.distance.to_bits() == target.to_bits();
        }
        let (position, yaw) = (player.position(), player.yaw);
        if turned {
            self.turn_to = None;
        }
        if zoomed {
            self.zoom_to = None;
        }
        self.figure
            .set_position(position - Vec3::Y * FIGURE_DROP)
            .set_rotation(Quat::from_rotation_y(-yaw));
    }
}

/// `from` moved toward `to` by at most `by`, landing on it exactly.
fn step_toward(from: f32, to: f32, by: f32) -> f32 {
    if (to - from).abs() <= by {
        to
    } else {
        from + by.copysign(to - from)
    }
}

/// Where the camera is and where the rig puts it with no wall in the way.
fn camera_and_unclipped(scene: Weak<ThirdPersonCamera>) -> (Vec3, Vec3) {
    from_main(move || {
        let player = scene.player.as_ref().expect("the scene has a player");
        let rig = player.third_person.as_ref().expect("the player is seen over its shoulder");
        let pivot = player.position() + Vec3::Y * rig.pivot_height;
        let expected = rig.place(pivot, player.direction(), player.right(), None);
        (scene.camera.position, expected)
    })
}

fn settle() {
    for _ in 0..SETTLE_FRAMES {
        wait_for_next_frame();
    }
}

/// Starts a turn to `yaw` and a zoom to `distance`, then waits for both
/// to land and the camera to catch up.
fn animate(mut scene: Weak<ThirdPersonCamera>, yaw: Option<f32>, distance: Option<f32>) -> Result<()> {
    from_main(move || {
        scene.turn_to = yaw;
        scene.zoom_to = distance;
    });
    for _ in 0..MOVE_FRAMES {
        wait_for_next_frame();
        if from_main(move || scene.turn_to.is_none() && scene.zoom_to.is_none()) {
            settle();
            return Ok(());
        }
    }
    bail!("the turn or the zoom did not finish in {MOVE_FRAMES} frames")
}

impl SceneTest for ThirdPersonCamera {
    fn perform_test(mut scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);
        settle();

        let (camera, expected) = camera_and_unclipped(scene);
        ensure!(
            camera.distance(expected) < TOLERANCE,
            "the camera is at {camera}, behind the player is {expected}"
        );
        checkpoint("the camera sits behind the figure, the half clear wall ahead")?;
        check_colors(BEHIND)?;

        animate(scene, Some(PI), None)?;
        let (camera, unclipped) = camera_and_unclipped(scene);
        ensure!(
            unclipped.z < WALL_FACE,
            "with no wall the camera would sit in or behind the stone"
        );
        ensure!(
            camera.z > WALL_FACE && camera.z < PLAYER.z,
            "the wall pulls the camera in front of it, it is at {camera}"
        );
        checkpoint("the figure turned half round, the wall behind it slid the camera in close")?;
        check_colors(PULLED_IN)?;

        animate(scene, Some(TAU), None)?;
        let (camera, expected) = camera_and_unclipped(scene);
        ensure!(
            camera.distance(expected) < TOLERANCE,
            "turned back, the camera slides out to {expected}"
        );
        checkpoint("the figure turned back, the camera slid out to its full distance")?;
        check_colors(SLID_OUT)?;

        let wall = from_main(move || scene.wall.weak_node());
        from_main(move || {
            let player = scene.player.as_mut().expect("the scene has a player");
            player.third_person.as_mut().expect("over the shoulder").skip.push(wall);
        });
        animate(scene, Some(TAU + PI), None)?;
        let (camera, expected) = camera_and_unclipped(scene);
        ensure!(
            camera.distance(expected) < TOLERANCE,
            "skipping the wall the camera goes through it to {expected}"
        );
        checkpoint("the wall is skipped, on the same turn the camera passed through it")?;
        check_colors(THROUGH_WALL)?;

        from_main(move || {
            let player = scene.player.as_mut().expect("the scene has a player");
            player.third_person.as_mut().expect("over the shoulder").skip.clear();
        });
        animate(scene, Some(TAU + PI + FRAC_PI_4 * 3.0), Some(2.5))?;
        let (camera, expected) = camera_and_unclipped(scene);
        ensure!(
            camera.distance(expected) < TOLERANCE,
            "zoomed in, the camera is at {camera}, not {expected}"
        );
        checkpoint("the camera zoomed in close over the figure's shoulder")?;
        check_colors(ZOOMED)
    }
}
