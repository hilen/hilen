use std::sync::mpsc::channel;

use anyhow::{Result, ensure};
use hilen::{
    AppRunner,
    dispatch::{from_main, spawn, wait_for_next_frame},
    gm::{LossyConvert, volume::Vec3},
    refs::Weak,
    scene::{Camera, SceneDepth, SceneManager, SceneSetup, SceneTest, scene},
};

use super::{TURN, build_rig, check_phone_photo, phone_at, photos, scene_camera};

/// How far a distance inside a flat face may be from the one worked out
/// by hand. The 4 samples of a pixel sit off its center, on a face
/// that slopes away they see slightly other distances.
const SHARE: f32 = 1e-3;

/// The rig of `Photo of scene`, and a depth photo from the phone above
/// it. The cube face, the floor and the sky each have a distance that is
/// worked out here from the camera alone. A plain photo after it still
/// has its known colors, and the frame on screen does not change.
#[scene]
#[derive(Default)]
struct DepthPhotoOfScene {}

impl SceneSetup for DepthPhotoOfScene {
    fn setup(&mut self) {
        build_rig(self);
    }
}

impl SceneTest for DepthPhotoOfScene {
    fn perform_test(scene: Weak<Self>) -> Result<()> {
        wait_for_next_frame();
        let before = AppRunner::take_screenshot()?;

        let phone = phone_at(0.0);
        let depth = depth_photo(phone, TURN)?;
        ensure!(
            depth.size.width == TURN.0 && depth.size.height == TURN.1 && depth.meters.len() == 1008 * 756,
            "the depth photo is {:?} with {} distances",
            depth.size,
            depth.meters.len()
        );

        // The middle hits the front face of the cube, z 0.6, the bottom
        // the floor, y 0, and the top has only the background.
        let probes = [
            (TURN.0 / 2, TURN.1 / 2, Some(Plane::Z(0.6))),
            (TURN.0 / 2, fraction(0.78, TURN.1), Some(Plane::Y(0.0))),
            (fraction(0.4, TURN.0), fraction(0.78, TURN.1), Some(Plane::Y(0.0))),
            (TURN.0 / 2, fraction(0.1, TURN.1), None),
        ];
        let mut wrong = vec![];
        for (x, y, plane) in probes {
            let seen = depth.at(x, y).unwrap_or(f32::NAN);
            let want = plane.map_or(f32::INFINITY, |plane| plane.distance(&phone, x, y));
            let close = if want.is_finite() {
                (seen - want).abs() <= want * SHARE
            } else {
                seen.is_infinite()
            };
            if !close {
                wrong.push(format!("{x} {y} is {seen} m, want {want} m"));
            }
        }
        ensure!(wrong.is_empty(), "the depth photo differs:\n{}", wrong.join("\n"));

        let light = photos(vec![phone], TURN)?.remove(0);
        check_phone_photo(&light, TURN)?;

        wait_for_next_frame();
        let after = AppRunner::take_screenshot()?;
        ensure!(
            before.data == after.data,
            "the depth photo changed the frame on screen"
        );
        ensure!(
            from_main(move || scene.camera) == scene_camera(),
            "the depth photo moved the camera of the scene"
        );
        Ok(())
    }
}

fn depth_photo(camera: Camera, size: (u32, u32)) -> Result<SceneDepth> {
    let (sender, receiver) = channel();
    from_main(move || {
        let depth = SceneManager::depth_photo(camera, size);
        spawn(async move {
            sender.send(depth.await).expect("the test waits for the depth photo");
        });
    });
    receiver.recv()?
}

/// A flat face of the rig, at its height or its depth.
#[derive(Clone, Copy)]
enum Plane {
    Y(f32),
    Z(f32),
}

impl Plane {
    /// The distance along the line of sight of `camera` to this plane
    /// through the middle of pixel `x` `y` of a photo of `TURN`.
    fn distance(self, camera: &Camera, x: u32, y: u32) -> f32 {
        let forward = (camera.target - camera.position).normalize();
        let right = forward.cross(camera.up).normalize();
        let up = right.cross(forward);
        let (width, height): (f32, f32) = (TURN.0.lossy_convert(), TURN.1.lossy_convert());
        let (x, y): (f32, f32) = (x.lossy_convert(), y.lossy_convert());
        let across = ((x + 0.5) / width * 2.0 - 1.0) * width / height;
        let down = 1.0 - (y + 0.5) / height * 2.0;
        let half = (camera.fov_y / 2.0).tan();
        // 1 step along the ray is 1 m along the line of sight.
        let ray: Vec3 = forward + right * across * half + up * down * half;
        match self {
            Self::Y(at) => (at - camera.position.y) / ray.y,
            Self::Z(at) => (at - camera.position.z) / ray.z,
        }
    }
}

fn fraction(part: f32, of: u32) -> u32 {
    let of: f32 = of.lossy_convert();
    (part * of).lossy_convert()
}
