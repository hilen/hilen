use std::f32::consts::FRAC_PI_2;

use glam::camera::rh::{
    proj::directx::{orthographic, perspective, perspective_infinite_reverse},
    view::look_at_mat4,
};

use crate::gm::{
    flat::{Point, Size},
    volume::{Mat4, Ray, Vec3, Vec4},
};

/// Orbiting stops this close to straight up or down, where the up vector
/// and the view direction would line up and the view matrix degenerates.
const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.01;

/// The smallest world height an orthographic zoom goes down to.
const MIN_HEIGHT: f32 = 1e-3;

/// How the camera puts the world onto the screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Projection {
    /// Far things look smaller, the way an eye sees. `Camera::fov_y` is
    /// how wide it looks.
    Perspective,
    /// A thing has the same size on the screen at any distance and
    /// parallel lines stay parallel, the front, top and side views of a
    /// part. `height` is the world height the view shows. The camera sees
    /// along the line to its target, from its near plane on, so it stands
    /// outside what it shows.
    Orthographic { height: f32 },
}

/// Where the scene is seen from. A right handed world with y up, the
/// same handedness as glTF and Blender.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub position:   Vec3,
    pub target:     Vec3,
    pub up:         Vec3,
    /// Vertical field of view in radians, for a perspective camera.
    pub fov_y:      f32,
    pub near:       f32,
    pub far:        f32,
    pub projection: Projection,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position:   Vec3::new(0.0, 4.0, 10.0),
            target:     Vec3::ZERO,
            up:         Vec3::Y,
            fov_y:      60.0_f32.to_radians(),
            near:       0.1,
            far:        500.0,
            projection: Projection::Perspective,
        }
    }
}

impl Camera {
    /// A camera at `position` looking at `target` that shows `height`
    /// world units from the bottom of the view to its top, at any
    /// distance.
    pub fn orthographic(position: impl Into<Vec3>, target: impl Into<Vec3>, height: f32) -> Self {
        Self {
            position: position.into(),
            target: target.into(),
            projection: Projection::Orthographic { height },
            ..Self::default()
        }
    }

    pub fn view(&self) -> Mat4 {
        look_at_mat4(self.position, self.target, self.up)
    }

    /// The unit direction the camera looks in.
    pub fn forward(&self) -> Vec3 {
        (self.target - self.position).normalize_or(Vec3::NEG_Z)
    }

    /// Depth lands in 0 to 1 reversed, 1 on the near plane and 0 at `far`,
    /// where 32 bit floats are densest, so the far away keeps its depth
    /// steps about as fine for its size as the near. A perspective camera
    /// with a `far` of `f32::INFINITY` has no far plane, a planet and its
    /// moon 300 km off fit 1 view. An orthographic one needs a finite
    /// `far`.
    pub fn projection(&self, aspect: f32) -> Mat4 {
        match self.projection {
            Projection::Perspective if self.far.is_infinite() => {
                perspective_infinite_reverse(self.fov_y, aspect, self.near)
            }
            Projection::Perspective => reverse_depth(perspective(self.fov_y, aspect, self.near, self.far)),
            Projection::Orthographic { height } => {
                let half = height / 2.0;
                reverse_depth(orthographic(
                    -half * aspect,
                    half * aspect,
                    -half,
                    half,
                    self.near,
                    self.far,
                ))
            }
        }
    }

    /// The world height the view shows at `depth` along the line of
    /// sight, the same at any depth for an orthographic camera.
    pub fn view_height(&self, depth: f32) -> f32 {
        match self.projection {
            Projection::Perspective => 2.0 * depth * (self.fov_y / 2.0).tan(),
            Projection::Orthographic { height } => height,
        }
    }

    /// The world size of one pixel at `world` for a view drawn over
    /// `area`, what keeps a thing the same size on the screen.
    pub fn units_per_pixel(&self, world: Vec3, area: Size) -> f32 {
        let depth = (world - self.position).dot(self.forward()).max(self.near);
        self.view_height(depth) / area.height.max(1.0)
    }

    pub fn view_projection(&self, aspect: f32) -> Mat4 {
        self.projection(aspect) * self.view()
    }

    /// The ray through a pixel of the view drawn over `area`, from the
    /// camera into the world, what a touch on the scene becomes.
    /// A perspective ray starts at the camera, an orthographic one on the
    /// near plane at the pixel and goes along the line of sight.
    pub fn ray(&self, point: Point, area: Size) -> Ray {
        let x = point.x / area.width * 2.0 - 1.0;
        let y = 1.0 - point.y / area.height * 2.0;
        let inverse = self.view_projection(area.width / area.height).inverse();
        // The depth is reversed, the near plane is at 1.
        let on_near = inverse.project_point3(Vec3::new(x, y, 1.0));
        match self.projection {
            Projection::Perspective => Ray {
                origin:    self.position,
                direction: (on_near - self.position).normalize_or(Vec3::NEG_Z),
            },
            Projection::Orthographic { .. } => Ray {
                origin:    on_near,
                direction: self.forward(),
            },
        }
    }

