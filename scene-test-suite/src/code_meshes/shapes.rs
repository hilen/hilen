//! The shapes of `Code meshes`, each built a different way: a tube swept
//! along a curve, a profile turned around an axis, an outline pushed up
//! while it turns, and a subdivided solid pushed in and out.

use std::f32::consts::TAU;

use hilen::{
    gm::volume::{Quat, Vec3, Vertex3D},
    scene::MeshData,
};

use crate::geometry::{WHITE, flat_triangle, float, icosphere, index, ratio};

fn vertex(pos: Vec3, normal: Vec3) -> Vertex3D {
    crate::geometry::vertex(pos, normal, WHITE)
}

/// Smooth quads over a grid of `rows` by `columns` vertices, wrapping
/// around the columns. Rows run along the first surface direction and
/// columns along the second, so their cross product must point out.
fn grid_indices(mesh: &mut MeshData, rows: usize, columns: usize, wrap_rows: bool) {
    let row_count = if wrap_rows { rows } else { rows - 1 };
    for row in 0..row_count {
        for column in 0..columns {
            let at = |r: usize, c: usize| index((r % rows) * columns + c % columns);
            let (a, b) = (at(row, column), at(row + 1, column));
            let (c, d) = (at(row, column + 1), at(row + 1, column + 1));
            mesh.indices.extend([a, b, c, c, b, d]);
        }
    }
}

/// A trefoil knot as a tube. The frame around the curve is carried along
/// it by parallel transport, then untwisted by what it gained over one
/// loop, so the end meets the start without a seam.
pub fn knot() -> MeshData {
    const SEGMENTS: usize = 320;
    const SIDES: usize = 20;
    const RADIUS: f32 = 0.22;

    let curve = |t: f32| {
        Vec3::new(
            t.sin() + 2.0 * (2.0 * t).sin(),
            t.cos() - 2.0 * (2.0 * t).cos(),
            -(3.0 * t).sin(),
        ) * 0.45
    };
    let tangent = |t: f32| (curve(t + 1e-3) - curve(t - 1e-3)).normalize();

    let tangents: Vec<Vec3> = (0..=SEGMENTS).map(|i| tangent(TAU * ratio(i, SEGMENTS))).collect();
    let mut normals = vec![tangents[0].any_orthonormal_vector()];
    for i in 1..=SEGMENTS {
        let carried = normals[i - 1] - tangents[i] * normals[i - 1].dot(tangents[i]);
        normals.push(carried.normalize());
    }
    let (start, end) = (normals[0], normals[SEGMENTS]);
    let twist = end.cross(start).dot(tangents[0]).atan2(end.dot(start));

    let mut mesh = MeshData::default();
    for i in 0..SEGMENTS {
        let t = TAU * ratio(i, SEGMENTS);
        let normal = Quat::from_axis_angle(tangents[i], twist * ratio(i, SEGMENTS)) * normals[i];
        let binormal = tangents[i].cross(normal);
        for side in 0..SIDES {
            // Around against the binormal, so along the curve then around
            // is outward.
            let angle = -TAU * ratio(side, SIDES);
            let out = normal * angle.cos() + binormal * angle.sin();
            mesh.vertices.push(vertex(curve(t) + out * RADIUS, out));
        }
    }
    grid_indices(&mut mesh, SEGMENTS, SIDES, true);
    mesh
}

/// A point on the Catmull Rom spline through `points`, `t` from 0 at the
/// first point to 1 at the last, so a few control points give a smooth
/// profile.
fn spline(points: &[(f32, f32)], t: f32) -> (f32, f32) {
    let last = points.len() - 1;
    let position = t * float(last);
    let segment = (0..last).find(|&i| position < float(i + 1)).unwrap_or(last - 1);
    let local = position - float(segment);
    let at = |i: usize| points[i.min(last)];
    let (p0, p1, p2, p3) = (
        at(segment.saturating_sub(1)),
        at(segment),
        at(segment + 1),
        at(segment + 2),
    );
    let blend = |a: f32, b: f32, c: f32, d: f32| {
        0.5 * (2.0 * b
            + (c - a) * local
            + (2.0 * a - 5.0 * b + 4.0 * c - d) * local * local
            + (3.0 * b - a - 3.0 * c + d) * local * local * local)
    };
    (blend(p0.0, p1.0, p2.0, p3.0), blend(p0.1, p1.1, p2.1, p3.1))
}

