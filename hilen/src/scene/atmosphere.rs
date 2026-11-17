//! The air of a planet: the sky seen from its ground and from space, the
//! haze over far land and the color the sun's light takes on its way
//! down. The method of Hillaire 2020, "A Scalable and Production Ready
//! Sky and Atmosphere Rendering Technique": 2 small tables, how much
//! light gets through the air from a height in a direction, and the light
//! scattered more than once, then a short march along each ray. The
//! tables are worked out here on the CPU when the air changes, the same
//! code the ambient light of the scene runs every frame.

use std::f32::consts::PI;

use half::f16;

use crate::gm::{LossyConvert, volume::Vec3};

/// A count or an index of a table as a float, all far under 2^24.
fn float(value: usize) -> f32 {
    value.lossy_convert()
}

/// The transmittance table: the way to the top of the air along x, the
/// height along y.
pub(crate) const TRANSMITTANCE_SIZE: (usize, usize) = (256, 64);
/// The table of light scattered more than once: the sun's height in the
/// sky along x, the height along y.
pub(crate) const MULTI_SIZE: usize = 32;
/// Steps of a ray through the air for a table texel.
const TABLE_STEPS: usize = 40;
/// Directions and steps the light scattered more than once is gathered
/// over at each texel of its table.
const MULTI_DIRECTIONS: usize = 64;
const MULTI_STEPS: usize = 20;
/// Directions the ambient light of the sky is gathered over.
const AMBIENT_DIRECTIONS: usize = 128;
const AMBIENT_STEPS: usize = 16;

/// The air of a planet whose ground is a ball, the sea level at
/// `ground_radius` from `center`. Rayleigh scattering by the molecules
/// makes the blue, Mie scattering by dust and drops the haze and the glow
/// around the sun, ozone the deep blue overhead at dusk. Each thins out
/// with height on its own scale. Put it in `SpaceSky::atmosphere`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Atmosphere {
    pub center:              Vec3,
    pub ground_radius:       f32,
    pub top_radius:          f32,
    /// Per meter at sea level, for red, green and blue.
    pub rayleigh_scattering: Vec3,
    /// The height over which it falls to a third.
    pub rayleigh_height:     f32,
    pub mie_scattering:      Vec3,
    /// Scattering plus absorption.
    pub mie_extinction:      Vec3,
    pub mie_height:          f32,
    /// How much Mie light goes on forward, 0 for every way alike.
    pub mie_g:               f32,
    /// Per meter at the middle of the ozone layer.
    pub ozone_absorption:    Vec3,
    /// The height of the middle of the ozone layer and how far it reaches
    /// up and down from there.
    pub ozone_center:        f32,
    pub ozone_width:         f32,
    /// The share of light the ground throws back up.
    pub ground_albedo:       f32,
}

impl Atmosphere {
    /// The air of Earth around `center`, the values of Hillaire 2020 and
    /// of Bruneton's precomputed scattering.
    pub fn earth(center: impl Into<Vec3>) -> Self {
        Self {
            center:              center.into(),
            ground_radius:       6_360_000.0,
            top_radius:          6_460_000.0,
            rayleigh_scattering: Vec3::new(5.802e-6, 13.558e-6, 33.1e-6),
            rayleigh_height:     8000.0,
            mie_scattering:      Vec3::splat(3.996e-6),
            mie_extinction:      Vec3::splat(3.996e-6 + 4.40e-6),
            mie_height:          1200.0,
            mie_g:               0.8,
            ozone_absorption:    Vec3::new(0.650e-6, 1.881e-6, 0.085e-6),
            ozone_center:        25_000.0,
            ozone_width:         15_000.0,
            ground_albedo:       0.3,
        }
    }

