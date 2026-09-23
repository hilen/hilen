//! Small builders the scene tests share for meshes made in code.

use std::f32::consts::{FRAC_PI_2, TAU};

use hilen::{
    gm::{
        LossyConvert,
        color::U8Color,
        volume::{Mat4, Vec3, Vertex3D},
    },
    scene::MeshData,
    ui::Point,
};

pub const WHITE: U8Color = U8Color::const_rgb(255, 255, 255);

pub fn index(i: usize) -> u32 {
    u32::try_from(i).expect("a test shape has few vertices")
}

pub fn float(i: usize) -> f32 {
    f32::from(u16::try_from(i).expect("a test shape has few steps"))
}

pub fn ratio(i: usize, count: usize) -> f32 {
    float(i) / float(count)
}

/// A fixed pseudo random number in 0 to 1 for a point, so the same point
/// always gets the same one.
pub fn hash(p: Vec3) -> f32 {
    (p.dot(Vec3::new(12.9898, 78.233, 37.719)).sin() * 43758.547).fract().abs()
}

/// `color` with every channel scaled by `factor`.
pub fn shade(color: U8Color, factor: f32) -> U8Color {
    let channel = |c: u8| -> u8 { (f32::from(c) * factor).round().clamp(0.0, 255.0).lossy_convert() };
    U8Color::const_rgb(channel(color.r), channel(color.g), channel(color.b))
}

/// `color` a little lighter or darker, picked by the point, so a surface
/// of one material reads as many hand placed facets.
pub fn vary(color: U8Color, at: Vec3, amount: f32) -> U8Color {
    shade(color, 1.0 + amount * (hash(at) * 2.0 - 1.0))
}

pub fn vertex(pos: Vec3, normal: Vec3, color: U8Color) -> Vertex3D {
    Vertex3D {
        color,
        ..Vertex3D::new(pos, normal, Point::default())
    }
}

/// A triangle with its own three vertices, so it shades flat.
pub fn flat_triangle(mesh: &mut MeshData, [a, b, c]: [Vec3; 3], color: U8Color) {
    let normal = (b - a).cross(c - a).normalize();
    let first = index(mesh.vertices.len());
    mesh.vertices.extend([a, b, c].map(|pos| vertex(pos, normal, color)));
    mesh.indices.extend([first, first + 1, first + 2]);
}

/// Adds `part` to `mesh` moved by `transform`, which may turn and move
/// but not stretch, so the normals only turn.
pub fn append(mesh: &mut MeshData, part: &MeshData, transform: Mat4) {
    let first = index(mesh.vertices.len());
    mesh.vertices.extend(part.vertices.iter().map(|v| Vertex3D {
        pos: transform.transform_point3(v.pos),
        normal: transform.transform_vector3(v.normal).normalize(),
        ..*v
    }));
    mesh.indices.extend(part.indices.iter().map(|i| first + i));
}

/// The ring of `sides` corners at `height`, starting at angle `turn`.
pub fn ring(sides: usize, radius: f32, height: f32, turn: f32) -> Vec<Vec3> {
    (0..sides)
        .map(|k| {
            let angle = TAU * ratio(k, sides) + turn;
            Vec3::new(radius * angle.cos(), height, radius * angle.sin())
        })
        .collect()
}

/// A prism with `sides` flat sides from radius `bottom` at y zero to
/// radius `top` at `height`, each side colored by `side`, with a flat cap
/// on each end that has a color. Zero at `top` makes a cone.
pub struct Frustum {
    pub sides:  usize,
    pub bottom: f32,
    pub top:    f32,
    pub height: f32,
}

impl Frustum {
    pub fn build(
        &self,
        side: impl Fn(usize) -> U8Color,
        top_cap: Option<U8Color>,
        bottom_cap: Option<U8Color>,
    ) -> MeshData {
        let low = ring(self.sides, self.bottom, 0.0, 0.0);
        let high = ring(self.sides, self.top, self.height, 0.0);
        let mut mesh = MeshData::default();
        for k in 0..self.sides {
            let next = (k + 1) % self.sides;
            let color = side(k);
            flat_triangle(&mut mesh, [low[k], high[k], low[next]], color);
            if self.top > 0.0 {
                flat_triangle(&mut mesh, [low[next], high[k], high[next]], color);
            }
            if let Some(color) = top_cap {
                flat_triangle(&mut mesh, [Vec3::Y * self.height, high[next], high[k]], color);
            }
            if let Some(color) = bottom_cap {
                flat_triangle(&mut mesh, [Vec3::ZERO, low[k], low[next]], color);
            }
        }
        mesh
    }
}

/// A smooth capsule standing on y around its middle, `height` from tip
/// to tip: a half ball under a straight part under a half ball.
pub fn capsule(radius: f32, height: f32, color: U8Color) -> MeshData {
    const SIDES: usize = 24;
    const CAP_STEPS: usize = 8;
    let reach = (height / 2.0 - radius).max(0.0);

    // Up the profile, pole to pole, each step an angle from the equator
    // and the middle of the half ball it belongs to.
    let profile = (0..=CAP_STEPS)
        .map(|step| (-FRAC_PI_2 + FRAC_PI_2 * ratio(step, CAP_STEPS), -reach))
        .chain((0..=CAP_STEPS).map(|step| (FRAC_PI_2 * ratio(step, CAP_STEPS), reach)));

    let mut mesh = MeshData::default();
    let mut rows = 0;
    for (lift, middle) in profile {
        for side in 0..SIDES {
            let angle = TAU * ratio(side, SIDES);
            let around = Vec3::new(angle.cos(), 0.0, angle.sin());
            let normal = around * lift.cos() + Vec3::Y * lift.sin();
            mesh.vertices.push(vertex(normal * radius + Vec3::Y * middle, normal, color));
        }
        rows += 1;
    }
    for row in 0..rows - 1 {
        for side in 0..SIDES {
            let at = |r: usize, s: usize| index(r * SIDES + s % SIDES);
            let (a, b, c, d) = (
                at(row, side),
                at(row + 1, side),
                at(row, side + 1),
                at(row + 1, side + 1),
            );
            mesh.indices.extend([a, b, c, c, b, d]);
        }
    }
    mesh
}

/// onto the sphere. Faces are corner indices, wound outward.
pub fn icosphere(splits: usize) -> (Vec<Vec3>, Vec<[usize; 3]>) {
    let t = f32::midpoint(1.0, 5f32.sqrt());
    let mut corners: Vec<Vec3> = [
        (-1.0, t, 0.0),
        (1.0, t, 0.0),
        (-1.0, -t, 0.0),
        (1.0, -t, 0.0),
        (0.0, -1.0, t),
        (0.0, 1.0, t),
        (0.0, -1.0, -t),
        (0.0, 1.0, -t),
        (t, 0.0, -1.0),
        (t, 0.0, 1.0),
        (-t, 0.0, -1.0),
        (-t, 0.0, 1.0),
    ]
    .map(|(x, y, z)| Vec3::new(x, y, z).normalize())
    .to_vec();
    let mut faces: Vec<[usize; 3]> = vec![
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];

    for _ in 0..splits {
        let mut split = Vec::with_capacity(faces.len() * 4);
        for [a, b, c] in faces {
            let mut middle = |p: usize, q: usize| {
                corners.push(((corners[p] + corners[q]) / 2.0).normalize());
                corners.len() - 1
            };
            let (ab, bc, ca) = (middle(a, b), middle(b, c), middle(c, a));
            split.extend([[a, ab, ca], [b, bc, ab], [c, ca, bc], [ab, bc, ca]]);
        }
        faces = split;
    }

    (corners, faces)
}
