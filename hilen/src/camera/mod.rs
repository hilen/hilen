//! The cameras of the device: the list, a live picture in a `CameraView`,
//! and a photo at full resolution that code takes with no tap of the user.
//!
//! ```ignore
//! use hilen::{camera::{Camera, CameraLocks, PhotoFormat}, dispatch::{on_main, spawn}};
//!
//! spawn(async move {
//!     let devices = Camera::devices().await?;
//!     let camera = Camera::open(&devices[0]).await?;
//!     let preview = camera.clone();
//!     on_main(move || view.preview.set_camera(Some(preview)));
//!     camera.set_locks(CameraLocks::ALL).await?;
//!     let photo = camera.take_photo(PhotoFormat::Jpeg).await?;
//!     anyhow::Ok(())
//! });
//! ```
//!
//! Every call works from any thread. The first call that needs the camera
//! asks the user for it. On iOS and on a packaged Mac app the system shows
//! the `[camera] usage` text of `hilen.toml`, which the build writes into
//! the app.

mod backend;
#[cfg(test)]
mod camera_test;
mod camera_view;
mod error;
#[cfg(any(test, feature = "ui-tests"))]
mod fake;
mod frame;
mod photo;

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod apple;
#[cfg(any(target_os = "macos", target_os = "ios"))]
use apple as system;

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "android")]
use android as system;

#[cfg(any(target_os = "windows", target_os = "linux"))]
mod desktop;
#[cfg(any(target_os = "windows", target_os = "linux"))]
use desktop as system;

#[cfg(wasm)]
mod web;
#[cfg(wasm)]
use web as system;

#[cfg(not(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "android",
    target_os = "windows",
    target_os = "linux",
    wasm
)))]
mod none;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use futures_channel::oneshot;
use log::{debug, error, info};
#[cfg(not(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "android",
    target_os = "windows",
    target_os = "linux",
    wasm
)))]
use none as system;
use parking_lot::Mutex;

#[cfg(any(test, feature = "ui-tests"))]
pub use self::fake::FakeCamera;
use self::{
    backend::{Backend, Done, Session},
    frame::FrameSlot,
};
pub use self::{
    camera_view::CameraView,
    error::CameraError,
    frame::CameraFrame,
    photo::{Photo, PhotoFormat},
};

/// A camera of the device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraDevice {
    /// The same for the same camera while the app runs.
    pub id:       String,
    pub name:     String,
    pub position: CameraPosition,
    pub lens:     CameraLens,
}

/// Where a camera looks from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraPosition {
    /// The side of a phone away from the user.
    Back,
    /// The side of the screen.
    Front,
    /// A camera on a cable or on the network, like a webcam.
    External,
    Unknown,
}

/// The lens of a camera. A phone with several lenses lists each one as its
/// own camera, so the app picks the lens by picking the camera.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraLens {
    /// The main lens of a phone, and the lens of most webcams.
    Wide,
    UltraWide,
    Telephoto,
    Unknown,
}

/// Which automatic settings of a camera stand still. A series of photos of
/// one object looks the same when all 3 are locked. `true` keeps the value
/// the camera has now, `false` gives the setting back to the camera.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CameraLocks {
    pub focus:         bool,
    pub exposure:      bool,
    pub white_balance: bool,
}

impl CameraLocks {
    pub const ALL: Self = Self {
        focus:         true,
        exposure:      true,
        white_balance: true,
    };
    pub const NONE: Self = Self {
        focus:         false,
        exposure:      false,
        white_balance: false,
    };
}

/// An open camera. Clones share it, the camera closes when the last clone
/// is dropped.
#[derive(Clone)]
pub struct Camera {
    inner: Arc<Inner>,
}

struct Inner {
    device:  CameraDevice,
    frames:  FrameSlot,
    session: Mutex<Box<dyn Session>>,
}

impl Drop for Inner {
    fn drop(&mut self) {
        info!("camera {} closed", self.device.name);
    }
}

