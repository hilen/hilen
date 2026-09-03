use std::f32::consts::FRAC_PI_2;

use glam::camera::rh::{proj::directx::perspective, view::look_at_mat4};

use crate::gm::{
    flat::{Point, Size},
    volume::{Mat4, Ray, Vec3},
};

/// Orbiting stops this close to straight up or down, where the up vector
/// and the view direction would line up and the view matrix degenerates.
const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.01;

/// Where the scene is seen from. A right handed world with y up, the
/// same handedness as glTF and Blender.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub position: Vec3,
    pub target:   Vec3,
    pub up:       Vec3,
    /// Vertical field of view in radians.
    pub fov_y:    f32,
    pub near:     f32,
    pub far:      f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 4.0, 10.0),
            target:   Vec3::ZERO,
            up:       Vec3::Y,
            fov_y:    60.0_f32.to_radians(),
            near:     0.1,
            far:      500.0,
        }
    }
}

impl Camera {
    pub fn view(&self) -> Mat4 {
        look_at_mat4(self.position, self.target, self.up)
    }

    /// Depth lands in 0 to 1, what wgpu expects.
    pub fn projection(&self, aspect: f32) -> Mat4 {
        perspective(self.fov_y, aspect, self.near, self.far)
    }

    pub fn view_projection(&self, aspect: f32) -> Mat4 {
        self.projection(aspect) * self.view()
    }

    /// The ray through a pixel of the view drawn over `area`, from the
    /// camera into the world, what a touch on the scene becomes.
    pub fn ray(&self, point: Point, area: Size) -> Ray {
        let x = point.x / area.width * 2.0 - 1.0;
        let y = 1.0 - point.y / area.height * 2.0;
        let far = self
            .view_projection(area.width / area.height)
            .inverse()
            .project_point3(Vec3::new(x, y, 1.0));
        Ray {
            origin:    self.position,
            direction: (far - self.position).normalize_or(Vec3::NEG_Z),
        }
    }

    /// Move along the line of sight, `factor` scales the distance to the
    /// target. Stops at twice the near plane, any closer and the target
    /// clips.
    pub fn zoom(&mut self, factor: f32) {
        let offset = self.position - self.target;
        let radius = offset.length();

        if radius == 0.0 {
            return;
        }

        let distance = (radius * factor).max(self.near * 2.0);
        self.position = self.target + offset / radius * distance;
    }

    /// Turn around the target on the sphere the camera sits on, `yaw`
    /// and `pitch` in radians.
    pub fn orbit(&mut self, yaw: f32, pitch: f32) {
        let offset = self.position - self.target;
        let radius = offset.length();

        if radius == 0.0 {
            return;
        }

        let yaw = offset.x.atan2(offset.z) + yaw;
        let pitch = ((offset.y / radius).asin() + pitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);

        self.position =
            self.target + Vec3::new(pitch.cos() * yaw.sin(), pitch.sin(), pitch.cos() * yaw.cos()) * radius;
    }
}

#[cfg(test)]
mod test {
    use std::f32::consts::TAU;

    use super::*;

    #[test]
    fn orbit_keeps_the_radius_and_a_full_turn_comes_back() {
        let start = Camera::default();
        let mut camera = start;
        let radius = (start.position - start.target).length();

        for _ in 0..8 {
            camera.orbit(TAU / 8.0, 0.0);
            assert!(((camera.position - camera.target).length() - radius).abs() < 1e-4);
        }

        assert!((camera.position - start.position).length() < 1e-3);
    }

    #[test]
    fn zoom_scales_the_distance_and_stops_at_the_near_plane() {
        let mut camera = Camera::default();
        let radius = (camera.position - camera.target).length();
        camera.zoom(0.5);
        assert!(((camera.position - camera.target).length() - radius / 2.0).abs() < 1e-4);
        camera.zoom(0.0);
        assert!(((camera.position - camera.target).length() - camera.near * 2.0).abs() < 1e-5);
        camera.zoom(4.0);
        assert!(((camera.position - camera.target).length() - camera.near * 8.0).abs() < 1e-4);
    }

    #[test]
    fn the_center_pixel_looks_at_the_target_and_a_corner_looks_away() {
        let camera = Camera::default();
        let area = Size::new(800.0, 600.0);
        let center = camera.ray(Point::new(400.0, 300.0), area);
        let to_target = (camera.target - camera.position).normalize();
        assert!(center.direction.dot(to_target) > 0.9999);
        assert_eq!(center.origin, camera.position);
        // The top left corner ray points up and left of the center one.
        let corner = camera.ray(Point::new(0.0, 0.0), area);
        assert!(corner.direction.y > center.direction.y);
        assert!(corner.direction.x < center.direction.x);
    }

    #[test]
    fn orbit_never_reaches_the_pole() {
        let mut camera = Camera::default();
        camera.orbit(0.0, 10.0);
        let offset = camera.position - camera.target;
        assert!(offset.y < offset.length());
        assert!(offset.x.abs() + offset.z.abs() > 0.0);
    }
}
