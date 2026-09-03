//! Cube map math for the scene sky: the six faces in wgpu's layer order
//! `+x -x +y -y +z -z`, a gradient sky, the GGX prefilter that fills the
//! reflection mips and the spherical harmonics of the irradiance. All of
//! it in linear light, the bytes at the edges are encoded sRGB.

use std::f32::consts::PI;

use glam::Vec3;

/// Mip levels of the reflection cube past the source, the last one is
/// fully rough. The mesh shader picks `roughness * ROUGH_LEVELS`.
pub const ROUGH_LEVELS: u16 = 5;

/// GGX samples per texel of a prefiltered level.
const SAMPLES: u16 = 64;

/// A cube map in linear light, the working form of everything here.
#[derive(Clone)]
pub struct LinearCube {
    pub size:  usize,
    pub faces: [Vec<Vec3>; 6],
}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// A texel coordinate or a face size as a float, exact up to the widest
/// face a cube can have.
fn coord(value: usize) -> f32 {
    f32::from(u16::try_from(value).expect("a cube face is at most 65535 texels wide"))
}

/// The direction through the center of texel `(x, y)` of `face`, the
/// same layout every GPU samples a cube with: `+x` looks down `-z` with
/// `y` up in the image, and so on around the cube.
pub fn texel_direction(face: usize, x: usize, y: usize, size: usize) -> Vec3 {
    let s = (coord(x) + 0.5) / coord(size) * 2.0 - 1.0;
    let t = (coord(y) + 0.5) / coord(size) * 2.0 - 1.0;
    face_direction(face, s, t).normalize()
}

/// `s` and `t` run from -1 to 1 across the face, `t` down the image.
fn face_direction(face: usize, s: f32, t: f32) -> Vec3 {
    match face {
        0 => Vec3::new(1.0, -t, -s),
        1 => Vec3::new(-1.0, -t, s),
        2 => Vec3::new(s, 1.0, t),
        3 => Vec3::new(s, -1.0, -t),
        4 => Vec3::new(s, -t, 1.0),
        _ => Vec3::new(-s, -t, -1.0),
    }
}

/// The face a direction lands on and where on it, `u` and `v` in 0 to 1.
pub fn face_uv(dir: Vec3) -> (usize, f32, f32) {
    let abs = dir.abs();
    let (face, sc, tc, ma) = if abs.x >= abs.y && abs.x >= abs.z {
        if dir.x > 0.0 {
            (0, -dir.z, -dir.y, abs.x)
        } else {
            (1, dir.z, -dir.y, abs.x)
        }
    } else if abs.y >= abs.z {
        if dir.y > 0.0 {
            (2, dir.x, dir.z, abs.y)
        } else {
            (3, dir.x, -dir.z, abs.y)
        }
    } else if dir.z > 0.0 {
        (4, dir.x, -dir.y, abs.z)
    } else {
        (5, -dir.x, -dir.y, abs.z)
    };
    (face, (sc / ma).midpoint(1.0), (tc / ma).midpoint(1.0))
}

