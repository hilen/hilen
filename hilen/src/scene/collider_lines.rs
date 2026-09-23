use std::f32::consts::{PI, TAU};

use crate::{
    gm::{
        LossyConvert,
        volume::{Mat4, Quat, Vec3},
    },
    scene::ColliderShape,
};

/// A shown collider is drawn a little larger than its solid, so its
/// edges are not cut by the faces of the drawn shape they lie on.
const INFLATE: f32 = 1.02;
/// A heightfield's grid floats this far over the ground it outlines.
const HEIGHTFIELD_LIFT: f32 = 0.02;
/// At most this many grid lines across a heightfield each way.
const HEIGHTFIELD_LINES: usize = 24;
/// Straight pieces in each full ring.
const RING_SEGMENTS: usize = 32;

/// The corners of the unit box, then the edges as pairs of corners.
const BOX_CORNERS: [Vec3; 8] = [
    Vec3::new(-0.5, -0.5, -0.5),
    Vec3::new(0.5, -0.5, -0.5),
    Vec3::new(0.5, 0.5, -0.5),
    Vec3::new(-0.5, 0.5, -0.5),
    Vec3::new(-0.5, -0.5, 0.5),
    Vec3::new(0.5, -0.5, 0.5),
    Vec3::new(0.5, 0.5, 0.5),
    Vec3::new(-0.5, 0.5, 0.5),
];
const BOX_EDGES: [(usize, usize); 12] = [
    (0, 1),
    (1, 2),
    (2, 3),
    (3, 0),
    (4, 5),
    (5, 6),
    (6, 7),
    (7, 4),
    (0, 4),
    (1, 5),
    (2, 6),
    (3, 7),
];

/// The wireframe of a collider as line segments in the world, its center
/// at `center`, turned by `rotation` and sized by `scale`: the edges of a
/// box, three rings of a ball, the end rings and four sides of a cylinder
/// or a capsule with arcs over the capsule's caps, and a coarse grid over
/// a heightfield.
pub(crate) fn collider_lines(
    shape: &ColliderShape,
    center: Vec3,
    rotation: Quat,
    scale: f32,
) -> Vec<(Vec3, Vec3)> {
    let mut lines = vec![];
    match shape {
        ColliderShape::Box(_) | ColliderShape::Ball(_) => {
            let size = shape.half_extents() * scale * 2.0 * INFLATE;
            let model = Mat4::from_scale_rotation_translation(size, rotation, center);
            if matches!(shape, ColliderShape::Ball(_)) {
                for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
                    ring(&mut lines, &model, axis);
                }
            } else {
                let corners = BOX_CORNERS.map(|corner| model.transform_point3(corner));
                lines.extend(BOX_EDGES.map(|(from, to)| (corners[from], corners[to])));
            }
        }
        ColliderShape::Cylinder { radius, height } | ColliderShape::Capsule { radius, height } => {
            let place = Mat4::from_rotation_translation(rotation, center);
            let radius = radius * scale * INFLATE;
            let capsule = matches!(shape, ColliderShape::Capsule { .. });
            // The straight part runs between the caps of a capsule.
            let reach = height * scale * INFLATE / 2.0 - if capsule { radius } else { 0.0 };
            upright(&mut lines, &place, radius, reach.max(0.0), capsule);
        }
        ColliderShape::Heightfield(field) => {
            let place = Mat4::from_rotation_translation(rotation, center);
            let point = |row: usize, column: usize| {
                let (x, z) = field.point(row, column);
                place.transform_point3(Vec3::new(x, field.height(row, column) + HEIGHTFIELD_LIFT, z) * scale)
            };
            for row in grid_lines(field.rows) {
                for column in 1..field.columns {
                    lines.push((point(row, column - 1), point(row, column)));
                }
            }
            for column in grid_lines(field.columns) {
                for row in 1..field.rows {
                    lines.push((point(row - 1, column), point(row, column)));
                }
            }
        }
    }
    lines
}

/// Which of `count` grid points get a line, evenly spread and always the
/// last one, so the far rim is outlined too.
fn grid_lines(count: usize) -> Vec<usize> {
    let mut picked: Vec<usize> = (0..count).step_by((count / HEIGHTFIELD_LINES).max(1)).collect();
    if picked.last() != Some(&(count - 1)) {
        picked.push(count - 1);
    }
    picked
}

