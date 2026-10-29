//! A capture of the GPU work of a few frames into a `.gputrace` file. Xcode
//! opens it with the time of every pass and every draw, what Metal System
//! Trace cannot tell apart inside 1 pass. Metal only, and Metal refuses a
//! capture to a file unless the app started with `METAL_CAPTURE_ENABLED=1`
//! in the env.

use std::{path::PathBuf, sync::Mutex};

use anyhow::{Result, bail};

/// A capture asked for and not started yet, and the frames a running one
/// still takes.
struct Capture {
    asked:       Option<(u32, PathBuf)>,
    frames_left: u32,
}

static CAPTURE: Mutex<Capture> = Mutex::new(Capture {
    asked:       None,
    frames_left: 0,
});

pub struct GpuCapture;

impl GpuCapture {
    /// Captures the next `frames` frames into `path`, a `.gputrace` that
    /// must not exist yet. The capture starts with the next frame drawn, a
    /// failure to start is logged then.
    pub fn frames(frames: u32, path: impl Into<PathBuf>) -> Result<()> {
        if !cfg!(apple) {
            bail!("a GPU capture needs Metal, an Apple system");
        }
        let path = path.into();
        if path.extension().is_none_or(|extension| extension != "gputrace") {
            bail!("a GPU capture goes into a .gputrace file, not {}", path.display());
        }
        if path.exists() {
            bail!("{} exists already", path.display());
        }
        let mut capture = CAPTURE.lock().expect("the capture lock is not poisoned");
        if capture.frames_left > 0 || capture.asked.is_some() {
            bail!("a GPU capture is already running");
        }
        capture.asked = Some((frames.max(1), path));
        Ok(())
    }
}

/// Starts an asked capture right before a frame is drawn.
pub(crate) fn begin_frame() {
    let mut capture = CAPTURE.lock().expect("the capture lock is not poisoned");
    let Some((frames, path)) = capture.asked.take() else {
        return;
    };
    match start(&path) {
        Ok(()) => {
            log::info!("GPU capture of {frames} frames into {}", path.display());
            capture.frames_left = frames;
        }
        Err(err) => log::error!("GPU capture into {} did not start: {err:#}", path.display()),
    }
}

/// Stops a running capture after its last frame is presented.
pub(crate) fn end_frame() {
    let mut capture = CAPTURE.lock().expect("the capture lock is not poisoned");
    if capture.frames_left == 0 {
        return;
    }
    capture.frames_left -= 1;
    if capture.frames_left == 0 {
        stop();
        log::info!("GPU capture done");
    }
}

#[cfg(apple)]
fn start(path: &std::path::Path) -> Result<()> {
    use anyhow::Context;
    use objc2_foundation::{NSString, NSURL};
    use objc2_metal::{MTLCaptureDescriptor, MTLCaptureDestination, MTLCaptureManager};

    use crate::window::Window;

    let manager = unsafe { MTLCaptureManager::sharedCaptureManager() };
    if !manager.supportsDestination(MTLCaptureDestination::GPUTraceDocument) {
        bail!("Metal captures no file, start the app with METAL_CAPTURE_ENABLED=1");
    }
    let hal =
        unsafe { Window::device().as_hal::<wgpu_hal::api::Metal>() }.context("the device is not Metal")?;
    let descriptor = MTLCaptureDescriptor::new();
    let path = path.to_str().context("the path is not UTF-8")?;
    let url = NSURL::fileURLWithPath(&NSString::from_str(path));
    unsafe {
        descriptor.setCaptureObject(Some(hal.raw_device().as_ref()));
        descriptor.setDestination(MTLCaptureDestination::GPUTraceDocument);
        descriptor.setOutputURL(Some(&url));
    }
    manager
        .startCaptureWithDescriptor_error(&descriptor)
        .map_err(|err| anyhow::anyhow!("{}", err.localizedDescription()))
}

#[cfg(apple)]
fn stop() {
    let manager = unsafe { objc2_metal::MTLCaptureManager::sharedCaptureManager() };
    manager.stopCapture();
}

#[cfg(not(apple))]
fn start(_: &std::path::Path) -> Result<()> {
    bail!("a GPU capture needs Metal, an Apple system")
}

#[cfg(not(apple))]
fn stop() {}
