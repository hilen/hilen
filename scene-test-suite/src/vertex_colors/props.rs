//! The props of `Vertex colors`, low poly and flat shaded, every face
//! colored on its own with a little variation so no two read the same.

use std::{f32::consts::TAU, ops::Range};

use hilen::{
    gm::{
        color::U8Color,
        volume::{Mat4, Quat, Vec3},
    },
    scene::MeshData,
};

use crate::geometry::{Frustum, append, flat_triangle, float, hash, icosphere, ratio, ring, vary};

const fn rgb(r: u8, g: u8, b: u8) -> U8Color {
    U8Color::const_rgb(r, g, b)
}

const BARK: U8Color = rgb(92, 63, 40);
const CUT_WOOD: U8Color = rgb(214, 172, 118);

/// Where the campfire burns, the ground under it is bare dirt.
pub const FIRE: Vec3 = Vec3::new(0.5, 0.0, 1.0);

/// Faceted grass, bare around the fire, with a few lighter tufts.
pub fn ground() -> MeshData {
    const CELLS: usize = 96;
    const SIZE: f32 = 24.0;

    let point = |column: usize, row: usize| {
        let x = SIZE * ratio(column, CELLS) - SIZE / 2.0;
        let z = SIZE * ratio(row, CELLS) - SIZE / 2.0;
        let bump = 0.03 * (0.9 * x + 0.3).sin() * (0.7 * z).cos() + 0.02 * hash(Vec3::new(x, 0.0, z));
        Vec3::new(x, bump, z)
    };
    let color = |center: Vec3| {
        let from_fire = Vec3::new(center.x - FIRE.x, 0.0, center.z - FIRE.z).length();
        let grain = hash(center * 3.1);
        if from_fire < 0.75 || (from_fire < 1.0 && grain > 0.5) {
            vary(rgb(110, 90, 67), center, 0.12)
        } else if grain > 0.88 {
            vary(rgb(121, 168, 74), center, 0.08)
        } else {
            vary(rgb(93, 140, 58), center, 0.1)
        }
    };

    let mut mesh = MeshData::default();
    for row in 0..CELLS {
        for column in 0..CELLS {
            let (a, b) = (point(column, row), point(column, row + 1));
            let (c, d) = (point(column + 1, row), point(column + 1, row + 1));
            // The diagonal flips from cell to cell, so the facets do not
            // line up into stripes.
            let triangles = if (row + column) % 2 == 0 {
                [[a, b, c], [c, b, d]]
            } else {
                [[a, b, d], [a, d, c]]
            };
            for triangle in triangles {
                let center = (triangle[0] + triangle[1] + triangle[2]) / 3.0;
                flat_triangle(&mut mesh, triangle, color(center));
            }
        }
    }
    mesh
}

/// A band of a flat round top between two radii, facing up.
fn top_band(mesh: &mut MeshData, sides: usize, inner: f32, outer: f32, y: f32, color: U8Color) {
    let (inside, outside) = (ring(sides, inner, y, 0.0), ring(sides, outer, y, 0.0));
    for k in 0..sides {
        let next = (k + 1) % sides;
        flat_triangle(
            mesh,
            [inside[k], outside[next], outside[k]],
            vary(color, outside[k], 0.06),
        );
        flat_triangle(
            mesh,
            [inside[k], inside[next], outside[next]],
            vary(color, inside[k], 0.06),
        );
    }
}

/// A chopping stump: bark down the sides in darker and lighter strips,
/// and the growth rings on the cut top.
pub fn stump() -> MeshData {
    const SIDES: usize = 11;
    const HEIGHT: f32 = 0.5;
    const TOP: f32 = 0.52;

    let shape = Frustum {
        sides:  SIDES,
        bottom: 0.6,
        top:    TOP,
        height: HEIGHT,
    };
    let mut mesh = shape.build(
        |k| {
            vary(
                if k % 2 == 0 { BARK } else { rgb(74, 50, 32) },
                Vec3::X * float(k),
                0.12,
            )
        },
        None,
        None,
    );

    let bands = [
        (0.47, TOP, BARK),
        (0.38, 0.47, CUT_WOOD),
        (0.3, 0.38, rgb(196, 150, 96)),
        (0.21, 0.3, CUT_WOOD),
        (0.12, 0.21, rgb(196, 150, 96)),
    ];
    for (inner, outer, color) in bands {
        top_band(&mut mesh, SIDES, inner, outer, HEIGHT, color);
    }
    let heart = ring(SIDES, 0.12, HEIGHT, 0.0);
    for k in 0..SIDES {
        let next = (k + 1) % SIDES;
        flat_triangle(
            &mut mesh,
            [Vec3::Y * HEIGHT, heart[next], heart[k]],
            rgb(176, 127, 74),
        );
    }
    mesh
}