    /// Earth's air on a planet of `ground_radius`, `thickness` deep.
    /// Every height shrinks by `thickness` over Earth's 100 km and every
    /// coefficient grows by as much, so light crosses as much air as on
    /// Earth and the sky has Earth's colors from the ground. A real 100
    /// km of air would be thicker than a small planet is wide.
    pub fn scaled(center: impl Into<Vec3>, ground_radius: f32, thickness: f32) -> Self {
        let earth = Self::earth(center);
        let k = thickness / (earth.top_radius - earth.ground_radius);
        Self {
            ground_radius,
            top_radius: ground_radius + thickness,
            rayleigh_scattering: earth.rayleigh_scattering / k,
            rayleigh_height: earth.rayleigh_height * k,
            mie_scattering: earth.mie_scattering / k,
            mie_extinction: earth.mie_extinction / k,
            mie_height: earth.mie_height * k,
            ozone_absorption: earth.ozone_absorption / k,
            ozone_center: earth.ozone_center * k,
            ozone_width: earth.ozone_width * k,
            ..earth
        }
    }

    /// The same air with its center at the origin, what the tables
    /// depend on.
    pub(crate) fn shape(&self) -> Self {
        Self {
            center: Vec3::ZERO,
            ..*self
        }
    }

    pub(crate) fn medium(&self, height: f32) -> Medium {
        let height = height.max(0.0);
        let rayleigh = self.rayleigh_scattering * (-height / self.rayleigh_height).exp();
        let mie_share = (-height / self.mie_height).exp();
        let ozone = (1.0 - (height - self.ozone_center).abs() / self.ozone_width).max(0.0);
        Medium {
            rayleigh,
            mie: self.mie_scattering * mie_share,
            extinction: rayleigh + self.mie_extinction * mie_share + self.ozone_absorption * ozone,
        }
    }

    /// How much light gets through the air from `r` from the center
    /// along a ray `mu` the cosine from straight up, to the top. A ray
    /// into the ground goes through it at sea level density, so almost
    /// nothing gets through.
    pub(crate) fn transmittance(&self, r: f32, mu: f32) -> Vec3 {
        let length = distance_to_top(r, mu, self.top_radius);
        let step = length / float(TABLE_STEPS);
        let mut depth = Vec3::ZERO;
        for index in 0..TABLE_STEPS {
            let t = (float(index) + 0.5) * step;
            let height = (r * r + t * t + 2.0 * r * mu * t).sqrt() - self.ground_radius;
            depth += self.medium(height).extinction * step;
        }
        (-depth).exp()
    }

    /// Where `r` and `mu` land in the transmittance table, from 0 to 1,
    /// the parameters of Bruneton 2017.
    pub(crate) fn transmittance_place(&self, r: f32, mu: f32) -> (f32, f32) {
        let horizon = self.horizon_reach();
        let rho = (r * r - self.ground_radius * self.ground_radius).max(0.0).sqrt();
        let d = distance_to_top(r, mu, self.top_radius);
        let d_min = self.top_radius - r;
        let d_max = rho + horizon;
        ((d - d_min) / (d_max - d_min).max(1e-6), rho / horizon)
    }

    /// The `r` and `mu` of a place in the transmittance table.
    fn transmittance_ray(&self, x_mu: f32, x_r: f32) -> (f32, f32) {
        let horizon = self.horizon_reach();
        let rho = horizon * x_r;
        let r = (rho * rho + self.ground_radius * self.ground_radius).sqrt();
        let d_min = self.top_radius - r;
        let d_max = rho + horizon;
        let d = d_min + x_mu * (d_max - d_min);
        let mu = if d <= 0.0 {
            1.0
        } else {
            (horizon * horizon - rho * rho - d * d) / (2.0 * r * d)
        };
        (r, mu.clamp(-1.0, 1.0))
    }

    /// From sea level to the top of the air along the ground.
    fn horizon_reach(&self) -> f32 {
        (self.top_radius * self.top_radius - self.ground_radius * self.ground_radius).sqrt()
    }

