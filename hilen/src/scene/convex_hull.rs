use std::collections::{BTreeSet, HashSet};

use log::warn;
use rapier3d::{
    parry::{
        shape::{ConvexPolyhedron, SharedShape},
        transformation::try_convex_hull,
    },
    prelude::ColliderBuilder,
};

use crate::{gm::volume::Vec3, scene::MeshData};

/// A hull thinner than this share of the box around it is a flat sheet, it
/// has no inside to push a body out of.
const SMALLEST_FILL: f32 = 1e-6;

/// The smallest convex solid around a list of points, what a falling lump
/// of any shape collides as, see `ColliderShape::ConvexHull`. It holds a
/// real solid by construction, so a collider made of it cannot fail.
#[derive(Clone, Debug, PartialEq)]
pub struct ConvexHull {
    vertices:  Vec<Vec3>,
    triangles: Vec<[u32; 3]>,
}

impl ConvexHull {
    /// The hull of `points`, in the node's space. `None` when they make no
    /// solid: under 4 different points, all on 1 line or on 1 plane, so
    /// thin that it fills under a millionth of the cube of its longest
    /// side, or a point that is not finite. A point inside the hull is
    /// dropped.
    pub fn from_points(points: &[Vec3]) -> Option<Self> {
        if !points.iter().all(|point| point.is_finite()) {
            return None;
        }
        // A mesh repeats a corner for each triangle it is on, and the
        // hull refuses a list with a point twice.
        let mut seen = HashSet::new();
        let points: Vec<Vec3> = points
            .iter()
            .filter(|point| seen.insert(point.to_array().map(f32::to_bits)))
            .copied()
            .collect();
        if points.len() < 4 {
            return None;
        }
        let (vertices, triangles) = try_convex_hull(&points).ok()?;
        ConvexPolyhedron::from_convex_mesh(vertices.clone(), &triangles)?;
        let hull = Self { vertices, triangles };
        hull.has_inside().then_some(hull)
    }

    /// The hull of every vertex of `mesh`, see `from_points`.
    pub fn from_mesh(mesh: &MeshData) -> Option<Self> {
        let points: Vec<Vec3> = mesh.vertices.iter().map(|vertex| vertex.pos).collect();
        Self::from_points(&points)
    }

    /// The corners of the hull, the points that were not inside it.
    pub fn vertices(&self) -> &[Vec3] {
        &self.vertices
    }

    /// The faces as 3 indices into `vertices`, counter clockwise seen from
    /// outside.
    pub fn triangles(&self) -> &[[u32; 3]] {
        &self.triangles
    }

    /// Every edge of the triangles once, as 2 points.
    pub(crate) fn edges(&self) -> Vec<(Vec3, Vec3)> {
        let mut edges = BTreeSet::new();
        for &[a, b, c] in &self.triangles {
            for (from, to) in [(a, b), (b, c), (c, a)] {
                edges.insert((from.min(to), from.max(to)));
            }
        }
        let at = |index: u32| self.vertices[usize::try_from(index).expect("a u32 index fits usize")];
        edges.into_iter().map(|(from, to)| (at(from), at(to))).collect()
    }