fn angle(segment: usize, of: usize) -> f32 {
    segment.lossy_convert() / of.lossy_convert()
}

/// The circle of diameter one around `axis`, sized and placed by `model`.
fn ring(lines: &mut Vec<(Vec3, Vec3)>, model: &Mat4, axis: Vec3) {
    let side = axis.any_orthonormal_vector();
    let up = axis.cross(side);
    let point = |segment: usize| {
        let angle = TAU * angle(segment, RING_SEGMENTS);
        model.transform_point3((side * angle.cos() + up * angle.sin()) * 0.5)
    };
    lines.extend((0..RING_SEGMENTS).map(|segment| (point(segment), point(segment + 1))));
}

/// A cylinder or capsule standing on y: a ring at each end of the
/// straight part `reach` above and below the middle, four straight sides,
/// and over a capsule's ends two half circles crossing each cap.
fn upright(lines: &mut Vec<(Vec3, Vec3)>, place: &Mat4, radius: f32, reach: f32, capsule: bool) {
    for y in [-reach, reach] {
        let model = Mat4::from_translation(Vec3::Y * y) * Mat4::from_scale(Vec3::splat(radius * 2.0));
        ring(lines, &(*place * model), Vec3::Y);
    }
    for side in [Vec3::X, Vec3::Z, Vec3::NEG_X, Vec3::NEG_Z] {
        let foot = side * radius;
        lines.push((
            place.transform_point3(foot - Vec3::Y * reach),
            place.transform_point3(foot + Vec3::Y * reach),
        ));
    }
    if !capsule {
        return;
    }
    let half = RING_SEGMENTS / 2;
    for (cap, sign) in [(reach, 1.0), (-reach, -1.0)] {
        for across in [Vec3::X, Vec3::Z] {
            let point = |segment: usize| {
                let angle = PI * angle(segment, half);
                let local = across * angle.cos() * radius + Vec3::Y * (cap + sign * angle.sin() * radius);
                place.transform_point3(local)
            };
            lines.extend((0..half).map(|segment| (point(segment), point(segment + 1))));
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use super::*;
    use crate::scene::Heightfield;

    fn highest(lines: &[(Vec3, Vec3)]) -> f32 {
        lines.iter().flat_map(|(a, b)| [a.y, b.y]).fold(f32::NEG_INFINITY, f32::max)
    }

    #[test]
    fn a_box_is_its_twelve_edges() {
        let lines = collider_lines(&ColliderShape::Box(Vec3::ONE), Vec3::ZERO, Quat::IDENTITY, 1.0);
        assert_eq!(lines.len(), 12);
    }

    // The caps of a capsule reach its full height, the straight sides stop
    // a radius short of it.
    #[test]
    fn a_capsule_reaches_its_height_through_its_caps() {
        let shape = ColliderShape::Capsule {
            radius: 0.5,
            height: 3.0,
        };
        let lines = collider_lines(&shape, Vec3::ZERO, Quat::IDENTITY, 2.0);
        assert_eq!(lines.len(), 2 * RING_SEGMENTS + 4 + 4 * (RING_SEGMENTS / 2));
        assert!((highest(&lines) - 3.0 * INFLATE).abs() < 1e-4);
    }

    #[test]
    fn a_cylinder_is_flat_topped() {
        let shape = ColliderShape::Cylinder {
            radius: 1.0,
            height: 2.0,
        };
        let lines = collider_lines(&shape, Vec3::Y, Quat::IDENTITY, 1.0);
        assert_eq!(lines.len(), 2 * RING_SEGMENTS + 4);
        assert!((highest(&lines) - (1.0 + INFLATE)).abs() < 1e-4);
    }

    // A dense terrain would bury the view in green, the grid stays coarse
    // and still closes on the far rim.
    #[test]
    fn a_heightfield_grid_is_coarse_and_closed() {
        let field = Heightfield::from_fn(97, 97, 10.0, 10.0, |_, _| 0.0);
        let lines = collider_lines(
            &ColliderShape::Heightfield(Arc::new(field)),
            Vec3::ZERO,
            Quat::IDENTITY,
            1.0,
        );
        // Every fourth of 97 points, the last one among them.
        assert_eq!(lines.len(), 2 * 25 * 96);
        assert!(lines.iter().any(|(a, _)| (a.x - 5.0).abs() < 1e-4));
    }
}