    pub(crate) fn tables(&self) -> AtmosphereTables {
        let (width, height) = TRANSMITTANCE_SIZE;
        let mut transmittance = Table::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let (r, mu) =
                    self.transmittance_ray(float(x) / float(width - 1), float(y) / float(height - 1));
                transmittance.texels[y * width + x] = self.transmittance(r, mu);
            }
        }
        let mut tables = AtmosphereTables {
            transmittance,
            multi: Table::new(MULTI_SIZE, MULTI_SIZE),
        };
        let thickness = self.top_radius - self.ground_radius;
        for y in 0..MULTI_SIZE {
            for x in 0..MULTI_SIZE {
                let mu_sun = float(x) / float(MULTI_SIZE - 1) * 2.0 - 1.0;
                // A hair over the ground, a ray from on it would start inside.
                let r = self.ground_radius + thickness * (float(y) / float(MULTI_SIZE - 1)).max(1e-3);
                tables.multi.texels[y * MULTI_SIZE + x] = self.multi_scattering(&tables, r, mu_sun);
            }
        }
        tables
    }

    /// The light scattered more than once that reaches a point `r` from
    /// the center with the sun at `mu_sun`, for a sun of 1: the light of 2
    /// scatterings gathered from every way, the series of the rest summed
    /// as `1 / (1 - f)`, Hillaire's equation 9.
    fn multi_scattering(&self, tables: &AtmosphereTables, r: f32, mu_sun: f32) -> Vec3 {
        let position = Vec3::new(0.0, r, 0.0);
        let to_sun = Vec3::new((1.0 - mu_sun * mu_sun).max(0.0).sqrt(), mu_sun, 0.0);
        let isotropic = 1.0 / (4.0 * PI);
        let (mut light, mut transfer) = (Vec3::ZERO, Vec3::ZERO);
        for dir in sphere_directions(MULTI_DIRECTIONS) {
            let mut through = Vec3::ONE;
            let Some((start, end, ground)) = self.span(position, dir, f32::INFINITY) else {
                continue;
            };
            let step = (end - start) / float(MULTI_STEPS);
            for index in 0..MULTI_STEPS {
                let point = position + dir * (start + (float(index) + 0.5) * step);
                let medium = self.medium(point.length() - self.ground_radius);
                let scattering = medium.rayleigh + medium.mie;
                let sun = tables.sun_through(self, point, to_sun);
                let step_through = (-medium.extinction * step).exp();
                light += through * integrate(scattering * sun * isotropic, medium.extinction, step_through);
                transfer += through * integrate(scattering, medium.extinction, step_through);
                through *= step_through;
            }
            if ground {
                let point = position + dir * end;
                let up = point.normalize();
                let sun = tables.sun_through(self, point, to_sun);
                light += through * sun * up.dot(to_sun).max(0.0) * self.ground_albedo / PI;
            }
        }
        let count = float(MULTI_DIRECTIONS);
        let (light, transfer) = (light / count, transfer / count);
        light / (Vec3::ONE - transfer).max(Vec3::splat(1e-3))
    }

    /// The part of the ray from `origin`, relative to the center, along
    /// `dir` that lies in the air, up to `limit`: where it enters, where
    /// it leaves, and whether it leaves into the ground. None for a ray
    /// that misses the air.
    pub(crate) fn span(&self, origin: Vec3, dir: Vec3, limit: f32) -> Option<(f32, f32, bool)> {
        let (_, top) = sphere_hits(origin, dir, self.top_radius)?;
        let start = sphere_hits(origin, dir, self.top_radius).map_or(0.0, |(near, _)| near.max(0.0));
        if top <= 0.0 {
            return None;
        }
        let ground = sphere_hits(origin, dir, self.ground_radius).filter(|(near, _)| *near > 0.0);
        let end = ground.map_or(top, |(near, _)| near.min(top));
        let end_in_limit = end.min(limit);
        (end_in_limit > start).then_some((start, end_in_limit, ground.is_some() && end <= limit))
    }

    /// The light the sky sends along `-dir` to `origin`, relative to the
    /// center, for a sun of 1 toward `to_sun`, and how much of what lies
    /// behind gets through, up to `limit`. Light in the units of a scene,
    /// where a white matte surface facing a light of 1 shows 1, so pi
    /// times radiance. With the ground lit where the ray ends on it.
    pub(crate) fn march(
        &self,
        tables: &AtmosphereTables,
        origin: Vec3,
        dir: Vec3,
        to_sun: Vec3,
        limit: f32,
        steps: usize,
    ) -> (Vec3, Vec3) {
        let Some((start, end, ground)) = self.span(origin, dir, limit) else {
            return (Vec3::ZERO, Vec3::ONE);
        };
        let cos = dir.dot(to_sun);
        let (rayleigh_phase, mie_phase) = (rayleigh_phase(cos), mie_phase(cos, self.mie_g));
        let step = (end - start) / float(steps);
        let (mut light, mut through) = (Vec3::ZERO, Vec3::ONE);
        for index in 0..steps {
            let point = origin + dir * (start + (float(index) + 0.5) * step);
            let r = point.length();
            let medium = self.medium(r - self.ground_radius);
            let sun = tables.sun_through(self, point, to_sun);
            let multi = tables.multi_at(self, r, point.dot(to_sun) / r);
            let source = sun * (medium.rayleigh * rayleigh_phase + medium.mie * mie_phase)
                + multi * (medium.rayleigh + medium.mie);
            let step_through = (-medium.extinction * step).exp();
            light += through * integrate(source, medium.extinction, step_through);
            through *= step_through;
        }
        light *= PI;
        if ground {
            let point = origin + dir * end;
            let up = point.normalize();
            let sun = tables.sun_through(self, point, to_sun);
            light += through * sun * up.dot(to_sun).max(0.0) * self.ground_albedo;
        }
        (light, through)
    }

    /// The 9 numbers of the light the whole sky throws on a surface at
    /// `camera`, relative to the center, for a sun of 1 toward `to_sun`,
    /// in the form `sky_irradiance` of the mesh shader reads, see
    /// `hilen_pixels::irradiance`.
    pub(crate) fn ambient(&self, tables: &AtmosphereTables, camera: Vec3, to_sun: Vec3) -> [Vec3; 9] {
        let mut coefficients = [Vec3::ZERO; 9];
        let solid_angle = 4.0 * PI / float(AMBIENT_DIRECTIONS);
        for dir in sphere_directions(AMBIENT_DIRECTIONS) {
            let (light, _) = self.march(tables, camera, dir, to_sun, f32::INFINITY, AMBIENT_STEPS);
            for (coefficient, basis) in coefficients.iter_mut().zip(hilen_pixels::sh_basis(dir)) {
                *coefficient += light * (basis * solid_angle);
            }
        }
        // The cosine lobe per band, already over pi for the Lambert term.
        let bands = [1.0, 2.0 / 3.0, 2.0 / 3.0, 2.0 / 3.0, 0.25, 0.25, 0.25, 0.25, 0.25];
        for (coefficient, band) in coefficients.iter_mut().zip(bands) {
            *coefficient *= band;
        }
        coefficients
    }
}

