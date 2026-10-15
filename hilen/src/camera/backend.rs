use anyhow::Result;

use crate::camera::{CameraDevice, CameraLocks, Photo, PhotoFormat, frame::FrameSlot};

/// How a backend answers. It may call it on any thread, once.
pub(super) type Done<T> = Box<dyn FnOnce(Result<T>) + Send>;

/// Where the cameras come from: the system, or a fake one in a test.
pub(super) trait Backend: Sync {
    /// Answers at once when the app has the camera already.
    fn request_access(&self, done: Done<()>);

    fn devices(&self, done: Done<Vec<CameraDevice>>);

    /// Puts every picture of the live stream into `frames`.
    fn open(&self, device: &CameraDevice, frames: FrameSlot, done: Done<Box<dyn Session>>);
}

/// An open camera. Dropping it closes the camera.
pub(super) trait Session: Send {
    fn take_photo(&mut self, format: PhotoFormat, done: Done<Photo>);

    /// Answers with the locks the camera holds after the call.
    fn set_locks(&mut self, locks: CameraLocks, done: Done<CameraLocks>);

    /// A backend that has to fetch a picture into its `FrameSlot`, rather
    /// than get one pushed by the system, fetches it here. A browser does.
    fn pull_frame(&mut self) {}
}