    /// The pixel of the view drawn over `area` where a world point lands,
    /// the reverse of `ray`. None for a point level with the camera or
    /// behind it, which is nowhere on the screen. A point off to the side
    /// lands outside `area`.
    pub fn screen_point(&self, world: Vec3, area: Size) -> Option<Point> {
        let clip = self.view_projection(area.width / area.height) * world.extend(1.0);
        // The w of a perspective point is its depth, an orthographic one
        // keeps w at 1 and needs the depth asked for.
        let depth = match self.projection {
            Projection::Perspective => clip.w,
            Projection::Orthographic { .. } => (world - self.position).dot(self.forward()),
        };
        if depth <= self.near * 0.5 {
            return None;
        }
        let x = clip.x / clip.w;
        let y = clip.y / clip.w;
        Some(Point::new(
            f32::midpoint(x, 1.0) * area.width,
            f32::midpoint(1.0, -y) * area.height,
        ))
    }

    /// Move along the line of sight, `factor` scales the distance to the
    /// target. Stops at twice the near plane, any closer and the target
    /// clips. An orthographic camera stays where it is and `factor`
    /// scales the height it shows.
    pub fn zoom(&mut self, factor: f32) {
        if let Projection::Orthographic { height } = &mut self.projection {
            *height = (*height * factor).max(MIN_HEIGHT);
            return;
        }

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

/// `projection` with its depth turned round: z becomes w minus z, so the
/// near plane lands on 1 and the far one on 0.
fn reverse_depth(projection: Mat4) -> Mat4 {
    Mat4::from_cols(
        Vec4::X,
        Vec4::Y,
        Vec4::new(0.0, 0.0, -1.0, 0.0),
        Vec4::new(0.0, 0.0, 1.0, 1.0),
    ) * projection
}

#[cfg(test)]
mod test {
    use std::f32::consts::TAU;

    use super::*;

    /// The depth a point straight ahead at `distance` lands on.
    fn depth_at(camera: &Camera, distance: f32) -> f32 {
        let ahead = camera.position + camera.forward() * distance;
        camera.view_projection(1.0).project_point3(ahead).z
    }

    #[test]
    fn the_depth_is_reversed_from_the_near_plane_to_the_far_one() {
        let camera = Camera::default();
        assert!((depth_at(&camera, camera.near) - 1.0).abs() < 1e-5);
        assert!(depth_at(&camera, camera.far).abs() < 1e-5);
        let flat = Camera::orthographic(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO, 4.0);
        assert!((depth_at(&flat, flat.near) - 1.0).abs() < 1e-5);
        assert!(depth_at(&flat, flat.far).abs() < 1e-5);
    }

    /// With no far plane a moon 300 km off still lands in the depth range,
    /// and 2 points 1 m apart there get depths 32 bit floats tell apart.
    #[test]
    fn with_no_far_plane_the_far_away_keeps_fine_depth() {
        let camera = Camera {
            far: f32::INFINITY,
            ..Camera::default()
        };
        let moon = depth_at(&camera, 300_000.0);
        assert!(moon > 0.0 && moon < 1e-5, "{moon}");
        assert!(
            depth_at(&camera, 299_999.0) > moon,
            "1 m nearer is a larger depth"
        );
        assert!((depth_at(&camera, camera.near) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn a_ray_through_the_middle_goes_where_the_camera_looks() {
        let area = Size::new(800.0, 600.0);
        for camera in [
            Camera::default(),
            Camera {
                far: f32::INFINITY,
                ..Camera::default()
            },
        ] {
            let ray = camera.ray(Point::new(400.0, 300.0), area);
            assert_eq!(ray.origin, camera.position);
            assert!(
                (ray.direction - camera.forward()).length() < 1e-4,
                "{:?}",
                ray.direction
            );
        }
        let flat = Camera::orthographic(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO, 4.0);
        let ray = flat.ray(Point::new(400.0, 300.0), area);
        assert!(
            (ray.origin - Vec3::new(0.0, 0.0, 10.0 - flat.near)).length() < 1e-4,
            "{:?}",
            ray.origin
        );
        assert!((ray.direction - Vec3::NEG_Z).length() < 1e-6);
    }

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

    // A label placed at the pixel must sit on the node it names, so the
    // pixel's own ray has to pass through the world point again.
    #[test]
    fn a_screen_point_lies_on_its_own_ray() {
        let camera = Camera::default();
        let area = Size::new(800.0, 600.0);
        for world in [Vec3::ZERO, Vec3::new(3.0, 1.0, -2.0), Vec3::new(-4.0, 5.0, 3.0)] {
            let point = camera.screen_point(world, area).expect("in front of the camera");
            let ray = camera.ray(point, area);
            let along = (world - ray.origin).dot(ray.direction);
            let miss = (ray.origin + ray.direction * along - world).length();
            assert!(miss < 1e-3, "{world} misses its ray by {miss}");
        }
        let center = camera.screen_point(camera.target, area).expect("the target is ahead");
        assert!((center.x - 400.0).abs() < 1e-3 && (center.y - 300.0).abs() < 1e-3);
    }

    #[test]
    fn a_point_behind_the_camera_is_nowhere() {
        let camera = Camera::default();
        let area = Size::new(800.0, 600.0);
        let behind = camera.position * 2.0 - camera.target;
        assert_eq!(camera.screen_point(behind, area), None);
        assert_eq!(camera.screen_point(camera.position, area), None);
    }

    fn front_view() -> Camera {
        Camera::orthographic(Vec3::new(0.0, 1.0, 20.0), Vec3::new(0.0, 1.0, 0.0), 8.0)
    }

    // The point of an orthographic view: a thing far away is drawn as
    // big as the same thing near.
    #[test]
    fn orthographic_size_does_not_change_with_distance() {
        let camera = front_view();
        let area = Size::new(800.0, 600.0);
        let width = |z: f32| {
            let left = camera.screen_point(Vec3::new(-1.0, 1.0, z), area).expect("in front");
            let right = camera.screen_point(Vec3::new(1.0, 1.0, z), area).expect("in front");
            right.x - left.x
        };
        assert!((width(10.0) - width(-30.0)).abs() < 1e-3);
        // 8 units over 600 pixels, so 2 units are 150 pixels wide.
        assert!((width(0.0) - 150.0).abs() < 1e-2, "width {}", width(0.0));
        let perspective = Camera {
            projection: Projection::Perspective,
            ..camera
        };
        let near = perspective.screen_point(Vec3::new(1.0, 1.0, 10.0), area).expect("in front");
        let far = perspective.screen_point(Vec3::new(1.0, 1.0, -30.0), area).expect("in front");
        assert!(near.x - 400.0 > (far.x - 400.0) * 2.0);
    }

    // Every orthographic ray is parallel to the line of sight and starts
    // on the near plane at its own pixel, and the pixel of a point lies
    // on its ray again.
    #[test]
    fn orthographic_rays_are_parallel_and_hit_their_points() {
        let camera = front_view();
        let area = Size::new(800.0, 600.0);
        let forward = camera.forward();
        for pixel in [
            Point::new(0.0, 0.0),
            Point::new(400.0, 300.0),
            Point::new(712.0, 81.0),
        ] {
            let ray = camera.ray(pixel, area);
            assert!(ray.direction.dot(forward) > 0.9999, "{ray:?}");
            assert!(((ray.origin - camera.position).dot(forward) - camera.near).abs() < 1e-3);
        }
        for world in [Vec3::ZERO, Vec3::new(3.0, 1.0, -2.0), Vec3::new(-2.5, 3.0, 4.0)] {
            let point = camera.screen_point(world, area).expect("in front of the camera");
            let ray = camera.ray(point, area);
            let along = (world - ray.origin).dot(ray.direction);
            let miss = (ray.origin + ray.direction * along - world).length();
            assert!(miss < 1e-3, "{world} misses its ray by {miss}");
        }
        let behind = camera.position + Vec3::Z;
        assert_eq!(camera.screen_point(behind, area), None);
    }

    #[test]
    fn orthographic_zoom_scales_the_height_and_keeps_the_camera() {
        let mut camera = front_view();
        camera.zoom(0.5);
        assert_eq!(camera.projection, Projection::Orthographic { height: 4.0 });
        assert_eq!(camera.position, front_view().position);
        camera.zoom(0.0);
        assert_eq!(camera.projection, Projection::Orthographic { height: MIN_HEIGHT });
        let area = Size::new(800.0, 600.0);
        let near = front_view().units_per_pixel(Vec3::new(0.0, 0.0, 15.0), area);
        let far = front_view().units_per_pixel(Vec3::new(0.0, 0.0, -50.0), area);
        assert!((near - far).abs() < 1e-6 && (near - 8.0 / 600.0).abs() < 1e-6);
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