/// What a bit of air does to light, per meter.
pub(crate) struct Medium {
    pub rayleigh:   Vec3,
    pub mie:        Vec3,
    pub extinction: Vec3,
}

/// A table of colors, read with a blend of the 4 texels around a place.
#[derive(Clone, Debug)]
pub(crate) struct Table {
    pub width:  usize,
    pub height: usize,
    pub texels: Vec<Vec3>,
}

impl Table {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            texels: vec![Vec3::ZERO; width * height],
        }
    }

    /// The color at `x` and `y` from 0 to 1, the first and the last texel
    /// on the edges.
    pub(crate) fn sample(&self, x: f32, y: f32) -> Vec3 {
        let fx = x.clamp(0.0, 1.0) * float(self.width - 1);
        let fy = y.clamp(0.0, 1.0) * float(self.height - 1);
        let (x0, y0): (usize, usize) = (fx.floor().lossy_convert(), fy.floor().lossy_convert());
        let (x1, y1) = ((x0 + 1).min(self.width - 1), (y0 + 1).min(self.height - 1));
        let (tx, ty) = (fx - float(x0), fy - float(y0));
        let at = |x: usize, y: usize| self.texels[y * self.width + x];
        let top = at(x0, y0).lerp(at(x1, y0), tx);
        let bottom = at(x0, y1).lerp(at(x1, y1), tx);
        top.lerp(bottom, ty)
    }
}

/// The 2 tables of an air.
#[derive(Clone, Debug)]
pub(crate) struct AtmosphereTables {
    pub transmittance: Table,
    pub multi:         Table,
}

