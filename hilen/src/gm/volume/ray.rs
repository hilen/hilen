use crate::gm::volume::{Quat, Vec3};

/// A half line from `origin` along the unit `direction`, what a touch
/// becomes in the world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray {
    pub origin:    Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub fn at(&self, distance: f32) -> Vec3 {
        self.origin + self.direction * distance
    }

    /// The distance to the ball, zero from inside it.
    pub fn hit_ball(&self, center: Vec3, radius: f32) -> Option<f32> {
        let to_center = center - self.origin;
        let along = to_center.dot(self.direction);
        let closest = to_center.length_squared() - along * along;
        let radius = radius * radius;
        if closest > radius {
            return None;
        }
        let half = (radius - closest).sqrt();
        let near = along - half;
        if near >= 0.0 {
            Some(near)
        } else if along + half >= 0.0 {
            Some(0.0)
        } else {
            None
        }
    }

    /// The distance to a box of `half` extents turned by `rotation`
    /// around `center`, zero from inside it. The slab test in the box's
    /// own space.
    pub fn hit_box(&self, center: Vec3, rotation: Quat, half: Vec3) -> Option<f32> {
        let inverse = rotation.inverse();
        let origin = inverse * (self.origin - center);
        let direction = inverse * self.direction;

        let mut near = 0.0_f32;
        let mut far = f32::INFINITY;

        for axis in 0..3 {
            if direction[axis].abs() < 1e-8 {
                if origin[axis].abs() > half[axis] {
                    return None;
                }
                continue;
            }
            let enter = (-half[axis] - origin[axis]) / direction[axis];
            let exit = (half[axis] - origin[axis]) / direction[axis];
            near = near.max(enter.min(exit));
            far = far.min(enter.max(exit));
            if near > far {
                return None;
            }
        }

        Some(near)
    }
}

#[cfg(test)]
mod test {
    use std::f32::consts::FRAC_PI_4;

    use super::*;

    fn down_from(x: f32, z: f32) -> Ray {
        Ray {
            origin:    Vec3::new(x, 10.0, z),
            direction: Vec3::NEG_Y,
        }
    }

    #[test]
    fn ball_hit_lands_on_its_surface() {
        let hit = down_from(0.0, 0.0).hit_ball(Vec3::ZERO, 2.0).unwrap();
        assert!((hit - 8.0).abs() < 1e-5);
        assert!(down_from(2.5, 0.0).hit_ball(Vec3::ZERO, 2.0).is_none());
    }

    #[test]
    fn a_ball_behind_the_origin_is_missed_and_one_around_it_is_zero() {
        let up = Ray {
            origin:    Vec3::ZERO,
            direction: Vec3::Y,
        };
        assert!(up.hit_ball(Vec3::new(0.0, -5.0, 0.0), 1.0).is_none());
        assert_eq!(up.hit_ball(Vec3::ZERO, 1.0), Some(0.0));
    }

    #[test]
    fn box_hit_lands_on_its_top() {
        let half = Vec3::new(1.0, 0.5, 1.0);
        let hit = down_from(0.5, -0.5).hit_box(Vec3::ZERO, Quat::IDENTITY, half).unwrap();
        assert!((hit - 9.5).abs() < 1e-5);
        assert!(down_from(1.5, 0.0).hit_box(Vec3::ZERO, Quat::IDENTITY, half).is_none());
    }

    #[test]
    fn a_turned_box_is_hit_by_its_corner() {
        // Turned 45 degrees a unit cube reaches sqrt 2 along x, and its
        // top corner stands where the unturned box has nothing.
        let turned = Quat::from_rotation_y(FRAC_PI_4);
        let half = Vec3::splat(1.0);
        assert!(down_from(1.3, 0.0).hit_box(Vec3::ZERO, turned, half).is_some());
        assert!(down_from(1.3, 0.0).hit_box(Vec3::ZERO, Quat::IDENTITY, half).is_none());
        assert!(down_from(0.9, 0.9).hit_box(Vec3::ZERO, turned, half).is_none());
    }

    #[test]
    fn a_sideways_ray_misses_the_box_it_does_not_reach() {
        let ray = Ray {
            origin:    Vec3::new(-5.0, 3.0, 0.0),
            direction: Vec3::X,
        };
        assert!(ray.hit_box(Vec3::ZERO, Quat::IDENTITY, Vec3::ONE).is_none());
        let lower = Ray {
            origin:    Vec3::new(-5.0, 0.5, 0.0),
            direction: Vec3::X,
        };
        assert!((lower.hit_box(Vec3::ZERO, Quat::IDENTITY, Vec3::ONE).unwrap() - 4.0).abs() < 1e-5);
    }
}
