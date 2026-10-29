//! Video for Linux through linuxvideo. Every camera is a `/dev/video`
//! device that captures video, its path is its id.

use std::io;

use anyhow::{Result, anyhow};
use linuxvideo::{
    BufType, CapabilityFlags, Device,
    controls::Cid,
    format::{PixFormat, PixelFormat},
    stream::ReadStream,
};
use log::debug;

use crate::camera::{
    CameraDevice, CameraLens, CameraLocks, CameraPosition,
    desktop::{Picture, Pixels, Webcam},
};

/// The pixel formats taken, the first one the camera has wins. A webcam
/// sends its largest pictures as JPEG.
const FORMATS: [(PixelFormat, Pixels); 5] = [
    (PixelFormat::MJPG, Pixels::Jpeg),
    (PixelFormat::JPEG, Pixels::Jpeg),
    (PixelFormat::YUYV, Pixels::Yuyv),
    (PixelFormat::RGB3, Pixels::Rgb),
    (PixelFormat::BGR3, Pixels::Bgr),
];

/// The values of the exposure menu of a UVC webcam, `V4L2_EXPOSURE_MANUAL`
/// and `V4L2_EXPOSURE_APERTURE_PRIORITY`.
const EXPOSURE_MANUAL: i32 = 1;
const EXPOSURE_AUTO: i32 = 3;

pub(super) fn list() -> Result<Vec<CameraDevice>> {
    let mut devices = Vec::new();
    for device in linuxvideo::list()? {
        let described = device.and_then(|device| {
            let capabilities = device.capabilities()?;
            let captures = capabilities.device_capabilities().contains(CapabilityFlags::VIDEO_CAPTURE);
            Ok(captures.then(|| CameraDevice {
                id:            device.path().map_or_default(|path| path.display().to_string()),
                name:          capabilities.card().to_string(),
                position:      CameraPosition::External,
                lens:          CameraLens::Unknown,
                near_focus_mm: None,
            }))
        });
        match described {
            Ok(Some(camera)) if !camera.id.is_empty() => devices.push(camera),
            Ok(_) => {}
            Err(err) => debug!("a video device is skipped: {err}"),
        }
    }
    devices.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(devices)
}

pub(super) fn open(device: &CameraDevice) -> Result<Box<dyn Webcam>> {
    let handle = Device::open(&device.id)?;
    let formats = handle
        .formats(BufType::VIDEO_CAPTURE)
        .map(|format| format.map(|format| format.pixel_format()))
        .collect::<io::Result<Vec<_>>>()?;
    let (format, pixels) =
        FORMATS
            .into_iter()
            .find(|(format, _)| formats.contains(format))
            .ok_or_else(|| {
                anyhow!(
                    "camera {} sends none of the formats the engine reads: {formats:?}",
                    device.name
                )
            })?;
    // The driver answers the largest size it has for the format.
    let capture = handle.video_capture(PixFormat::new(u32::MAX, u32::MAX, format))?;
    let size = capture.format();
    let (width, height) = (size.width(), size.height());
    debug!("camera {} streams {format:?} at {width} by {height}", device.name);
    Ok(Box::new(LinuxWebcam {
        path: device.id.clone(),
        stream: capture.into_stream()?,
        pixels,
        width,
        height,
    }))
}

struct LinuxWebcam {
    path:   String,
    stream: ReadStream,
    pixels: Pixels,
    width:  u32,
    height: u32,
}

impl Webcam for LinuxWebcam {
    fn picture(&mut self) -> Result<Picture> {
        let bytes = self.stream.dequeue(|buffer| {
            if buffer.is_error() {
                return Err(io::Error::other("the camera marked the picture as broken"));
            }
            Ok(buffer.to_vec())
        })?;
        Ok(Picture {
            pixels: self.pixels,
            bytes,
            width: self.width,
            height: self.height,
        })
    }

    /// A webcam has a switch for each automatic setting. The stream owns
    /// its handle, so the switches go through a second one, which V4L2
    /// allows next to the stream.
    fn set_locks(&mut self, locks: CameraLocks) -> Result<CameraLocks> {
        let mut controls = Device::open(&self.path)?;
        let mut set = |control: Cid, value: i32| match controls.write_control_raw(control, value) {
            Ok(()) => true,
            Err(err) => {
                debug!("camera {} has no control {control:?}: {err}", self.path);
                false
            }
        };
        let exposure = if locks.exposure {
            EXPOSURE_MANUAL
        } else {
            EXPOSURE_AUTO
        };
        Ok(CameraLocks {
            focus:         set(Cid::FOCUS_AUTO, i32::from(!locks.focus)) && locks.focus,
            exposure:      set(Cid::EXPOSURE_AUTO, exposure) && locks.exposure,
            white_balance: set(Cid::AUTO_WHITE_BALANCE, i32::from(!locks.white_balance))
                && locks.white_balance,
        })
    }
}