impl AtmosphereTables {
    /// How much of the sun reaches `point`, relative to the center: what
    /// the air lets through, none behind the ground.
    fn sun_through(&self, air: &Atmosphere, point: Vec3, to_sun: Vec3) -> Vec3 {
        let r = point.length();
        let mu = point.dot(to_sun) / r;
        if hits_ground(r, mu, air.ground_radius) {
            return Vec3::ZERO;
        }
        let (x, y) = air.transmittance_place(r, mu);
        self.transmittance.sample(x, y)
    }

    fn multi_at(&self, air: &Atmosphere, r: f32, mu_sun: f32) -> Vec3 {
        let height = (r - air.ground_radius) / (air.top_radius - air.ground_radius);
        self.multi.sample(mu_sun * 0.5 + 0.5, height)
    }
}

/// The length from `r` along `mu` to the sphere of `top`.
fn distance_to_top(r: f32, mu: f32, top: f32) -> f32 {
    let discriminant = r * r * (mu * mu - 1.0) + top * top;
    (-r * mu + discriminant.max(0.0).sqrt()).max(0.0)
}

fn hits_ground(r: f32, mu: f32, ground: f32) -> bool {
    mu < 0.0 && r * r * (mu * mu - 1.0) + ground * ground >= 0.0
}

/// Where the ray from `origin` along unit `dir` meets the sphere of
/// `radius` around the origin, near and far, None if it misses.
fn sphere_hits(origin: Vec3, dir: Vec3, radius: f32) -> Option<(f32, f32)> {
    let b = origin.dot(dir);
    let c = origin.length_squared() - radius * radius;
    let discriminant = b * b - c;
    (discriminant >= 0.0).then(|| {
        let root = discriminant.sqrt();
        (-b - root, -b + root)
    })
}

/// The light a step adds, the source times the share of the step it
/// survives, `(source - source * through) / extinction`.
fn integrate(source: Vec3, extinction: Vec3, through: Vec3) -> Vec3 {
    (source - source * through) / extinction.max(Vec3::splat(1e-12))
}

fn rayleigh_phase(cos: f32) -> f32 {
    3.0 / (16.0 * PI) * (1.0 + cos * cos)
}

/// The Cornette-Shanks phase of Mie scattering.
fn mie_phase(cos: f32, g: f32) -> f32 {
    let g2 = g * g;
    3.0 / (8.0 * PI) * (1.0 - g2) * (1.0 + cos * cos) / ((2.0 + g2) * (1.0 + g2 - 2.0 * g * cos).powf(1.5))
}

/// `count` directions spread evenly over the sphere, a Fibonacci spiral.
fn sphere_directions(count: usize) -> impl Iterator<Item = Vec3> {
    let golden = PI * (3.0 - 5.0_f32.sqrt());
    (0..count).map(move |index| {
        let y = 1.0 - 2.0 * (float(index) + 0.5) / float(count);
        let ring = (1.0 - y * y).sqrt();
        let angle = golden * float(index);
        Vec3::new(angle.cos() * ring, y, angle.sin() * ring)
    })
}

/// A 32 bit float as the 16 bits of a half float, rounded to the nearest,
/// what the tables go to the GPU as.
pub(crate) fn half_bits(value: f32) -> u16 {
    f16::from_f32(value).to_bits()
}

#[cfg(test)]
mod test {
    use super::*;

    fn earth() -> Atmosphere {
        Atmosphere::earth(Vec3::ZERO)
    }

    #[test]
    fn the_table_place_comes_back_to_its_ray() {
        let air = earth();
        for (x_mu, x_r) in [(0.0, 0.0), (0.3, 0.1), (0.7, 0.5), (1.0, 1.0), (0.5, 0.9)] {
            let (r, mu) = air.transmittance_ray(x_mu, x_r);
            let (back_mu, back_r) = air.transmittance_place(r, mu);
            assert!((back_mu - x_mu).abs() < 2e-3, "{x_mu} came back {back_mu}");
            assert!((back_r - x_r).abs() < 2e-3, "{x_r} came back {back_r}");
        }
    }

