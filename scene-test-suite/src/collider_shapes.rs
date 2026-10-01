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
     284    4 - #d5e0ec
     568    4 - #d5e0ec
     424   36 - #d5e0eb
     144   56 - #d4dfeb
     592  112 - #d3dde8
     132  172 - #71a76f
     364  176 - #65e89e
     424  180 - #96ddbc
      52  184 - #70a56e
     212  188 - #63e79c
     236  188 - #95daba
     512  196 - #1be863
     556  220 - #c3ccd5
     580  224 - #c3ccd5
       4  228 - #c3cbd5
     336  228 - #8c57cd
     592  228 - #c3cbd5
     336  232 - #8c57cd
     340  232 - #8c57cd
     572  232 - #c2cad3
     328  236 - #684095
     348  236 - #8b52b9
     184  240 - #6da26d
     260  240 - #1ce863
     332  240 - #7944a6
     336  244 - #8248af
     348  244 - #8c54b9
     588  244 - #c0c8d1
     328  248 - #6a4097
     340  248 - #874bb5
     328  252 - #6b4099
     336  252 - #8248af
     328  256 - #6b4099
     332  256 - #7944a6
     344  256 - #8a4cb8
     120  260 - #36cf66
      56  280 - #35cf66
     396  280 - #00ff60
     476  288 - #8a7b55
     300  296 - #659867
     488  296 - #b74e50
     504  296 - #b74f51
     184  300 - #4195ef
     204  300 - #25cda9
     476  300 - #8e4647
     196  304 - #51a0f4
     476  304 - #8e4647
     176  308 - #468ddd
     184  308 - #4299f0
     468  308 - #8e4747
     192  312 - #51a1f2
     204  312 - #4aa0f1
     468  312 - #8e4747
     492  312 - #b3423c
     212  316 - #4493dc
     504  316 - #b3423c
     180  320 - #3889d5
     184  324 - #3586ce
     192  324 - #1dc99f
     204  324 - #378dd5
     592  324 - #629365
     184  328 - #3171ac
     192  328 - #307fc2
     196  328 - #3181c3
     200  328 - #317ebe
     480  328 - #b3423c
     500  328 - #b3423c
      12  340 - #00ff60
     368  356 - #c07a47
     380  364 - #eb9253
     384  364 - #ec8f4b
      88  368 - #00ff60
     308  368 - #00ff60
     360  368 - #a3673a
     368  368 - #d37e3b
     372  368 - #df8540
     376  368 - #e88e4d
     380  368 - #ed9459
     384  368 - #ed914f
     388  368 - #ec8d45
     256  372 - #34cd65
     360  372 - #a36536
     364  372 - #be7236
     380  372 - #ea8e4a
     384  372 - #ec8e47
     372  376 - #d77f39
     360  380 - #a46434
     364  380 - #ac662e
     360  384 - #a36432
     376  384 - #cf7931
     388  384 - #dc8138
     592  384 - #629465
     364  388 - #af682e
     360  392 - #a36330
     380  392 - #d77d33
     172  400 - #33cc64
     356  400 - #53b44f
     368  400 - #c0712f
     388  404 - #db8341
     360  408 - #a2622e
     376  412 - #6abd49
     512  416 - #18e461
     372  420 - #bb6c2a
     380  420 - #c07031
     312  436 - #1be763
       8  440 - #659767
     108  440 - #34cd65
     184  444 - #00ff60
     444  444 - #639566
     592  444 - #639667
     236  476 - #35cf65
     504  476 - #18e461
     316  492 - #00ff60
     164  508 - #36cf66
     404  508 - #629365
      80  516 - #6a9f6b
     276  524 - #34cd64
     592  524 - #659868
     496  544 - #18e461
       4  548 - #1ae763
     336  552 - #31ca63
     208  568 - #689c6a
     536  580 - #32ca63
     452  584 - #00ff60
     372  588 - #18e461
     116  592 - #6a9f6b
     300  592 - #19e461
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
