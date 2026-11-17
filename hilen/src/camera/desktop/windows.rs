//! Media Foundation through nokhwa. A camera is opened by its number in
//! the list, which is its id.

use anyhow::{Result, anyhow};
use log::{debug, warn};
use nokhwa::{
    Camera,
    pixel_format::RgbFormat,
    query,
    utils::{ApiBackend, CameraIndex, FrameFormat, RequestedFormat, RequestedFormatType},
};

use crate::camera::{
    CameraDevice, CameraLens, CameraLocks, CameraPosition,
    desktop::{Picture, Pixels, Webcam},
};

pub(super) fn list() -> Result<Vec<CameraDevice>> {
    let cameras = query(ApiBackend::MediaFoundation).map_err(|err| anyhow!("{err}"))?;
    Ok(cameras
        .into_iter()
        .map(|camera| CameraDevice {
            id:            camera.index().as_string(),
            name:          camera.human_name(),
            position:      CameraPosition::External,
            lens:          CameraLens::Unknown,
            near_focus_mm: None,
        })
        .collect())
}

pub(super) fn open(device: &CameraDevice) -> Result<Box<dyn Webcam>> {
    let index = device
        .id
        .parse()
        .map_or_else(|_| CameraIndex::String(device.id.clone()), CameraIndex::Index);
    let format = RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestResolution);
    let mut camera = Camera::with_backend(index, format, ApiBackend::MediaFoundation)?;
    camera.open_stream()?;
    debug!("camera {} streams {}", device.name, camera.camera_format());
    Ok(Box::new(WindowsWebcam {
        name: device.name.clone(),
        camera,
    }))
}

struct WindowsWebcam {
    name:   String,
    camera: Camera,
}

impl Webcam for WindowsWebcam {
    fn picture(&mut self) -> Result<Picture> {
        let buffer = self.camera.frame()?;
        let size = buffer.resolution();
        let pixels = match buffer.source_frame_format() {
            FrameFormat::MJPEG => Pixels::Jpeg,
            FrameFormat::YUYV => Pixels::Yuyv,
            FrameFormat::NV12 => Pixels::Nv12,
            FrameFormat::GRAY => Pixels::Gray,
            FrameFormat::RAWRGB => Pixels::Rgb,
            FrameFormat::RAWBGR => Pixels::Bgr,
        };
        Ok(Picture {
            pixels,
            bytes: buffer.buffer().to_vec(),
            width: size.width(),
            height: size.height(),
        })
    }

    /// nokhwa sets a value of Media Foundation but keeps it automatic, it
    /// cannot switch a setting to manual. So nothing locks on Windows yet.
    fn set_locks(&mut self, locks: CameraLocks) -> Result<CameraLocks> {
        if locks != CameraLocks::NONE {
            warn!("camera {} cannot lock {locks:?} on Windows yet", self.name);
        }
        Ok(CameraLocks::NONE)
    }
}

impl Drop for WindowsWebcam {
    fn drop(&mut self) {
        if let Err(err) = self.camera.stop_stream() {
            warn!("camera {} failed to stop: {err}", self.name);
        }
    }
}