    /// Straight up from sea level Earth lets through about 90 % of red
    /// and 70 % of blue, the numbers of Bruneton's tables.
    #[test]
    fn earth_passes_more_red_than_blue_straight_up() {
        let air = earth();
        let up = air.transmittance(air.ground_radius, 1.0);
        assert!((0.85..0.95).contains(&up.x), "{up}");
        assert!((0.6..0.8).contains(&up.z), "{up}");
        let low = air.transmittance(air.ground_radius, 0.02);
        assert!(low.z < 0.05 && low.x > low.z * 5.0, "a low sun is red: {low}");
    }

    #[test]
    fn a_smaller_air_lets_through_the_same_light() {
        let earth = earth();
        let small = Atmosphere::scaled(Vec3::ZERO, 20_000.0, 5000.0);
        let a = earth.transmittance(earth.ground_radius, 1.0);
        let b = small.transmittance(small.ground_radius, 1.0);
        assert!((a - b).length() < 1e-3, "{a} against {b}");
    }

    /// At noon the sky overhead is blue, at sunset the sky toward the sun
    /// is redder than overhead, and from above the air nothing lights a
    /// ray into space.
    #[test]
    fn the_sky_is_blue_and_red_at_sunset() {
        let air = Atmosphere::scaled(Vec3::ZERO, 20_000.0, 5000.0);
        let tables = air.tables();
        let ground = Vec3::new(0.0, air.ground_radius + 2.0, 0.0);
        let (noon, _) = air.march(&tables, ground, Vec3::Y, Vec3::Y, f32::INFINITY, 32);
        assert!(noon.z > noon.x * 1.5, "noon overhead is blue: {noon}");
        let to_sun = Vec3::new(1.0, 0.03, 0.0).normalize();
        let (sunset, _) = air.march(
            &tables,
            ground,
            Vec3::new(1.0, 0.05, 0.0).normalize(),
            to_sun,
            f32::INFINITY,
            32,
        );
        assert!(sunset.x > sunset.z, "toward a setting sun it is red: {sunset}");
        let space = Vec3::new(0.0, air.top_radius + 100.0, 0.0);
        let (out, through) = air.march(&tables, space, Vec3::Y, Vec3::Y, f32::INFINITY, 32);
        assert_eq!((out, through), (Vec3::ZERO, Vec3::ONE));
    }

    /// From space the air glows around the planet's edge, a ray that
    /// grazes it gathers light.
    #[test]
    fn from_space_the_edge_glows() {
        let air = Atmosphere::scaled(Vec3::ZERO, 20_000.0, 5000.0);
        let tables = air.tables();
        let far = Vec3::new(0.0, 0.0, 200_000.0);
        let grazing = (Vec3::new(0.0, air.ground_radius + 1000.0, 0.0) - far).normalize();
        let (light, _) = air.march(&tables, far, grazing, Vec3::Y, f32::INFINITY, 32);
        assert!(light.z > 0.01, "{light}");
    }

    #[test]
    fn the_sky_lights_the_ground_by_day_and_not_by_night() {
        let air = Atmosphere::scaled(Vec3::ZERO, 20_000.0, 5000.0);
        let tables = air.tables();
        let ground = Vec3::new(0.0, air.ground_radius + 2.0, 0.0);
        let lit = |to_sun: Vec3| {
            let coefficients = air.ambient(&tables, ground, to_sun);
            coefficients
                .iter()
                .zip(hilen_pixels::sh_basis(Vec3::Y))
                .map(|(c, b)| *c * b)
                .sum::<Vec3>()
        };
        let day = lit(Vec3::Y);
        let night = lit(Vec3::NEG_Y);
        assert!(day.z > 0.05 && day.z > day.x, "a day sky lights blue: {day}");
        assert!(
            night.length() < day.length() * 0.01,
            "night {night} against day {day}"
        );
    }

    #[test]
    fn half_floats_keep_their_value() {
        assert_eq!(half_bits(0.0), 0);
        assert_eq!(half_bits(1.0), 0x3c00);
        assert_eq!(half_bits(-2.0), 0xc000);
        assert_eq!(half_bits(0.5), 0x3800);
        assert_eq!(half_bits(65504.0), 0x7bff);
        assert_eq!(half_bits(1e9), 0x7c00);
        assert_eq!(half_bits(6.0e-8), 1);
    }
}
