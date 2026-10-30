use std::{f32::consts::FRAC_PI_4, sync::Arc};

use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    gm::volume::{Quat, Ray, Shape3, Vec3},
    refs::Weak,
    scene::{
        Body, Camera, CoefficientCombineRule, ColliderShape, ConvexHull, MeshData, Model, Node,
        NodeTemplates, SceneCreation, SceneSetup, SceneTest, Sky, Wall, WorldMove, scene, step_scene,
    },
    ui::Color,
    ui_test::checkpoint,
};

use crate::geometry::{WHITE, flat_triangle};

/// Frames for the lump to fall and come to rest.
const SETTLE_FRAMES: u32 = 300;
const LUMP_AT: Vec3 = Vec3::new(-1.5, 2.5, 0.0);
/// A second lump lies this far along x from the origin of its node, and
/// the origin is over this place.
const SIDE_LUMP_OFF: f32 = 1.5;
const SIDE_LUMP_AT: Vec3 = Vec3::new(-3.0, 2.5, -1.5);
/// The base of the lump below its origin.
const LUMP_BASE: f32 = 0.4;
/// How far a rest or a hit may be from the one worked out by hand.
const TOLERANCE: f32 = 0.03;
/// Where the wall stands, a pyramid with its tip this far up.
const PYRAMID_AT: Vec3 = Vec3::new(1.5, 0.0, 0.0);
const PYRAMID_TIP: f32 = 1.5;

fn the_move() -> WorldMove {
    WorldMove {
        turn:  Quat::from_rotation_y(FRAC_PI_4),
        shift: Vec3::new(120.0, 0.0, -80.0),
    }
}

/// A cut off pyramid of dug ground that leans to 1 side, its base 0.4 under
/// its origin. A box around it would stick out at every slanted side.
fn lump_points() -> Vec<Vec3> {
    vec![
        Vec3::new(-0.5, -LUMP_BASE, -0.45),
        Vec3::new(0.55, -LUMP_BASE, -0.4),
        Vec3::new(0.5, -LUMP_BASE, 0.5),
        Vec3::new(-0.45, -LUMP_BASE, 0.45),
        Vec3::new(-0.1, 0.3, -0.15),
        Vec3::new(0.3, 0.35, -0.1),
        Vec3::new(0.25, 0.25, 0.2),
        Vec3::new(-0.15, 0.3, 0.1),
    ]
}

fn pyramid_points() -> Vec<Vec3> {
    vec![
        Vec3::new(-1.0, 0.0, -1.0),
        Vec3::new(1.0, 0.0, -1.0),
        Vec3::new(1.0, 0.0, 1.0),
        Vec3::new(-1.0, 0.0, 1.0),
        Vec3::new(0.0, PYRAMID_TIP, 0.0),
    ]
}

/// The faces of a hull as a mesh to draw.
fn hull_mesh(hull: &ConvexHull) -> MeshData {
    let mut mesh = MeshData::default();
    for triangle in hull.triangles() {
        let corners = triangle.map(|index| hull.vertices()[usize::try_from(index).expect("fits usize")]);
        flat_triangle(&mut mesh, corners, WHITE);
    }
    mesh
}

fn ground() -> MeshData {
    let mut mesh = MeshData::default();
    let corner = |x: f32, z: f32| Vec3::new(x, 0.0, z);
    let side = 20.0;
    flat_triangle(
        &mut mesh,
        [corner(-side, -side), corner(-side, side), corner(side, side)],
        WHITE,
    );
    flat_triangle(
        &mut mesh,
        [corner(-side, -side), corner(side, side), corner(side, -side)],
        WHITE,
    );
    mesh
}

/// A convex hull on the real nodes of a scene. A lump falls as a `Body`
/// onto a triangle mesh and rests on its wide base, and a `Wall` stands as a
/// pyramid. A second lump lies beside the origin of its node. Its body has to
/// balance around the lump and not around that origin, or it tips over. A ray,
/// a swept ball and a placed hull meet the pyramid where its
/// sides are, where the box around it would answer otherwise. Then the
/// whole world moves and turns: the lump stays at rest on the moved ground
/// and the queries give the same answers in the moved places. Every collider
/// is drawn as its green lines, along the faces of the hulls.
#[scene]
#[derive(Default)]
struct ConvexHullCollider {
    lump:      Weak<Body>,
    side_lump: Weak<Body>,
    pyramid:   Weak<Wall>,
}