impl Camera {
    /// Asks the user for the camera when the app does not have it yet. A
    /// refusal is `CameraError::Refused`. `devices` and `open` ask by
    /// themselves, call this to ask at a moment the app picks.
    pub async fn request_access() -> Result<()> {
        let (done, answer) = reply();
        backend().request_access(done);
        answer.await.inspect_err(|err| error!("no access to the camera: {err:#}"))
    }

    /// The cameras of the device, after the user allowed the camera.
    pub async fn devices() -> Result<Vec<CameraDevice>> {
        Self::request_access().await?;
        let (done, answer) = reply();
        backend().devices(done);
        let devices = answer.await.inspect_err(|err| error!("cameras failed to list: {err:#}"))?;
        let names: Vec<_> = devices.iter().map(|device| device.name.as_str()).collect();
        info!("cameras: {names:?}");
        Ok(devices)
    }

    /// Opens `device` and starts its live picture.
    pub async fn open(device: &CameraDevice) -> Result<Self> {
        Self::request_access().await?;
        let frames = FrameSlot::default();
        let (done, answer) = reply();
        backend().open(device, frames.clone(), done);
        let session = answer
            .await
            .inspect_err(|err| error!("camera {} failed to open: {err:#}", device.name))?;
        info!(
            "camera {} opened, id {}, {:?} {:?}",
            device.name, device.id, device.position, device.lens
        );
        Ok(Self {
            inner: Arc::new(Inner {
                device: device.clone(),
                frames,
                session: Mutex::new(session),
            }),
        })
    }

    pub fn device(&self) -> &CameraDevice {
        &self.inner.device
    }

    /// Takes a photo at the full resolution of the camera. `Heic` falls back
    /// to `Jpeg` where the camera has no HEIC, `Photo::format` says which.
    pub async fn take_photo(&self, format: PhotoFormat) -> Result<Photo> {
        let (done, answer) = reply();
        self.inner.session.lock().take_photo(format, done);
        let photo = answer.await.inspect_err(|err| {
            error!(
                "camera {} failed to take a photo: {err:#}",
                self.inner.device.name
            );
        })?;
        info!(
            "camera {} took a {:?} photo, {} by {}, {} bytes",
            self.inner.device.name,
            photo.format,
            photo.width,
            photo.height,
            photo.bytes.len()
        );
        Ok(photo)
    }

    /// Locks or frees focus, exposure and white balance. Gives the locks
    /// the camera holds now, a camera with no such setting, like a webcam
    /// with a fixed focus, leaves that one `false`.
    pub async fn set_locks(&self, locks: CameraLocks) -> Result<CameraLocks> {
        let (done, answer) = reply();
        self.inner.session.lock().set_locks(locks, done);
        let held = answer
            .await
            .inspect_err(|err| error!("camera {} failed to lock: {err:#}", self.inner.device.name))?;
        info!(
            "camera {} locks asked {locks:?}, held {held:?}",
            self.inner.device.name
        );
        Ok(held)
    }

    /// The newest picture of the live stream, at most `CameraFrame::MAX_SIDE`
    /// on its longest side. None until the first one came.
    pub fn frame(&self) -> Option<CameraFrame> {
        self.inner.session.lock().pull_frame();
        self.inner.frames.latest()
    }
}

fn backend() -> &'static dyn Backend {
    #[cfg(any(test, feature = "ui-tests"))]
    if fake::is_installed() {
        return &fake::Fake;
    }
    &system::System
}

/// The callback a backend answers through, and the future that waits for
/// it.
fn reply<T: Send + 'static>() -> (Done<T>, impl Future<Output = Result<T>>) {
    let (sender, receiver) = oneshot::channel();
    let done: Done<T> = Box::new(move |result| {
        if sender.send(result).is_err() {
            debug!("a camera answer came after the caller stopped waiting");
        }
    });
    let answer = async move {
        receiver
            .await
            .map_err(|canceled| anyhow!("the camera dropped the request: {canceled}"))?
    };
    (done, answer)
}
