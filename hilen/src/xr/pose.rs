use crate::{
    gm::{
        flat::Point,
        volume::{Quat, Ray, Vec3},
    },
    scene::Camera,
};

/// A place and a turn in the world. A turn of none looks down -z with y
/// up, the way `OpenXR` and the scene camera both do.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub position: Vec3,
    pub rotation: Quat,
}

impl Pose {
    /// The unit direction the pose looks or points in.
    pub fn forward(&self) -> Vec3 {
        self.rotation * Vec3::NEG_Z
    }
}

/// A hand of the player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hand {
    Left,
    Right,
}

impl Hand {
    #[cfg(windows)]
    pub(crate) const BOTH: [Hand; 2] = [Hand::Left, Hand::Right];

    pub(crate) fn index(self) -> usize {
        match self {
            Hand::Left => 0,
            Hand::Right => 1,
        }
    }
}

/// A controller in a hand, read once a frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HandState {
    /// Where the hand holds the controller, for a thing held in the hand.
    pub grip:    Pose,
    /// Where the controller points from, for a ray.
    pub aim:     Pose,
    /// From 0 when let go to 1 when pulled all the way.
    pub trigger: f32,
    /// From 0 when let go to 1 when squeezed all the way.
    pub squeeze: f32,
    /// The thumbstick, x to the right and y up, each from -1 to 1.
    pub stick:   Point,
    /// The lower of the 2 buttons under the thumb, A on an Index controller.
    pub a:       bool,
    /// The upper of the 2 buttons under the thumb, B on an Index controller.
    pub b:       bool,
}

impl HandState {
    /// The ray the controller points along, the one to pick a node with,
    /// see `SceneBase::hit_by`.
    pub fn ray(&self) -> Ray {
        Ray {
            origin:    self.aim.position,
            direction: self.aim.forward(),
        }
    }
}

/// Where the space of the headset sits in the world: its origin on the
/// scene camera and its -z along the heading of the camera, level with
/// the ground. A scene moves the player in VR by moving its camera, and a
/// scene made for the screen shows in the headset with no change.
#[cfg(any(windows, test))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Rig {
    origin: Vec3,
    turn:   Quat,
}

#[cfg(any(windows, test))]
impl Rig {
    pub(crate) fn of(camera: &Camera) -> Self {
        let forward = camera.forward();
        // A camera that looks straight down or up has no heading, it
        // keeps facing -z.
        let yaw = if forward.x.abs() + forward.z.abs() < 1e-6 {
            0.0
        } else {
            (-forward.x).atan2(-forward.z)
        };
        Self {
            origin: camera.position,
            turn:   Quat::from_rotation_y(yaw),
        }
    }

    /// A pose in the space of the headset as a pose in the world.
    pub(crate) fn world(&self, pose: Pose) -> Pose {
        Pose {
            position: self.origin + self.turn * pose.position,
            rotation: self.turn * pose.rotation,
        }
    }
}

#[cfg(test)]
mod test {
    use std::f32::consts::FRAC_PI_2;

    use super::*;

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-5
    }

    #[test]
    fn the_rig_stands_on_the_camera_and_faces_its_heading() {
        let camera = Camera {
            position: Vec3::new(5.0, 1.7, 2.0),
            target: Vec3::new(9.0, 0.0, 2.0),
            ..Camera::default()
        };
        let rig = Rig::of(&camera);

        let still = rig.world(Pose {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        });
        assert!(close(still.position, camera.position));
        assert!(close(still.forward(), Vec3::X), "{}", still.forward());

        // A step forward in the headset is a step along the heading of the
        // camera, and the height stays where the headset put it.
        let step = rig.world(Pose {
            position: Vec3::new(0.0, 0.2, -1.0),
            rotation: Quat::IDENTITY,
        });
        assert!(
            close(step.position, Vec3::new(6.0, 1.9, 2.0)),
            "{}",
            step.position
        );
    }

    #[test]
    fn a_head_turned_in_the_headset_turns_in_the_world() {
        let rig = Rig::of(&Camera {
            position: Vec3::ZERO,
            target: Vec3::new(0.0, 0.0, 3.0),
            ..Camera::default()
        });
        let left = rig.world(Pose {
            position: Vec3::ZERO,
            rotation: Quat::from_rotation_y(FRAC_PI_2),
        });
        // The camera faces +z, a quarter turn left from there faces +x.
        assert!(close(left.forward(), Vec3::X), "{}", left.forward());
    }

    #[test]
    fn a_camera_straight_down_keeps_facing_minus_z() {
        let rig = Rig::of(&Camera {
            position: Vec3::new(0.0, 10.0, 0.0),
            target: Vec3::ZERO,
            up: Vec3::NEG_Z,
            ..Camera::default()
        });
        let pose = rig.world(Pose {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        });
        assert!(close(pose.forward(), Vec3::NEG_Z));
    }

    #[test]
    fn the_ray_of_a_hand_starts_at_the_aim_and_points_along_it() {
        let aim = Pose {
            position: Vec3::new(1.0, 1.0, 1.0),
            rotation: Quat::from_rotation_x(-FRAC_PI_2),
        };
        let hand = HandState {
            grip: aim,
            aim,
            trigger: 0.0,
            squeeze: 0.0,
            stick: Point::default(),
            a: false,
            b: false,
        };
        let ray = hand.ray();
        assert_eq!(ray.origin, aim.position);
        assert!(close(ray.direction, Vec3::NEG_Y), "{}", ray.direction);
    }
}