/// The blade's outline seen from the side, counter clockwise, the edge
/// facing +x: the poll behind the handle, the beard hooked down under
/// the neck and the long curved edge.
const HEAD: [(f32, f32); 10] = [
    (-0.1, 0.1),
    (-0.1, -0.08),
    (0.12, -0.07),
    (0.22, -0.22),
    (0.33, -0.34),
    (0.4, -0.18),
    (0.43, 0.0),
    (0.4, 0.14),
    (0.3, 0.2),
    (0.12, 0.09),
];
/// The outline segments from `HEAD[4]` to `HEAD[7]` are the honed edge.
const EDGE: Range<usize> = 4..7;
/// Where the head sits on the handle.
const HEAD_HEIGHT: f32 = 0.86;
/// The middle of the edge in the axe's own space, what bites the stump.
pub const AXE_EDGE: Vec3 = Vec3::new(0.43, HEAD_HEIGHT, 0.0);

/// A bearded axe standing on its handle, the head sticking out along +x.
pub fn axe() -> MeshData {
    let mut mesh = MeshData::default();

    let wood = rgb(154, 106, 60);
    let knob = Frustum {
        sides:  8,
        bottom: 0.05,
        top:    0.042,
        height: 0.06,
    };
    append(
        &mut mesh,
        &knob.build(
            |k| vary(wood, Vec3::Y * float(k), 0.1),
            None,
            Some(rgb(120, 80, 45)),
        ),
        Mat4::IDENTITY,
    );
    let shaft = Frustum {
        sides:  8,
        bottom: 0.034,
        top:    0.028,
        height: 0.96,
    };
    append(
        &mut mesh,
        &shaft.build(|k| vary(wood, Vec3::Z * float(k), 0.1), Some(CUT_WOOD), None),
        Mat4::from_translation(Vec3::Y * 0.05),
    );
    // A leather strap wound around the grip, one turn per ring.
    for turn in 0..8 {
        let strap = Frustum {
            sides:  8,
            bottom: 0.041,
            top:    0.04,
            height: 0.032,
        };
        let color = if turn % 2 == 0 {
            rgb(74, 50, 34)
        } else {
            rgb(94, 65, 44)
        };
        let part = strap.build(
            |k| vary(color, Vec3::new(float(turn), float(k), 0.0), 0.08),
            None,
            None,
        );
        let at = Mat4::from_rotation_translation(
            Quat::from_rotation_y(0.2 * float(turn)),
            Vec3::Y * (0.09 + 0.033 * float(turn)),
        );
        append(&mut mesh, &part, at);
    }

    head(&mut mesh);
    mesh
}

/// Half the thickness of the head at `x`: thick at the poll, thinning to
/// almost nothing at the edge.
fn thickness(x: f32) -> f32 {
    0.055 * ((0.44 - x) / 0.34).clamp(0.06, 1.0)
}

fn head(mesh: &mut MeshData) {
    let iron = rgb(84, 91, 99);
    let honed = rgb(185, 193, 200);
    let point = |(x, y): (f32, f32), side: f32| Vec3::new(x, y + HEAD_HEIGHT, side * thickness(x));
    // The outline is star shaped around this point, every corner in
    // plain sight of it, so a fan fills each face.
    let center = (0.14, -0.02);

    for (k, &a) in HEAD.iter().enumerate() {
        let b = HEAD[(k + 1) % HEAD.len()];
        let on_edge = EDGE.contains(&k);
        let face = if on_edge { honed } else { iron };

        let front = [point(center, 1.0), point(a, 1.0), point(b, 1.0)];
        flat_triangle(mesh, front, vary(face, front[1], 0.06));
        let back = [point(center, -1.0), point(b, -1.0), point(a, -1.0)];
        flat_triangle(mesh, back, vary(face, back[2] + Vec3::Z, 0.06));

        let rim = if on_edge {
            rgb(223, 229, 234)
        } else {
            rgb(69, 75, 82)
        };
        let (front_a, front_b) = (point(a, 1.0), point(b, 1.0));
        let (back_a, back_b) = (point(a, -1.0), point(b, -1.0));
        flat_triangle(mesh, [front_a, back_a, front_b], rim);
        flat_triangle(mesh, [front_b, back_a, back_b], rim);
    }
}