impl SceneSetup for ConvexHullCollider {
    fn needs_physics(&self) -> bool {
        true
    }

    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 3.5, 6.0),
            target: Vec3::new(0.0, 0.5, 0.0),
            ..Camera::default()
        };
        self.sky = Some(Sky::gradient(
            Color::hex("#3a7bd5"),
            Color::hex("#d9e4f0"),
            Color::hex("#5a4a3a"),
        ));
        self.show_colliders = true;

        let ground = ground();
        self.make_node::<Wall>(
            Shape3::Model(Model::from_mesh("Convex hull ground", ground.clone())),
            Vec3::ZERO,
        )
        .set_collider(ColliderShape::TriMesh(Arc::new(ground)), Vec3::ZERO)
        .set_restitution(0.0, CoefficientCombineRule::Min)
        .set_color(Color::hex("#6b9a55"))
        .set_roughness(0.9);

        let lump = ConvexHull::from_points(&lump_points()).expect("the lump is a solid");
        let model = Model::from_mesh("Convex hull lump", hull_mesh(&lump));
        let mut body = self.make_node::<Body>(Shape3::Model(model), LUMP_AT);
        body.set_collider(ColliderShape::ConvexHull(Arc::new(lump)), Vec3::ZERO)
            .set_color(Color::hex("#8a5a33"))
            .set_roughness(0.9)
            .set_restitution(0.0, CoefficientCombineRule::Min);
        self.lump = body;

        let aside: Vec<Vec3> = lump_points().iter().map(|point| *point + Vec3::X * SIDE_LUMP_OFF).collect();
        let lump = ConvexHull::from_points(&aside).expect("the lump is a solid");
        let model = Model::from_mesh("Convex hull side lump", hull_mesh(&lump));
        let mut body = self.make_node::<Body>(Shape3::Model(model), SIDE_LUMP_AT);
        body.set_collider(ColliderShape::ConvexHull(Arc::new(lump)), Vec3::ZERO)
            .set_color(Color::hex("#a0703f"))
            .set_roughness(0.9)
            .set_restitution(0.0, CoefficientCombineRule::Min);
        self.side_lump = body;

        let pyramid = ConvexHull::from_points(&pyramid_points()).expect("the pyramid is a solid");
        let model = Model::from_mesh("Convex hull pyramid", hull_mesh(&pyramid));
        let mut wall = self.make_node::<Wall>(Shape3::Model(model), PYRAMID_AT);
        wall.set_collider(ColliderShape::ConvexHull(Arc::new(pyramid)), Vec3::ZERO)
            .set_color(Color::hex("#c0392b"))
            .set_roughness(0.8);
        self.pyramid = wall;
    }
}