fn smoothstep(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// A smooth sky, `zenith` straight up through `horizon` at eye level to
/// `ground` straight down, linear light.
pub fn sky_gradient(size: usize, zenith: Vec3, horizon: Vec3, ground: Vec3) -> LinearCube {
    let faces = std::array::from_fn(|face| {
        let mut texels = Vec::with_capacity(size * size);
        for y in 0..size {
            for x in 0..size {
                let up = texel_direction(face, x, y, size).y;
                let color = if up >= 0.0 {
                    horizon.lerp(zenith, smoothstep(up))
                } else {
                    horizon.lerp(ground, smoothstep(-up))
                };
                texels.push(color);
            }
        }
        texels
    });
    LinearCube { size, faces }
}

impl LinearCube {
    /// From six square faces of encoded sRGB RGBA bytes, `size` texels
    /// wide each.
    pub fn from_bytes(size: usize, faces: &[Vec<u8>; 6]) -> Self {
        let faces = std::array::from_fn(|face| {
            assert_eq!(faces[face].len(), size * size * 4, "a cube face is square RGBA");
            faces[face]
                .as_chunks::<4>()
                .0
                .iter()
                .map(|px| {
                    Vec3::new(
                        srgb_to_linear(f32::from(px[0]) / 255.0),
                        srgb_to_linear(f32::from(px[1]) / 255.0),
                        srgb_to_linear(f32::from(px[2]) / 255.0),
                    )
                })
                .collect()
        });
        Self { size, faces }
    }

    fn texel(&self, face: usize, x: usize, y: usize) -> Vec3 {
        self.faces[face][y * self.size + x]
    }

    /// Bilinear on the face the direction lands on, clamped at the
    /// face edge.
    fn sample(&self, dir: Vec3) -> Vec3 {
        let (face, u, v) = face_uv(dir);
        let last = coord(self.size - 1);
        let fx = (u * coord(self.size) - 0.5).clamp(0.0, last);
        let fy = (v * coord(self.size) - 0.5).clamp(0.0, last);
        let x0 = floor_index(fx);
        let y0 = floor_index(fy);
        let x1 = (x0 + 1).min(self.size - 1);
        let y1 = (y0 + 1).min(self.size - 1);
        let tx = fx - coord(x0);
        let ty = fy - coord(y0);
        let top = self.texel(face, x0, y0).lerp(self.texel(face, x1, y0), tx);
        let bottom = self.texel(face, x0, y1).lerp(self.texel(face, x1, y1), tx);
        top.lerp(bottom, ty)
    }

    /// Half the size with a 2 by 2 box filter.
    fn halved(&self) -> Self {
        let size = (self.size / 2).max(1);
        let faces = std::array::from_fn(|face| {
            let mut texels = Vec::with_capacity(size * size);
            for y in 0..size {
                for x in 0..size {
                    let x0 = (x * 2).min(self.size - 1);
                    let y0 = (y * 2).min(self.size - 1);
                    let x1 = (x0 + 1).min(self.size - 1);
                    let y1 = (y0 + 1).min(self.size - 1);
                    let sum = self.texel(face, x0, y0)
                        + self.texel(face, x1, y0)
                        + self.texel(face, x0, y1)
                        + self.texel(face, x1, y1);
                    texels.push(sum / 4.0);
                }
            }
            texels
        });
        Self { size, faces }
    }

    fn mip_chain(&self) -> Vec<Self> {
        let mut levels = vec![self.clone()];
        while levels.last().is_some_and(|level| level.size > 1) {
            let next = levels.last().unwrap().halved();
            levels.push(next);
        }
        levels
    }
}

/// The whole part of a float from 0 up to the widest face, no cast, so
/// the pedantic lints stay on in this crate. Adding 2^23 parks the
/// nearest integer in the mantissa, and one step back turns that round
/// into a floor.
fn floor_index(value: f32) -> usize {
    const MAGIC: f32 = 8_388_608.0;
    let rounded = usize::try_from((value + MAGIC).to_bits() - MAGIC.to_bits()).expect("u32 fits usize");
    if coord(rounded) > value {
        rounded - 1
    } else {
        rounded
    }
}

/// The van der Corput radical inverse, the second half of a Hammersley
/// point.
fn radical_inverse(mut i: u16) -> f32 {
    let mut inverse = 0.0;
    let mut digit = 0.5;
    while i > 0 {
        if i & 1 == 1 {
            inverse += digit;
        }
        digit *= 0.5;
        i >>= 1;
    }
    inverse
}

fn hammersley(i: u16, count: u16) -> (f32, f32) {
    (f32::from(i) / f32::from(count), radical_inverse(i))
}

/// The prefiltered reflection levels after the source: level `i` of
/// `1..=ROUGH_LEVELS` holds what a surface of roughness `i / ROUGH_LEVELS`
/// reflects, half the size of the one before, GGX importance sampled
/// with the source mip chain so a bright texel does not sparkle.
pub fn prefilter(source: &LinearCube) -> Vec<LinearCube> {
    assert!(
        source.size >= 1 << ROUGH_LEVELS,
        "a sky face must be at least {} texels wide",
        1 << ROUGH_LEVELS
    );
    let chain = source.mip_chain();
    let texel_solid_angle = 4.0 * PI / (6.0 * coord(source.size) * coord(source.size));

    (1..=ROUGH_LEVELS)
        .map(|level| {
            let roughness = f32::from(level) / f32::from(ROUGH_LEVELS);
            let alpha = roughness * roughness;
            let size = source.size >> level;
            let faces = std::array::from_fn(|face| {
                let mut texels = Vec::with_capacity(size * size);
                for y in 0..size {
                    for x in 0..size {
                        let normal = texel_direction(face, x, y, size);
                        texels.push(prefilter_texel(&chain, normal, alpha, texel_solid_angle));
                    }
                }
                texels
            });
            LinearCube { size, faces }
        })
        .collect()
}

/// The split sum assumption, the view and the normal both along the
/// reflection direction.
fn prefilter_texel(chain: &[LinearCube], normal: Vec3, alpha: f32, texel_solid_angle: f32) -> Vec3 {
    let up = if normal.z.abs() < 0.999 { Vec3::Z } else { Vec3::X };
    let tangent = up.cross(normal).normalize();
    let bitangent = normal.cross(tangent);

    let mut sum = Vec3::ZERO;
    let mut weight = 0.0;

    for i in 0..SAMPLES {
        let (u, v) = hammersley(i, SAMPLES);
        let phi = 2.0 * PI * u;
        let cos_theta = ((1.0 - v) / (1.0 + (alpha * alpha - 1.0) * v)).sqrt();
        let sin_theta = (1.0 - cos_theta * cos_theta).max(0.0).sqrt();
        let half =
            tangent * (sin_theta * phi.cos()) + bitangent * (sin_theta * phi.sin()) + normal * cos_theta;
        let light = half * (2.0 * normal.dot(half)) - normal;
        let n_dot_l = normal.dot(light);
        if n_dot_l <= 0.0 {
            continue;
        }
        let n_dot_h = cos_theta;
        let d = ggx(n_dot_h, alpha);
        let pdf = d / 4.0;
        let sample_solid_angle = 1.0 / (f32::from(SAMPLES) * pdf + 1e-6);
        let lod = (0.5 * (sample_solid_angle / texel_solid_angle).log2() + 1.0).max(0.0);
        let level = &chain[floor_index(lod + 0.5).min(chain.len() - 1)];
        sum += level.sample(light) * n_dot_l;
        weight += n_dot_l;
    }

    sum / weight.max(1e-6)
}

fn ggx(n_dot_h: f32, alpha: f32) -> f32 {
    let a2 = alpha * alpha;
    let f = (n_dot_h * a2 - n_dot_h) * n_dot_h + 1.0;
    a2 / (PI * f * f)
}

/// The nine spherical harmonics of the diffuse light a surface gets from
/// the cube, already scaled so `sum(coefficient * basis(normal))` is the
/// color a white matte surface with that normal shows.
pub fn irradiance(cube: &LinearCube) -> [Vec3; 9] {
    let mut coefficients = [Vec3::ZERO; 9];
    let size = cube.size;
    for face in 0..6 {
        for y in 0..size {
            for x in 0..size {
                let s = (coord(x) + 0.5) / coord(size) * 2.0 - 1.0;
                let t = (coord(y) + 0.5) / coord(size) * 2.0 - 1.0;
                let solid_angle = 4.0 / ((1.0 + s * s + t * t).powf(1.5) * coord(size) * coord(size));
                let dir = texel_direction(face, x, y, size);
                let color = cube.texel(face, x, y) * solid_angle;
                for (coefficient, basis) in coefficients.iter_mut().zip(sh_basis(dir)) {
                    *coefficient += color * basis;
                }
            }
        }
    }
    // The cosine lobe per band, over pi for the Lambert term.
    let bands = [1.0, 2.0 / 3.0, 2.0 / 3.0, 2.0 / 3.0, 0.25, 0.25, 0.25, 0.25, 0.25];
    for (coefficient, band) in coefficients.iter_mut().zip(bands) {
        *coefficient *= band;
    }
    coefficients
}

/// The real spherical harmonics up to band two, the same order and
/// constants the mesh shader evaluates.
pub fn sh_basis(dir: Vec3) -> [f32; 9] {
    let Vec3 { x, y, z } = dir;
    [
        0.282_095,
        0.488_603 * y,
        0.488_603 * z,
        0.488_603 * x,
        1.092_548 * x * y,
        1.092_548 * y * z,
        0.315_392 * (3.0 * z * z - 1.0),
        1.092_548 * x * z,
        0.546_274 * (x * x - y * y),
    ]
}

#[cfg(test)]
mod test {
    use super::*;

    fn constant(size: usize, color: Vec3) -> LinearCube {
        LinearCube {
            size,
            faces: std::array::from_fn(|_| vec![color; size * size]),
        }
    }

    // The GPU samples the faces with this layout, so a texel has to land
    // back on itself through the direction it was built for.
    #[test]
    fn face_uv_round_trips_every_texel() {
        let size = 8;
        for face in 0..6 {
            for y in 0..size {
                for x in 0..size {
                    let (back, u, v) = face_uv(texel_direction(face, x, y, size));
                    assert_eq!(back, face);
                    assert_eq!(floor_index(u * coord(size)), x);
                    assert_eq!(floor_index(v * coord(size)), y);
                }
            }
        }
    }

    #[test]
    fn a_flat_sky_lights_every_normal_the_same() {
        let sky = Vec3::new(0.2, 0.5, 0.8);
        let coefficients = irradiance(&constant(32, sky));
        for dir in [
            Vec3::X,
            Vec3::NEG_Y,
            Vec3::Z,
            Vec3::new(1.0, 1.0, 1.0).normalize(),
        ] {
            let lit: Vec3 = coefficients.iter().zip(sh_basis(dir)).map(|(c, b)| *c * b).sum();
            assert!((lit - sky).length() < 0.01, "{dir:?} gets {lit:?}");
        }
    }

    #[test]
    fn a_flat_sky_stays_flat_through_the_prefilter() {
        let sky = Vec3::new(0.9, 0.4, 0.1);
        let levels = prefilter(&constant(32, sky));
        assert_eq!(levels.len(), usize::from(ROUGH_LEVELS));
        assert_eq!(levels.last().unwrap().size, 1);
        for level in &levels {
            for texel in level.faces.iter().flatten() {
                assert!((*texel - sky).length() < 1e-3);
            }
        }
    }

    #[test]
    fn gradient_is_brightest_up_and_darkest_down() {
        let cube = sky_gradient(32, Vec3::ONE, Vec3::splat(0.5), Vec3::ZERO);
        let top = cube.sample(Vec3::Y);
        let side = cube.sample(Vec3::X);
        let bottom = cube.sample(Vec3::NEG_Y);
        assert!(top.x > 0.99 && (side.x - 0.5).abs() < 0.02 && bottom.x < 0.01);
    }

    #[test]
    fn bytes_decode_the_srgb_curve() {
        let faces: [Vec<u8>; 6] = std::array::from_fn(|_| vec![128, 255, 0, 255]);
        let cube = LinearCube::from_bytes(1, &faces);
        let texel = cube.texel(3, 0, 0);
        assert!((texel - Vec3::new(0.2158, 1.0, 0.0)).length() < 1e-3);
    }

    #[test]
    fn floor_index_and_radical_inverse() {
        assert_eq!(floor_index(0.0), 0);
        assert_eq!(floor_index(0.4), 0);
        assert_eq!(floor_index(0.6), 0);
        assert_eq!(floor_index(2.999), 2);
        assert_eq!(floor_index(3.0), 3);
        assert_eq!(floor_index(3.5), 3);
        assert_eq!(floor_index(65_534.9), 65_534);
        assert!((radical_inverse(1) - 0.5).abs() < 1e-6);
        assert!((radical_inverse(2) - 0.25).abs() < 1e-6);
        assert!((radical_inverse(3) - 0.75).abs() < 1e-6);
    }
}
