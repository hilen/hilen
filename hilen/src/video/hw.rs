//! The hardware decoder device. ffmpeg picks the codec, this hands it the
//! platform device and the pixel format callback that makes it decode on the
//! GPU, and copies a decoded frame back into system memory.

use std::ptr::{null, null_mut};

use ffmpeg_next::{
    Error, codec,
    ffi::{
        AVBufferRef, AVCodecContext, AVHWDeviceType, AVPixelFormat, av_hwdevice_ctx_create,
        av_hwframe_transfer_data,
    },
    format::Pixel,
    frame,
};

#[cfg(macos)]
const DEVICE: AVHWDeviceType = AVHWDeviceType::AV_HWDEVICE_TYPE_VIDEOTOOLBOX;
#[cfg(macos)]
const FORMAT: AVPixelFormat = AVPixelFormat::AV_PIX_FMT_VIDEOTOOLBOX;

#[cfg(linux)]
const DEVICE: AVHWDeviceType = AVHWDeviceType::AV_HWDEVICE_TYPE_VAAPI;
#[cfg(linux)]
const FORMAT: AVPixelFormat = AVPixelFormat::AV_PIX_FMT_VAAPI;

#[cfg(win)]
const DEVICE: AVHWDeviceType = AVHWDeviceType::AV_HWDEVICE_TYPE_D3D11VA;
#[cfg(win)]
const FORMAT: AVPixelFormat = AVPixelFormat::AV_PIX_FMT_D3D11;

/// The pixel format of a frame the device decoded.
pub(crate) fn pixel() -> Pixel {
    Pixel::from(FORMAT)
}

/// Attaches the device to the codec context before it opens. False when the
/// platform gives no device, the codec then decodes in software.
pub(crate) fn attach(context: &mut codec::context::Context) -> bool {
    let mut device: *mut AVBufferRef = null_mut();
    // SAFETY: an out pointer for the device, no device name and no options.
    let created = unsafe { av_hwdevice_ctx_create(&raw mut device, DEVICE, null(), null_mut(), 0) };
    if created < 0 || device.is_null() {
        return false;
    }
    // SAFETY: the context is open for writing and not yet opened by ffmpeg.
    // The device reference is owned by the codec context from here on and
    // freed with it.
    unsafe {
        let raw = context.as_mut_ptr();
        (*raw).hw_device_ctx = device;
        (*raw).get_format = Some(pick_format);
    }
    true
}

/// ffmpeg offers the formats the codec can decode into, hardware ones first
/// and the software one last. Take ours, or the software one when ours is
/// not on the list.
unsafe extern "C" fn pick_format(
    _context: *mut AVCodecContext,
    formats: *const AVPixelFormat,
) -> AVPixelFormat {
    let mut cursor = formats;
    let mut last = AVPixelFormat::AV_PIX_FMT_NONE;
    // SAFETY: ffmpeg ends the list with AV_PIX_FMT_NONE.
    unsafe {
        while *cursor != AVPixelFormat::AV_PIX_FMT_NONE {
            if *cursor == FORMAT {
                return FORMAT;
            }
            last = *cursor;
            cursor = cursor.add(1);
        }
    }
    last
}

/// Copies a frame the device decoded into system memory, NV12 for 8 bit
/// content. `to` must be empty, ffmpeg allocates it.
pub(crate) fn transfer(from: &frame::Video, to: &mut frame::Video) -> Result<(), Error> {
    // SAFETY: both frames are valid ffmpeg frames.
    let code = unsafe { av_hwframe_transfer_data(to.as_mut_ptr(), from.as_ptr(), 0) };
    if code < 0 {
        return Err(Error::from(code));
    }
    Ok(())
}
