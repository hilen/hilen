use crate::gm::volume::Vec3;

/// A mass that pulls everything in a scene toward its center, a planet or
/// a moon, see `SceneBase::set_wells`. `gm` is the gravitational constant
/// times the mass, so the pull at `distance` from the center is `gm /
/// distance²`. Inside `radius` the pull falls in a straight line to 0 at
/// the center, the way it does inside a solid ball, so a body that digs
/// to the core is not flung away.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GravityWell {
    pub center: Vec3,
    pub gm:     f32,
    pub radius: f32,
}

impl GravityWell {
    /// A well whose pull at its surface `radius` is `surface_gravity`, a
    /// planet of 20 km with 9.81 on its ground.
    pub fn with_surface_gravity(center: impl Into<Vec3>, radius: f32, surface_gravity: f32) -> Self {
        Self {
            center: center.into(),
            gm: surface_gravity * radius * radius,
            radius,
        }
    }

    /// The pull of this well at `point`, toward its center.
    pub fn pull(&self, point: Vec3) -> Vec3 {
        let toward = self.center - point;
        let distance = toward.length();
        if distance <= f32::EPSILON {
            return Vec3::ZERO;
        }
        let strength = if distance < self.radius {
            self.gm * distance / self.radius.powi(3)
        } else {
            self.gm / (distance * distance)
        };
        toward * (strength / distance)
    }
}

/// The pull at `point` of `uniform` gravity plus every well.
pub(crate) fn gravity_at(uniform: Vec3, wells: &[GravityWell], point: Vec3) -> Vec3 {
    wells.iter().fold(uniform, |pull, well| pull + well.pull(point))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn a_planet_pulls_its_surface_gravity_on_its_ground() {
        let planet = GravityWell::with_surface_gravity(Vec3::new(0.0, -20_000.0, 0.0), 20_000.0, 9.81);
        let pull = planet.pull(Vec3::ZERO);
        assert!((pull - Vec3::new(0.0, -9.81, 0.0)).length() < 1e-3, "{pull}");
        let high = planet.pull(Vec3::new(0.0, 20_000.0, 0.0));
        assert!(
            (high.length() - 9.81 / 4.0).abs() < 1e-3,
            "twice as far, a quarter: {high}"
        );
    }

    #[test]
    fn inside_the_pull_falls_to_nothing_at_the_center() {
        let planet = GravityWell::with_surface_gravity(Vec3::ZERO, 100.0, 10.0);
        assert!((planet.pull(Vec3::new(50.0, 0.0, 0.0)) - Vec3::new(-5.0, 0.0, 0.0)).length() < 1e-4);
        assert_eq!(planet.pull(Vec3::ZERO), Vec3::ZERO);
    }

    #[test]
    fn the_pulls_add_up() {
        let a = GravityWell::with_surface_gravity(Vec3::new(-10.0, 0.0, 0.0), 1.0, 1.0);
        let b = GravityWell::with_surface_gravity(Vec3::new(10.0, 0.0, 0.0), 1.0, 1.0);
        let between = gravity_at(Vec3::new(0.0, -1.0, 0.0), &[a, b], Vec3::ZERO);
        assert!((between - Vec3::new(0.0, -1.0, 0.0)).length() < 1e-6, "{between}");
    }
}
