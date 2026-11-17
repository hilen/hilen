mod colors;

use std::f32::consts::FRAC_PI_2;

use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Mat4, Quat, Shape3, Vec3},
    refs::Weak,
    scene::{
        Camera, ColliderShape, Node, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, Sky, Wall,
        scene,
    },
    ui::Color,
    ui_test::{check_colors, checkpoint, set_record_probe_count},
};

use self::colors::{DROPPED, GROWN_UP, STANDING, SWUNG, WALKED_AWAY};

const TOLERANCE: f32 = 1e-3;

/// The body's middle stands this high over the floor at scale 1, the legs
/// reach down to it.
const HIP: f32 = 1.05;
const START: Vec3 = Vec3::new(-0.6, HIP, 0.0);
const WALKED: Vec3 = Vec3::new(1.2, HIP, -0.9);
const GROWN: f32 = 1.4;

/// Turned to the left, the sword arm towards the camera, so the blade and
/// its swing show their whole length.
fn side_on() -> Quat {
    Quat::from_rotation_y(-FRAC_PI_2)
}

// Where each part hangs in its parent's space.
const HEAD: Vec3 = Vec3::new(0.0, 0.72, 0.0);
const SHOULDER: Vec3 = Vec3::new(0.45, 0.3, 0.0);
const ARM: Vec3 = Vec3::new(0.0, -0.3, 0.0);
const HAND: Vec3 = Vec3::new(0.0, -0.35, 0.0);
/// The blade points forward out of the fist, its middle half a blade on.
const SWORD: Vec3 = Vec3::new(0.0, 0.0, 0.55);

/// A knight built from parts hung on parts, the way a game character
/// holds its gear: legs, a head with eyes and two arms on the body, and
/// down the right arm a shoulder, the arm, the hand and a sword in it,
/// with a red hitbox on the blade whose green collider rides along. The
/// knight stands, swings the arm up and the sword and its hitbox follow,
/// turns and walks away and everything follows, grows and every part
/// grows with it, then drops the sword on the ground and walks back
/// without it. At every step the sword sits where the transforms of the
/// chain above it put it, and the hitbox collider is found there.
#[scene]
#[derive(Default)]
struct NodeParenting {
    body:     Weak<Prop>,
    shoulder: Weak<Prop>,
    sword:    Weak<Prop>,
    hitbox:   Weak<Wall>,
}

impl NodeParenting {
    fn part(&mut self, shape: Shape3, at: Vec3, color: &str, parent: Weak<dyn Node>) -> Weak<Prop> {
        let mut part = self.make_node::<Prop>(shape, at);
        part.set_color(Color::hex(color)).set_roughness(0.7).attach_to(parent);
        part
    }
}

impl SceneSetup for NodeParenting {
    fn needs_physics(&self) -> bool {
        true
    }

    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.4, 2.8, 6.2),
            target: Vec3::new(0.3, 0.9, -0.3),
            ..Camera::default()
        };
        self.sky = Some(Sky::gradient(
            Color::hex("#3a7bd5"),
            Color::hex("#d9e4f0"),
            Color::hex("#5a4a3a"),
        ));
        self.show_colliders = true;

        self.make_node::<Wall>(Shape3::Plane(20.0), Vec3::ZERO)
            .set_color(Color::hex("#6f8f5a"))
            .set_roughness(0.9);

        self.body = self.make_node::<Prop>(Shape3::cuboid(0.7, 0.9, 0.4), START);
        self.body
            .set_color(Color::hex("#2f5fa7"))
            .set_roughness(0.6)
            .set_rotation(side_on());
        let body = self.body.weak_node();

        for x in [-0.18, 0.18] {
            self.part(
                Shape3::cuboid(0.25, 0.6, 0.3),
                Vec3::new(x, -0.75, 0.0),
                "#2b2b3a",
                body,
            );
        }
        let head = self.part(Shape3::Ball(0.28), HEAD, "#f0c8a0", body).weak_node();
        for x in [-0.1, 0.1] {
            self.part(Shape3::Ball(0.05), Vec3::new(x, 0.05, 0.25), "#1a1a1a", head);
        }
        self.part(
            Shape3::cuboid(0.18, 0.7, 0.18),
            Vec3::new(-0.45, -0.05, 0.0),
            "#264d88",
            body,
        );

        self.shoulder = self.part(Shape3::Ball(0.13), SHOULDER, "#264d88", body);
        let arm = self.part(
            Shape3::cuboid(0.18, 0.6, 0.18),
            ARM,
            "#264d88",
            self.shoulder.weak_node(),
        );
        let hand = self.part(Shape3::Ball(0.1), HAND, "#f0c8a0", arm.weak_node());
        self.sword = self.part(
            Shape3::cuboid(0.07, 0.05, 1.0),
            SWORD,
            "#d5dbe1",
            hand.weak_node(),
        );
        self.sword.set_metallic(1.0).set_roughness(0.25);
        let sword = self.sword.weak_node();
        self.part(
            Shape3::cuboid(0.32, 0.07, 0.07),
            Vec3::new(0.0, 0.0, -0.45),
            "#8a6d2f",
            sword,
        );

        self.hitbox = self.make_node::<Wall>(Shape3::cuboid(0.26, 0.26, 1.0), Vec3::ZERO);
        self.hitbox.set_color(Color::hex("#e74c3c").with_alpha(0.3)).attach_to(sword);
    }
}

