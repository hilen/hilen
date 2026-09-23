use std::sync::Arc;

use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::{
        color::U8Color,
        volume::{Mat4, Shape3, Vec3},
    },
    refs::Weak,
    scene::{
        Body, Camera, CoefficientCombineRule, ColliderShape, Heightfield, MeshData, Model, Node,
        NodeTemplates, SceneCreation, SceneSetup, SceneTest, Sky, Wall, scene,
    },
    ui::Color,
    ui_test::{check_colors, set_record_probe_count},
};

use crate::geometry::{Frustum, WHITE, append, capsule};

/// Frames for the bodies to fall onto the hills and come to rest.
const SETTLE_FRAMES: usize = 300;
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
    fn perform_test(scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);

        for _ in 0..SETTLE_FRAMES {
            wait_for_next_frame();
        }

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

        check_colors(RESTING)
    }
}

const RESTING: &str = r"
       4    4 - #d5e0ec
     312    4 - #d5e0ec
     592    4 - #d5e0ec
     160   16 - #d5e0ec
     452   40 - #d5dfeb
       8  116 - #d2dde8
     112  164 - #66eaa0
     400  180 - #64e89d
     408  180 - #96dcbc
     420  180 - #96dcbc
     232  188 - #94daba
     240  188 - #31f37e
     528  196 - #00ff60
      80  220 - #6fa56e
     560  220 - #c3ccd5
     472  224 - #6ba06c
     580  224 - #c3ccd5
       4  228 - #c3cbd5
     336  228 - #8c57cd
     592  228 - #c3cbd5
     336  232 - #8c56cd
     340  232 - #8c56cd
     144  236 - #699d6a
     328  236 - #684095
     348  236 - #8b52b8
     576  236 - #c1c9d2
     332  240 - #7944a6
     336  244 - #8248af
     348  244 - #8b54b9
     592  244 - #c0c8d1
     328  248 - #694097
     340  248 - #874bb5
     328  252 - #6b4098
     336  252 - #8248af
     328  256 - #6b4098
     332  256 - #7944a6
     344  256 - #8a4cb8
     248  264 - #00ff60
     404  280 - #1ae763
      80  284 - #6ba06b
     476  288 - #897b55
     488  292 - #b74f51
     300  296 - #659867
       8  300 - #00ff60
     188  300 - #4297f1
     476  300 - #8e4647
     496  300 - #b74e50
     196  304 - #51a0f4
     204  304 - #499ef2
     476  304 - #8e4647
     184  308 - #4299f0
     468  308 - #8e4747
     508  308 - #b3423c
     192  312 - #51a1f2
     204  312 - #4a9ff1
     360  312 - #35ce65
     468  312 - #8e4747
     488  312 - #b3423c
     176  316 - #3c85cf
     212  316 - #4492dc
     180  320 - #3889d5
     472  320 - #8e4646
     184  324 - #3586ce
     204  324 - #378dd5
     476  324 - #b3423c
     504  324 - #b3423c
     576  324 - #18e461
     184  328 - #3070ac
     192  328 - #307fc2
     196  328 - #3181c3
     200  328 - #307ebe
     488  328 - #b3423c
      24  364 - #36d066
     380  364 - #eb9153
     384  364 - #ec8f4b
      80  368 - #1ae662
     360  368 - #a2673a
     368  368 - #d37e3b
     372  368 - #df8540
     376  368 - #e88e4d
     380  368 - #ed9459
     384  368 - #ed914f
     388  368 - #ec8d45
     280  372 - #00ff60
     364  372 - #be7236
     380  372 - #ea8e4a
     384  372 - #ec8e47
     360  380 - #a46434
     364  380 - #ac662e
     592  380 - #629465
     136  384 - #32ca63
     360  384 - #a36432
     376  384 - #cf7931
     388  384 - #dc8138
     364  392 - #b0682e
     380  392 - #d77d32
     360  396 - #a36330
     388  396 - #dc813b
     356  400 - #53b44e
     376  400 - #69bc48
     368  404 - #c1712f
     360  408 - #a2622e
     388  408 - #db8343
     180  420 - #34cd64
     372  420 - #bb6c2a
     380  420 - #c07031
     500  428 - #619365
       4  436 - #659867
     116  440 - #33cd64
     240  440 - #00ff60
     308  440 - #00ff60
     576  456 - #639566
     396  468 - #32cb63
      80  504 - #35ce65
     360  504 - #649666
       8  508 - #33cd64
     172  508 - #00ff60
     448  516 - #00ff60
     276  524 - #33cc64
     576  536 - #19e562
     132  548 - #35cf65
     532  580 - #31c962
     468  584 - #30c862
     376  588 - #18e361
      12  592 - #6a9f6b
     100  592 - #6a9f6b
     224  592 - #669968
     308  592 - #31c962
";

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