/// A vase turned on a lathe: a profile of radius and height spun around
/// the y axis. It runs from the middle of the foot out and up the outside
/// over the belly, the neck and the rim, then down the inside of the neck
/// to the middle again, so the solid is closed and the mouth is open.
pub fn vase() -> MeshData {
    const STEPS: usize = 160;
    const SIDES: usize = 64;
    const PROFILE: [(f32, f32); 14] = [
        (0.0, 0.0),
        (0.42, 0.0),
        (0.46, 0.1),
        (0.4, 0.25),
        (0.72, 0.8),
        (0.8, 1.3),
        (0.58, 2.0),
        (0.3, 2.5),
        (0.27, 2.85),
        (0.44, 3.15),
        (0.46, 3.22),
        (0.36, 3.1),
        (0.2, 2.85),
        (0.0, 2.7),
    ];

    let mut mesh = MeshData::default();
    for step in 0..=STEPS {
        let s = ratio(step, STEPS);
        let (r, y) = spline(&PROFILE, s);
        let ahead = spline(&PROFILE, (s + 1e-3).min(1.0));
        let behind = spline(&PROFILE, (s - 1e-3).max(0.0));
        let (dr, dy) = (ahead.0 - behind.0, ahead.1 - behind.1);
        for side in 0..SIDES {
            let angle = TAU * ratio(side, SIDES);
            let around = Vec3::new(angle.cos(), 0.0, angle.sin());
            // Out of the profile's own direction: the outside faces away
            // from the axis, the inside of the neck faces it.
            let normal = (around * dy - Vec3::Y * dr).normalize_or(Vec3::NEG_Y);
            mesh.vertices.push(vertex(around * r.max(0.0) + Vec3::Y * y, normal));
        }
    }
    grid_indices(&mut mesh, STEPS + 1, SIDES, false);
    mesh
}

/// A quad `[low, high, low next, high next]` with one normal for all four
/// corners, so a face bent by the twist still reads as one facet.
fn flat_quad(mesh: &mut MeshData, [a, b, c, d]: [Vec3; 4]) {
    let normal = (b - c).cross(d - a).normalize();
    let first = index(mesh.vertices.len());
    mesh.vertices.extend([a, b, c, d].map(|pos| vertex(pos, normal)));
    mesh.indices
        .extend([first, first + 1, first + 2, first + 2, first + 1, first + 3]);
}

/// A five pointed star pushed up in many thin layers, each turned a
/// little more than the one below, so its ten sides spiral up the column
/// like a twisted drill. Each side is flat across, with a flat cap on
/// each end.
pub fn twisted_star() -> MeshData {
    const POINTS: usize = 5;
    const LAYERS: usize = 40;
    const HEIGHT: f32 = 3.0;
    const TWIST: f32 = 2.4;

    let outline = |layer: usize| -> Vec<Vec3> {
        let y = HEIGHT * ratio(layer, LAYERS);
        let turn = TWIST * ratio(layer, LAYERS);
        (0..POINTS * 2)
            .map(|k| {
                let angle = TAU * ratio(k, POINTS * 2) + turn;
                let r = if k % 2 == 0 { 0.8 } else { 0.38 };
                Vec3::new(r * angle.cos(), y, r * angle.sin())
            })
            .collect()
    };

    let mut mesh = MeshData::default();
    let rings: Vec<Vec<Vec3>> = (0..=LAYERS).map(outline).collect();
    for layer in 0..LAYERS {
        let (low, high) = (&rings[layer], &rings[layer + 1]);
        for k in 0..low.len() {
            let next = (k + 1) % low.len();
            flat_quad(&mut mesh, [low[k], high[k], low[next], high[next]]);
        }
    }
    let (bottom, top) = (&rings[0], &rings[LAYERS]);
    let (bottom_center, top_center) = (Vec3::ZERO, Vec3::Y * HEIGHT);
    for k in 0..bottom.len() {
        let next = (k + 1) % bottom.len();
        flat_triangle(&mut mesh, [bottom_center, bottom[k], bottom[next]], WHITE);
        flat_triangle(&mut mesh, [top_center, top[next], top[k]], WHITE);
    }
    mesh
}

/// A boulder: an icosahedron split three times into 1280 faces, pushed in
/// and out by a few slow waves and a little grain, squashed low and shaded
/// flat.
pub fn rock() -> MeshData {
    let (corners, faces) = icosphere(3);

    // Split edges were added once per face that shares them, so the same
    // point has to get the same push, which a function of its position
    // gives.
    let bump = |p: Vec3| {
        let waves = 0.16 * (2.1 * p.x + 0.7).sin() * (1.7 * p.z - 0.4).cos()
            + 0.1 * (3.3 * p.y + 1.9 * p.x).sin()
            + 0.06 * (5.1 * p.z - 2.3 * p.y).sin();
        let grain = (p.dot(Vec3::new(12.9898, 78.233, 37.719)).sin() * 43758.547).fract().abs();
        1.0 + waves + 0.03 * grain
    };
    let squash = Vec3::new(1.0, 0.7, 1.0);
    let pushed: Vec<Vec3> = corners.iter().map(|&p| p * bump(p) * squash).collect();

    let mut mesh = MeshData::default();
    for [a, b, c] in faces {
        let triangle = [pushed[a], pushed[b], pushed[c]];
        let normal = (triangle[1] - triangle[0]).cross(triangle[2] - triangle[0]);
        let center = (triangle[0] + triangle[1] + triangle[2]) / 3.0;
        // A deep push can turn a face over, keep every face looking out.
        if normal.dot(center) < 0.0 {
            flat_triangle(&mut mesh, [triangle[0], triangle[2], triangle[1]], WHITE);
        } else {
            flat_triangle(&mut mesh, triangle, WHITE);
        }
    }
    mesh
}