    /// The lowest and the highest corner of the box around the hull.
    pub(crate) fn bounds(&self) -> (Vec3, Vec3) {
        self.vertices.iter().fold(
            (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
            |(low, high), &vertex| (low.min(vertex), high.max(vertex)),
        )
    }

    /// The rapier collider at a node's uniform `scale`. A scale so small
    /// that the hull falls apart in floats, like 0, gives a ball of that
    /// size in its place, the way a box of that size stays a box.
    pub(crate) fn builder(&self, scale: f32) -> ColliderBuilder {
        let scaled: Vec<Vec3> = self.vertices.iter().map(|vertex| *vertex * scale).collect();
        if let Some(solid) = ConvexPolyhedron::from_convex_mesh(scaled, &self.triangles) {
            ColliderBuilder::new(SharedShape::new(solid))
        } else {
            let (low, high) = self.bounds();
            warn!("a convex hull has no solid at scale {scale}, it collides as a ball");
            ColliderBuilder::ball(high.distance(low) / 2.0 * scale.abs())
        }
    }

    /// Whether the hull fills enough of the box around it to be a solid.
    /// Points on 1 plane can come back as 2 faces back to back. The volume
    /// is the sum of the cones from the first corner to every face.
    fn has_inside(&self) -> bool {
        let (low, high) = self.bounds();
        let size = high - low;
        let apex = self.vertices[0];
        let volume: f32 = self
            .triangles
            .iter()
            .map(|triangle| {
                let [a, b, c] = triangle
                    .map(|index| self.vertices[usize::try_from(index).expect("a u32 index fits usize")]);
                (a - apex).dot((b - apex).cross(c - apex)) / 6.0
            })
            .sum();
        volume.abs() > SMALLEST_FILL * size.max_element().powi(3)
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use rapier3d::prelude::RigidBodyBuilder;

    use super::*;
    use crate::{
        gm::{LossyConvert, flat::Point, volume::Vertex3D},
        scene::{ColliderShape, scene::ScenePhysics},
    };

    const STEP: f32 = 1.0 / 240.0;

    /// The ground at height 0 as 1 big quad of a triangle mesh.
    fn flat_ground() -> MeshData {
        let corner = |x: f32, z: f32| Vertex3D::new(Vec3::new(x, 0.0, z), Vec3::Y, Point::default());
        MeshData {
            vertices: vec![
                corner(-20.0, -20.0),
                corner(-20.0, 20.0),
                corner(20.0, 20.0),
                corner(20.0, -20.0),
            ],
            indices:  vec![0, 1, 2, 0, 2, 3],
        }
    }

    /// A lump of dug ground, a cut off pyramid leaning to 1 side, with its
    /// flat wide base 0.4 under the origin and its corners on no grid.
    fn lump() -> Vec<Vec3> {
        vec![
            Vec3::new(-0.5, -0.4, -0.45),
            Vec3::new(0.55, -0.4, -0.4),
            Vec3::new(0.5, -0.4, 0.5),
            Vec3::new(-0.45, -0.4, 0.45),
            Vec3::new(-0.1, 0.3, -0.15),
            Vec3::new(0.3, 0.35, -0.1),
            Vec3::new(0.25, 0.25, 0.2),
            Vec3::new(-0.15, 0.3, 0.1),
            // Inside, it has to go.
            Vec3::new(0.1, 0.0, 0.05),
        ]
    }

    /// A falling hull the way `Body` makes a body, in a world with the
    /// ground as a fixed triangle mesh.
    fn dropped(
        hull: &ConvexHull,
        scale: f32,
        at: Vec3,
        ground: MeshData,
    ) -> (ScenePhysics, rapier3d::dynamics::RigidBodyHandle) {
        let mut physics = ScenePhysics::default();
        physics
            .sets
            .colliders
            .insert(ColliderShape::TriMesh(Arc::new(ground)).builder(1.0));
        let body = RigidBodyBuilder::dynamic().translation(at).ccd_enabled(true).build();
        let collider = ColliderShape::ConvexHull(Arc::new(hull.clone())).builder(scale).build();
        let (body, _) = physics.sets.insert(body, collider);
        (physics, body)
    }

    /// The lowest corner of the hull in the world, as the body holds it.
    fn lowest(physics: &ScenePhysics, body: rapier3d::dynamics::RigidBodyHandle, hull: &ConvexHull) -> f32 {
        let pose = physics.sets.rigid_bodies[body].position();
        hull.vertices()
            .iter()
            .map(|&vertex| (*pose * vertex).y)
            .fold(f32::INFINITY, f32::min)
    }

    #[test]
    fn points_inside_are_dropped() {
        let hull = ConvexHull::from_points(&lump()).expect("a lump is a solid");
        assert_eq!(hull.vertices().len(), 8);
        assert!(!hull.vertices().contains(&Vec3::new(0.1, 0.0, 0.05)));
        for triangle in hull.triangles() {
            let [a, b, c] = triangle.map(|index| hull.vertices()[index as usize]);
            let center = hull.vertices().iter().copied().sum::<Vec3>() / 8.0;
            assert!((b - a).cross(c - a).dot(a - center) > 0.0, "a face looks inward");
        }
    }

    #[test]
    fn points_that_make_no_solid_make_no_hull() {
        let cases: [(&str, Vec<Vec3>); 8] = [
            ("no points", vec![]),
            ("3 points", vec![Vec3::ZERO, Vec3::X, Vec3::Y]),
            ("1 point 5 times", vec![Vec3::ONE; 5]),
            (
                "3 points, some twice",
                vec![Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::X, Vec3::Y],
            ),
            (
                "on 1 line",
                (0..6).map(|i| Vec3::X * f32::from(u8::try_from(i).unwrap())).collect(),
            ),
            (
                "on 1 plane",
                vec![
                    Vec3::new(0.0, 2.0, 0.0),
                    Vec3::new(1.0, 2.0, 0.0),
                    Vec3::new(1.0, 2.0, 1.0),
                    Vec3::new(0.0, 2.0, 1.0),
                    Vec3::new(0.5, 2.0, 0.3),
                ],
            ),
            (
                "on a turned plane",
                vec![
                    Vec3::new(0.0, 0.0, 0.0),
                    Vec3::new(1.0, 1.0, 0.0),
                    Vec3::new(0.0, 1.0, 1.0),
                    Vec3::new(1.0, 2.0, 1.0),
                    Vec3::new(2.0, 3.0, 1.0),
                ],
            ),
            (
                "a point that is not a number",
                vec![Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z, Vec3::NAN],
            ),
        ];
        for (name, points) in cases {
            assert!(ConvexHull::from_points(&points).is_none(), "{name} made a hull");
        }
        assert!(ConvexHull::from_points(&[Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z * f32::INFINITY]).is_none());
        assert!(
            ConvexHull::from_mesh(&flat_ground()).is_none(),
            "a flat mesh made a hull"
        );
    }

    // A dug lump comes from a grid, so most of its points lie on the same
    // planes and lines, and each corner is there once per triangle.
    #[test]
    fn a_grid_of_points_makes_a_hull() {
        let mut points = vec![];
        for x in 0..6u8 {
            for y in 0..6u8 {
                for z in 0..6u8 {
                    let at = Vec3::new(f32::from(x), f32::from(y), f32::from(z)) * 0.2;
                    points.extend([at, at]);
                }
            }
        }
        let hull = ConvexHull::from_points(&points).expect("a full grid is a box");
        let (low, high) = hull.bounds();
        assert!(low.distance(Vec3::ZERO) < 1e-5 && high.distance(Vec3::ONE) < 1e-5);
    }

    // The points of a real surface, a ball of rings: a few hundred points,
    // all on the hull, no 2 faces in 1 plane.
    #[test]
    fn a_round_cloud_makes_a_hull_of_all_its_points() {
        let mut points = vec![Vec3::Y, Vec3::NEG_Y];
        for ring in 1..16u8 {
            let down = f32::from(ring) / 16.0 * std::f32::consts::PI;
            for step in 0..24u8 {
                let round = f32::from(step) / 24.0 * std::f32::consts::TAU;
                points.push(Vec3::new(
                    down.sin() * round.cos(),
                    down.cos(),
                    down.sin() * round.sin(),
                ));
            }
        }
        let hull = ConvexHull::from_points(&points).expect("a ball is a solid");
        assert_eq!(hull.vertices().len(), points.len());
    }

    #[test]
    fn a_mesh_gives_the_hull_of_its_vertices() {
        let corner = |pos: Vec3| Vertex3D::new(pos, Vec3::Y, Point::default());
        let mesh = MeshData {
            vertices: lump().into_iter().map(corner).collect(),
            indices:  vec![0, 1, 2],
        };
        assert_eq!(
            ConvexHull::from_mesh(&mesh).expect("its vertices are a solid").vertices().len(),
            8
        );
    }

    #[test]
    fn a_hull_falls_lands_and_rests_on_a_triangle_mesh() {
        let hull = ConvexHull::from_points(&lump()).unwrap();
        let (mut physics, body) = dropped(&hull, 1.0, Vec3::new(1.0, 3.0, -2.0), flat_ground());
        let mut fell = false;
        for _ in 0..240 * 6 {
            physics.update_physics(&[], STEP);
            fell |= physics.sets.rigid_bodies[body].linvel().y < -2.0;
        }
        assert!(fell, "it never fell");
        let rest = &physics.sets.rigid_bodies[body];
        assert!(rest.linvel().length() < 0.01, "still moves at {}", rest.linvel());
        assert!(rest.angvel().length() < 0.01, "still turns at {}", rest.angvel());
        let bottom = lowest(&physics, body, &hull);
        assert!(bottom.abs() < 0.02, "its lowest corner rests at {bottom}");
        // It stands on its wide base, 0.4 under the origin, not on a box
        // around it and not sunk.
        let at = physics.sets.rigid_bodies[body].translation();
        assert!((at.y - 0.4).abs() < 0.02, "its origin rests at {}", at.y);
        assert!(
            (at.x - 1.0).abs() < 0.05 && (at.z + 2.0).abs() < 0.05,
            "it slid to {at}"
        );
    }

    // Tipped far over it has to roll onto a face of the hull, a box or a
    // ball around the lump would rest in another pose.
    #[test]
    fn a_tipped_hull_rolls_onto_a_face() {
        let hull = ConvexHull::from_points(&lump()).unwrap();
        let (mut physics, body) = dropped(&hull, 1.0, Vec3::new(0.0, 2.0, 0.0), flat_ground());
        physics.sets.rigid_bodies[body].set_rotation(rapier3d::math::Rotation::from_rotation_x(2.2), true);
        for _ in 0..240 * 10 {
            physics.update_physics(&[], STEP);
        }
        let rest = &physics.sets.rigid_bodies[body];
        assert!(rest.linvel().length() < 0.02, "still moves at {}", rest.linvel());
        let pose = *rest.position();
        let touching = hull.vertices().iter().filter(|&&vertex| (pose * vertex).y.abs() < 0.02).count();
        assert!(
            touching >= 3,
            "{touching} corners touch the ground, a face has at least 3"
        );
    }

    #[test]
    fn a_fast_hull_does_not_pass_through_a_triangle_mesh() {
        let hull = ConvexHull::from_points(&lump()).unwrap();
        let (mut physics, body) = dropped(&hull, 1.0, Vec3::new(0.0, 40.0, 0.0), flat_ground());
        physics.sets.rigid_bodies[body].set_linvel(Vec3::new(0.0, -300.0, 0.0), true);
        for _ in 0..240 * 4 {
            physics.update_physics(&[], STEP);
        }
        let bottom = lowest(&physics, body, &hull);
        assert!(bottom > -0.05, "its lowest corner is at {bottom}");
    }

    #[test]
    fn the_scale_of_a_node_sizes_the_hull() {
        let hull = ConvexHull::from_points(&lump()).unwrap();
        let (mut physics, body) = dropped(&hull, 2.0, Vec3::new(0.0, 3.0, 0.0), flat_ground());
        for _ in 0..240 * 6 {
            physics.update_physics(&[], STEP);
        }
        let y = physics.sets.rigid_bodies[body].translation().y;
        assert!((y - 0.8).abs() < 0.03, "its origin rests at {y}");
    }

    #[test]
    fn a_scale_of_0_gives_a_collider_with_no_panic() {
        let hull = ConvexHull::from_points(&lump()).unwrap();
        ColliderShape::ConvexHull(Arc::new(hull)).builder(0.0).build();
    }

    // A wall takes a collider too, and a ray and a swept shape meet it.
    #[test]
    fn rays_and_sweeps_meet_the_hull() {
        let shape = ColliderShape::ConvexHull(Arc::new(ConvexHull::from_points(&lump()).unwrap()));
        let collider = shape.builder(1.0).build();
        let ray = rapier3d::parry::query::Ray::new(Vec3::new(0.0, -5.0, 0.0), Vec3::Y);
        let distance = collider
            .shape()
            .cast_ray(collider.position(), &ray, 100.0, true)
            .expect("hits the base");
        assert!((distance - 4.6).abs() < 1e-4, "the base is {distance} away");
        // Beside the slanted side and over the wide base, inside the box
        // around the lump, so a box would be hit.
        let beside = rapier3d::parry::query::Ray::new(Vec3::new(0.5, 5.0, 0.45), Vec3::NEG_Y);
        let top = collider
            .shape()
            .cast_ray(collider.position(), &beside, 100.0, true)
            .expect("hits the side");
        assert!(top > 5.0 + 0.2, "a ray down the corner stops at {top}");
        let (low, high) = shape.bounds();
        assert_eq!(low, Vec3::new(-0.5, -0.4, -0.45));
        assert_eq!(high, Vec3::new(0.55, 0.35, 0.5));
        assert!(shape.half_extents().distance((high - low) / 2.0) < 1e-6);
    }

    #[test]
    fn equal_points_are_equal_shapes() {
        let hull =
            |points: &[Vec3]| ColliderShape::ConvexHull(Arc::new(ConvexHull::from_points(points).unwrap()));
        assert_eq!(hull(&lump()), hull(&lump()));
        let mut other = lump();
        other[0].x -= 0.1;
        assert_ne!(hull(&lump()), hull(&other));
    }

    // Dug ground gives any list of points, nothing in it may stop the game.
    #[test]
    fn no_list_of_points_panics() {
        let mut seed = 1u32;
        let mut next = || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            seed >> 16
        };
        let mut hulls = 0u32;
        for round in 0..3000u32 {
            let count = 4 + next() % 40;
            // Few different numbers, so lines, planes and twins come often.
            let steps = 1 + round % 5;
            let points: Vec<Vec3> = (0..count)
                .map(|_| {
                    let mut axis = || {
                        let value: f32 = (next() % (steps + 1)).lossy_convert();
                        value * 0.25
                    };
                    Vec3::new(axis(), axis(), axis())
                })
                .collect();
            let hull = ConvexHull::from_points(&points);
            assert_eq!(
                hull.is_some(),
                spans_space(&points),
                "a list of {count} points on a grid of {steps} steps: {points:?}"
            );
            if let Some(hull) = hull {
                hulls += 1;
                hull.builder(1.0).build();
            }
        }
        assert!(hulls > 500, "only {hulls} of the lists made a hull");
    }

    /// Whether the points are not all on 1 plane.
    fn spans_space(points: &[Vec3]) -> bool {
        let first = points[0];
        let line = points.iter().map(|&point| point - first).find(|side| side.length() > 1e-6);
        let Some(line) = line else {
            return false;
        };
        let normal = points
            .iter()
            .map(|&point| line.cross(point - first))
            .find(|side| side.length() > 1e-6);
        let Some(normal) = normal else {
            return false;
        };
        points.iter().any(|&point| normal.dot(point - first).abs() > 1e-6)
    }
}