/// Where the sword's middle is, worked out by multiplying the transforms
/// of the chain from the body down, the way a renderer would.
fn sword_by_matrices(body: (Vec3, Quat, f32), shoulder: Quat) -> Vec3 {
    let local = |at: Vec3, turn: Quat| Mat4::from_rotation_translation(turn, at);
    let (at, turn, scale) = body;
    let chain = Mat4::from_scale_rotation_translation(Vec3::splat(scale), turn, at)
        * local(SHOULDER, shoulder)
        * local(ARM, Quat::IDENTITY)
        * local(HAND, Quat::IDENTITY)
        * local(SWORD, Quat::IDENTITY);
    chain.transform_point3(Vec3::ZERO)
}

/// The sword is where the chain puts it, and the hitbox collider rides
/// on it: a speck of a ball at the sword's middle overlaps the hitbox.
fn sword_is_at(scene: Weak<NodeParenting>, expected: Vec3) -> Result<()> {
    let sword = from_main(move || scene.sword.position());
    ensure!(
        sword.distance(expected) < TOLERANCE,
        "the sword is at {sword}, the chain puts it at {expected}"
    );
    hitbox_at(scene, expected)
}

fn hitbox_at(scene: Weak<NodeParenting>, point: Vec3) -> Result<()> {
    let speck = ColliderShape::Ball(0.02);
    let found = from_main(move || scene.overlapping(&speck, point, Quat::IDENTITY, &[]));
    let hitbox = from_main(move || scene.hitbox.weak_node());
    ensure!(
        found.contains(&hitbox),
        "the hitbox collider rides on the sword at {point}"
    );
    Ok(())
}

impl SceneTest for NodeParenting {
    fn perform_test(mut scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);
        wait_for_next_frame();

        sword_is_at(scene, sword_by_matrices((START, side_on(), 1.0), Quat::IDENTITY))?;
        checkpoint("the knight stands holding the sword, its red hitbox on the blade")?;
        check_colors(STANDING)?;

        let swing = Quat::from_rotation_x(-1.4);
        from_main(move || {
            scene.shoulder.set_rotation(swing);
        });
        wait_for_next_frame();
        sword_is_at(scene, sword_by_matrices((START, side_on(), 1.0), swing))?;
        checkpoint("the arm swings up at the shoulder, the hand, sword and hitbox follow it")?;
        check_colors(SWUNG)?;

        let turn = Quat::from_rotation_y(-FRAC_PI_2 / 2.0);
        from_main(move || {
            scene.body.set_position(WALKED).set_rotation(turn);
        });
        wait_for_next_frame();
        sword_is_at(scene, sword_by_matrices((WALKED, turn, 1.0), swing))?;
        checkpoint("the knight turns and walks away, every part follows the body")?;
        check_colors(WALKED_AWAY)?;

        let tall = Vec3::new(WALKED.x, HIP * GROWN, WALKED.z);
        from_main(move || {
            scene.body.set_scale(GROWN).set_position(tall);
        });
        wait_for_next_frame();
        sword_is_at(scene, sword_by_matrices((tall, turn, GROWN), swing))?;
        let size = from_main(move || scene.sword.world_scale());
        ensure!(
            (size - GROWN).abs() < TOLERANCE,
            "the sword grows with the knight, it is {size}"
        );
        checkpoint("the knight grows, every part grows with it, the sword and hitbox too")?;
        check_colors(GROWN_UP)?;

        let (held, after, size) = from_main(move || {
            let held = scene.sword.position();
            scene.sword.detach();
            (held, scene.sword.position(), scene.sword.world_scale())
        });
        ensure!(
            after.distance(held) < TOLERANCE,
            "letting go leaves the sword where it was"
        );
        ensure!(
            (size - GROWN).abs() < TOLERANCE,
            "letting go keeps the sword's size, it is {size}"
        );

        let dropped = Vec3::new(2.3, 0.04, 0.6);
        // Laid flat on the grass, the blade lies along its own length.
        from_main(move || {
            scene
                .sword
                .set_scale(1.0)
                .set_position(dropped)
                .set_rotation(Quat::from_rotation_y(0.6));
            scene.body.set_scale(1.0).set_position(START).set_rotation(side_on());
            scene.shoulder.set_rotation(Quat::IDENTITY);
        });
        wait_for_next_frame();
        hitbox_at(scene, dropped)?;
        checkpoint("the knight dropped the sword on the ground and walked back without it")?;
        check_colors(DROPPED)
    }
}