/// A fire ring: lumpy stones around an ash bed, logs leaned into a
/// teepee with pale cut ends and charred tips, and the flames inside.
pub fn campfire() -> MeshData {
    let mut mesh = MeshData::default();

    let ash = Frustum {
        sides:  9,
        bottom: 0.4,
        top:    0.38,
        height: 0.015,
    };
    append(
        &mut mesh,
        &ash.build(|_| rgb(58, 53, 50), Some(rgb(47, 43, 41)), None),
        Mat4::IDENTITY,
    );

    for i in 0..9 {
        let angle = TAU * ratio(i, 9) + 0.2;
        let place = Mat4::from_rotation_translation(
            Quat::from_rotation_y(-angle),
            Vec3::new(0.56 * angle.cos(), 0.05, 0.56 * angle.sin()),
        );
        append(&mut mesh, &stone(float(i)), place);
    }

    for i in 0..5 {
        let angle = TAU * ratio(i, 5) + 0.5;
        let (sin, cos) = angle.sin_cos();
        let foot = Vec3::new(0.46 * cos, 0.03, 0.46 * sin);
        let tip = Vec3::new(0.05 * cos, 0.52, 0.05 * sin);
        let log = Frustum {
            sides:  6,
            bottom: 0.06,
            top:    0.052,
            height: (tip - foot).length(),
        };
        let bark = |k: usize| vary(BARK, Vec3::new(float(i), float(k), 1.0), 0.14);
        let part = log.build(bark, Some(rgb(43, 37, 34)), Some(CUT_WOOD));
        let lean = Quat::from_rotation_arc(Vec3::Y, (tip - foot).normalize());
        append(&mut mesh, &part, Mat4::from_rotation_translation(lean, foot));
    }

    let flames = [
        (0.2, 0.85, rgb(240, 138, 36), Vec3::new(0.0, 0.0, 0.0)),
        (0.15, 0.62, rgb(229, 86, 30), Vec3::new(0.08, 0.0, 0.06)),
        (0.12, 0.55, rgb(255, 210, 63), Vec3::new(-0.06, 0.0, -0.03)),
    ];
    for (radius, height, color, offset) in flames {
        let flame = Frustum {
            sides: 5,
            bottom: radius,
            top: 0.0,
            height,
        };
        let part = flame.build(|k| vary(color, Vec3::splat(float(k)) + offset, 0.06), None, None);
        append(&mut mesh, &part, Mat4::from_translation(offset + Vec3::Y * 0.015));
    }
    mesh
}

/// A fist sized stone, an icosahedron split once and squashed unevenly.
fn stone(seed: f32) -> MeshData {
    let (corners, faces) = icosphere(1);
    let lump = |p: Vec3| p * Vec3::new(0.15, 0.1, 0.12) * (0.85 + 0.3 * hash(p + Vec3::splat(seed)));
    let mut mesh = MeshData::default();
    for [a, b, c] in faces {
        let triangle = [lump(corners[a]), lump(corners[b]), lump(corners[c])];
        let center = (triangle[0] + triangle[1] + triangle[2]) / 3.0;
        // A deep push can turn a face over, keep every face looking out.
        let triangle = if (triangle[1] - triangle[0]).cross(triangle[2] - triangle[0]).dot(center) < 0.0 {
            [triangle[0], triangle[2], triangle[1]]
        } else {
            triangle
        };
        let grey = if center.y > 0.04 {
            rgb(150, 146, 139)
        } else {
            rgb(122, 119, 113)
        };
        flat_triangle(&mut mesh, triangle, vary(grey, center + Vec3::splat(seed), 0.12));
    }
    mesh
}

/// A pine: a short trunk under four tiers of needles, each tier a cone
/// with a ragged hem, turned against the one below, darker underneath.
pub fn pine() -> MeshData {
    let mut mesh = MeshData::default();
    let trunk = Frustum {
        sides:  6,
        bottom: 0.13,
        top:    0.09,
        height: 0.7,
    };
    append(
        &mut mesh,
        &trunk.build(|k| vary(BARK, Vec3::Y * float(k), 0.14), None, None),
        Mat4::IDENTITY,
    );

    let greens = [rgb(47, 107, 58), rgb(58, 125, 68), rgb(40, 89, 58)];
    let tiers = [
        (0.45, 0.95, 1.0),
        (0.95, 0.78, 0.9),
        (1.4, 0.6, 0.8),
        (1.8, 0.4, 0.75),
    ];
    for (tier, (base, radius, height)) in tiers.into_iter().enumerate() {
        const SIDES: usize = 8;
        let apex = Vec3::Y * (base + height);
        let hem: Vec<Vec3> = ring(SIDES, radius, base, 0.4 * float(tier))
            .into_iter()
            .enumerate()
            .map(|(k, p)| {
                let jitter = hash(Vec3::new(float(k), float(tier), 0.5));
                Vec3::new(
                    p.x * (0.85 + 0.25 * jitter),
                    base - 0.08 * jitter,
                    p.z * (0.85 + 0.25 * jitter),
                )
            })
            .collect();
        for k in 0..SIDES {
            let next = (k + 1) % SIDES;
            let pick = hash(hem[k] * 1.7);
            let green = greens[usize::from(pick > 0.33) + usize::from(pick > 0.66)];
            flat_triangle(&mut mesh, [hem[k], apex, hem[next]], vary(green, hem[k], 0.08));
            let under = Vec3::Y * (base + 0.05);
            flat_triangle(&mut mesh, [under, hem[k], hem[next]], rgb(31, 74, 45));
        }
    }
    mesh
}