/// The answers about the pyramid, asked and checked in the places `world`
/// moved everything to.
fn check_queries(scene: Weak<ConvexHullCollider>, world: WorldMove) -> Result<()> {
    let turn = world.turn;
    let moved = move |point: Vec3| world.point(point);
    let (pyramid, tip_hit, beside_hit, sweep, overlaps_near, overlaps_far) = from_main(move || {
        let down = Ray {
            origin:    moved(PYRAMID_AT + Vec3::new(0.0, 5.0, 0.0)),
            direction: turn * Vec3::NEG_Y,
        };
        // Over a corner of the base, inside the box around the pyramid. A box
        // would stop it at the height of the tip.
        let beside = Ray {
            origin:    moved(PYRAMID_AT + Vec3::new(0.9, 5.0, 0.9)),
            direction: turn * Vec3::NEG_Y,
        };
        let ball = ColliderShape::Ball(0.1);
        let sweep_from = moved(PYRAMID_AT + Vec3::new(0.0, 0.5, 3.0));
        // A small hull at the foot of the pyramid, and a corner away from it.
        let small = ColliderShape::ConvexHull(Arc::new(
            ConvexHull::from_points(&[
                Vec3::new(-0.1, -0.1, -0.1),
                Vec3::new(0.1, -0.1, -0.1),
                Vec3::new(0.0, -0.1, 0.1),
                Vec3::new(0.0, 0.1, 0.0),
            ])
            .expect("a small tetrahedron"),
        ));
        (
            scene.pyramid.weak_node(),
            scene.cast_ray(down, 20.0, &[]),
            scene.cast_ray(beside, 20.0, &[]),
            scene.cast_shape(&ball, sweep_from, Quat::IDENTITY, turn * Vec3::NEG_Z, 10.0, &[]),
            scene.overlapping(&small, moved(PYRAMID_AT + Vec3::new(0.0, 1.0, 0.0)), turn, &[]),
            scene.overlapping(&small, moved(PYRAMID_AT + Vec3::new(0.9, 1.0, 0.9)), turn, &[]),
        )
    });

    let tip_hit = tip_hit.ok_or_else(|| anyhow::anyhow!("the ray down missed the pyramid"))?;
    ensure!(tip_hit.node == pyramid, "the ray down stops on the pyramid");
    ensure!(
        (tip_hit.distance - (5.0 - PYRAMID_TIP)).abs() < TOLERANCE,
        "the tip is 3.5 below, got {}",
        tip_hit.distance
    );
    let beside_hit = beside_hit.ok_or_else(|| anyhow::anyhow!("the ray beside the tip found nothing"))?;
    ensure!(
        beside_hit.distance > 5.0 - PYRAMID_TIP * 0.2,
        "the side is low over the corner, the ray stopped after {}",
        beside_hit.distance
    );
    // The side falls 1.5 over 1 along z, at height 0.5 it is 2 / 3 from the
    // middle, the ball stops 0.1 in front of it.
    let sweep = sweep.ok_or_else(|| anyhow::anyhow!("the swept ball found nothing"))?;
    ensure!(sweep.node == pyramid, "the swept ball stops on the pyramid");
    ensure!(
        sweep.distance > 2.0 && sweep.distance < 2.33,
        "the swept ball stopped after {}",
        sweep.distance
    );
    ensure!(
        overlaps_near.contains(&pyramid),
        "a hull placed inside the pyramid does not overlap it"
    );
    ensure!(
        !overlaps_far.contains(&pyramid),
        "a hull over the corner of the base overlaps the pyramid, its box and not its hull"
    );
    Ok(())
}

impl SceneTest for ConvexHullCollider {
    fn stepped() -> bool {
        true
    }

    fn perform_test(scene: Weak<Self>) -> Result<()> {
        step_scene(SETTLE_FRAMES);

        let (at, speed) = from_main(move || (scene.lump.position(), scene.lump.velocity()));
        ensure!(speed.length() < 0.05, "the lump still moves at {speed}");
        ensure!(
            (at.y - LUMP_BASE).abs() < TOLERANCE,
            "the lump rests with its origin at {at}, its base is {LUMP_BASE} below that"
        );
        let (side_at, side_turn) =
            from_main(move || (scene.side_lump.position(), scene.side_lump.rotation()));
        ensure!(
            (side_at.y - LUMP_BASE).abs() < TOLERANCE && side_turn.angle_between(Quat::IDENTITY) < 0.05,
            "the lump beside its origin rests at {side_at} turned by {side_turn}, it should stand on its base"
        );
        check_queries(
            scene,
            WorldMove {
                turn:  Quat::IDENTITY,
                shift: Vec3::ZERO,
            },
        )?;
        checkpoint("the lump rests on its base, green lines along the lump and the pyramid")?;

        let world = the_move();
        from_main(move || {
            let mut scene = scene;
            scene.move_world(world);
        });
        step_scene(60);
        let (moved, speed) = from_main(move || (scene.lump.position(), scene.lump.velocity()));
        ensure!(speed.length() < 0.05, "after the move the lump moves at {speed}");
        ensure!(
            moved.distance(world.point(at)) < TOLERANCE,
            "the lump went from {at} to {moved}, the move puts it at {}",
            world.point(at)
        );
        check_queries(scene, world)?;
        checkpoint("the world moved and turned, the same picture and the same answers")
    }
}
