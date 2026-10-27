use std::f32::consts::{PI, TAU};

use hilen::{
    gm::{
        LossyConvert,
        color::U8Color,
        volume::{Vec3, Vertex3D},
    },
    scene::MeshData,
    ui::Point,
};

use crate::geometry::{index, ratio};

const RINGS: usize = 8;
const SEGMENTS: usize = 12;

/// A small fixed sequence, the same world on every run.
pub fn pick(i: usize, salt: u32) -> f32 {
    let i = u32::try_from(i).expect("a test world has few spots");
    let mut x = i.wrapping_mul(2_654_435_761) ^ salt.wrapping_mul(40_503);
    x ^= x >> 15;
    x = x.wrapping_mul(2_246_822_519);
    x ^= x >> 13;
    f32::from(u16::try_from(x % 10_000).expect("below 10 000")) / 10_000.0
}

/// A ball stretched by `radii` along each axis.
pub fn push_ellipsoid(mesh: &mut MeshData, center: Vec3, radii: Vec3, color: U8Color) {
    let first = index(mesh.vertices.len());
    for ring in 0..=RINGS {
        let polar = PI * ratio(ring, RINGS);
        for segment in 0..=SEGMENTS {
            let around = TAU * ratio(segment, SEGMENTS);
            let unit = Vec3::new(
                polar.sin() * around.cos(),
                polar.cos(),
                polar.sin() * around.sin(),
            );
            mesh.vertices.push(Vertex3D {
                color,
                ..Vertex3D::new(
                    center + unit * radii,
                    (unit / radii).normalize(),
                    Point::default(),
                )
            });
        }
    }
    let row = index(SEGMENTS + 1);
    for ring in 0..index(RINGS) {
        for segment in 0..index(SEGMENTS) {
            let a = first + ring * row + segment;
            let b = a + row;
            mesh.indices.extend([a, a + 1, b, a + 1, b + 1, b]);
        }
    }
}

/// The 4 sides of a square trunk standing on the origin.
fn push_trunk(mesh: &mut MeshData, height: f32, width: f32, color: U8Color) {
    let half = width / 2.0;
    for (normal, u) in [
        (Vec3::X, Vec3::NEG_Z),
        (Vec3::NEG_X, Vec3::Z),
        (Vec3::Z, Vec3::X),
        (Vec3::NEG_Z, Vec3::NEG_X),
    ] {
        let first = index(mesh.vertices.len());
        let center = normal * half;
        for (a, b) in [(-1.0, 0.0), (1.0, 0.0), (1.0, 1.0), (-1.0, 1.0)] {
            let pos = center + u * half * a + Vec3::Y * height * b;
            mesh.vertices.push(Vertex3D {
                color,
                ..Vertex3D::new(pos, normal, Point::default())
            });
        }
        mesh.indices.extend([first, first + 1, first + 2, first, first + 2, first + 3]);
    }
}

/// A trunk with a crown of leaf balls, colored in its vertices.
pub fn tree(shape: usize) -> MeshData {
    let mut mesh = MeshData::default();
    let height = 3.5 + pick(shape, 1) * 4.5;
    push_trunk(
        &mut mesh,
        height,
        0.4 + pick(shape, 2) * 0.4,
        U8Color::const_rgb(92, 63, 40),
    );
    let leaf = U8Color::const_rgb(30, (90.0 + pick(shape, 3) * 80.0).lossy_convert(), 25);
    for ball in 0..10 {
        let seed = shape * 16 + ball;
        let angle = TAU * pick(seed, 4);
        let out = pick(seed, 5) * 1.8;
        let center = Vec3::new(angle.cos() * out, height + pick(seed, 6) * 1.5, angle.sin() * out);
        push_ellipsoid(&mut mesh, center, Vec3::splat(0.9 + pick(seed, 7) * 0.8), leaf);
    }
    mesh
}

/// A fighter jet about 9 m long, its nose along `-z`: a grey body, a dark
/// canopy, swept wings and a fin.
pub fn jet() -> MeshData {
    let metal = U8Color::const_rgb(120, 126, 134);
    let glass = U8Color::const_rgb(20, 40, 70);
    let orange = U8Color::const_rgb(205, 100, 50);
    let mut mesh = MeshData::default();
    push_ellipsoid(&mut mesh, Vec3::ZERO, Vec3::new(0.6, 0.6, 4.5), metal);
    push_ellipsoid(
        &mut mesh,
        Vec3::new(0.0, 0.45, -1.8),
        Vec3::new(0.35, 0.3, 1.2),
        glass,
    );
    push_ellipsoid(
        &mut mesh,
        Vec3::new(0.0, -0.1, 0.8),
        Vec3::new(4.5, 0.08, 1.4),
        metal,
    );
    push_ellipsoid(
        &mut mesh,
        Vec3::new(0.0, 0.0, 3.4),
        Vec3::new(1.8, 0.06, 0.6),
        metal,
    );
    push_ellipsoid(
        &mut mesh,
        Vec3::new(0.0, 0.9, 3.3),
        Vec3::new(0.06, 1.0, 0.7),
        orange,
    );
    mesh
}
