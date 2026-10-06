use std::sync::Arc;

use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    gm::{
        color::U8Color,
        volume::{Mat4, Shape3, Vec3},
    },
    refs::Weak,
    scene::{
        Body, Camera, CoefficientCombineRule, ColliderShape, Heightfield, MeshData, Model, Node,
        NodeTemplates, SceneCreation, SceneSetup, SceneTest, Sky, Wall, scene, step_scene,
    },
    ui::Color,
    ui_test::checkpoint,
};

use crate::geometry::{Frustum, WHITE, append, capsule};

/// Frames for the bodies to fall onto the hills and come to rest.
const SETTLE_FRAMES: u32 = 300;
/// How far a resting body's lowest point may sit above the ground under
/// its middle. A round bottom on a slope touches a little uphill of it.
const REST_ABOVE: f32 = 0.2;
/// How far it may sink below, the contact skin rapier keeps.
const REST_BELOW: f32 = 0.03;

const CAPSULE_RADIUS: f32 = 0.3;
const CAPSULE_HEIGHT: f32 = 1.4;
const CYLINDER_RADIUS: f32 = 0.35;
const CYLINDER_HEIGHT: f32 = 0.8;
const BALL: f32 = 0.45;
const CUBE: f32 = 0.7;

/// Hills in a shallow bowl.
fn ground(x: f32, z: f32) -> f32 {
    0.012 * (x * x + z * z) + 0.35 * (0.8 * x + 0.4).sin() * (0.7 * z).cos()
}

/// Bodies of four collider shapes dropped onto rolling ground, every
/// collider drawn as its green wireframe. The ground is a mesh built from
/// a heightfield and collides as that heightfield, so the bodies rest on
/// the hills where they are drawn, not on the box around them. The
/// capsule and the cylinder are meshes in code with a capsule and a
/// cylinder collider put on, held upright the way a walking character
/// is, the ball rolls into a hollow and the cube tumbles.
#[scene]
#[derive(Default)]
struct ColliderShapes {
    bodies: Vec<(Weak<Body>, f32)>,
}

impl ColliderShapes {
    fn drop(&mut self, shape: Shape3, at: (f32, f32), color: &str) -> Weak<Body> {
        let mut body = self.make_node::<Body>(shape, Vec3::new(at.0, 3.0, at.1));
        body.set_color(Color::hex(color))
            .set_roughness(0.6)
            .set_restitution(0.0, CoefficientCombineRule::Min)
            .set_damping(0.4, 1.5);
        body
    }
}

impl SceneSetup for ColliderShapes {
    fn needs_physics(&self) -> bool {
        true
    }

    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 6.5, 10.5),
            target: Vec3::new(0.3, 0.2, -0.5),
            ..Camera::default()
        };
        self.sky = Some(Sky::gradient(
            Color::hex("#3a7bd5"),
            Color::hex("#d9e4f0"),
            Color::hex("#5a4a3a"),
        ));
        self.show_colliders = true;

        let field = Heightfield::from_fn(81, 81, 16.0, 16.0, ground);
        let terrain = Model::from_mesh("Collider shapes ground", field.mesh());
        self.make_node::<Wall>(Shape3::Model(terrain), Vec3::ZERO)
            .set_collider(ColliderShape::Heightfield(Arc::new(field)), Vec3::ZERO)
            .set_restitution(0.0, CoefficientCombineRule::Min)
            .set_color(Color::hex("#6b9a55"))
            .set_roughness(0.9);

        let capsule_mesh = Model::from_mesh(
            "Collider shapes capsule",
            capsule(CAPSULE_RADIUS, CAPSULE_HEIGHT, WHITE),
        );
        let mut standing = self.drop(Shape3::Model(capsule_mesh), (1.5, 3.4), "#e67e22");
        standing.set_collider(
            ColliderShape::Capsule {
                radius: CAPSULE_RADIUS,
                height: CAPSULE_HEIGHT,
            },
            Vec3::ZERO,
        );
        standing.lock_rotations();
        self.bodies.push((standing, CAPSULE_HEIGHT / 2.0));

        let cylinder_mesh = Model::from_mesh(
            "Collider shapes cylinder",
            cylinder(CYLINDER_RADIUS, CYLINDER_HEIGHT, WHITE),
        );
        let mut drum = self.drop(Shape3::Model(cylinder_mesh), (1.5, -3.6), "#8e44ad");
        drum.set_collider(
            ColliderShape::Cylinder {
                radius: CYLINDER_RADIUS,
                height: CYLINDER_HEIGHT,
            },
            Vec3::ZERO,
        );
        drum.lock_rotations();
        self.bodies.push((drum, CYLINDER_HEIGHT / 2.0));

        let ball = self.drop(Shape3::Ball(BALL), (-2.4, 0.0), "#3498db");
        self.bodies.push((ball, BALL));

        let cube = self.drop(Shape3::cube(CUBE), (4.6, 0.4), "#c0392b");
        // A tumbled cube rests on a face or an edge, its middle at least
        // half a side up.
        self.bodies.push((cube, CUBE / 2.0));
    }
}

impl SceneTest for ColliderShapes {
    fn stepped() -> bool {
        true
    }

    fn perform_test(scene: Weak<Self>) -> Result<()> {
        step_scene(SETTLE_FRAMES);

        let rests = from_main(move || {
            scene
                .bodies
                .iter()
                .map(|(body, below)| (body.position(), *below))
                .collect::<Vec<_>>()
        });
        for (position, below) in rests {
            let floor = ground(position.x, position.z);
            let lowest = position.y - below;
            ensure!(
                lowest > floor - REST_BELOW && lowest < floor + REST_ABOVE,
                "a body at {position} rests with its bottom at {lowest}, the ground under it is at {floor}"
            );
        }

        checkpoint("every body rests on the hills, its collider drawn around it")
    }
}

/// A flat sided cylinder standing on y around its middle, capped.
fn cylinder(radius: f32, height: f32, color: U8Color) -> MeshData {
    let shape = Frustum {
        sides: 20,
        bottom: radius,
        top: radius,
        height,
    };
    let mut mesh = MeshData::default();
    append(
        &mut mesh,
        &shape.build(|_| color, Some(color), Some(color)),
        Mat4::from_translation(Vec3::NEG_Y * height / 2.0),
    );
    mesh
}
