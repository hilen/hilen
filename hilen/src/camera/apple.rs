//! `AVFoundation` on macOS and iOS. 1 capture session with 2 outputs: the
//! video data output gives the live picture as BGRA on a queue of its own,
//! the photo output takes the photo at full resolution. Every call on the
//! session runs on 1 serial queue, the way Apple asks for it.

use std::{
    path::Path,
    ptr::from_ref,
    slice,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread::{sleep, spawn},
    time::{Duration, Instant},
};

use anyhow::{Error, Result, anyhow};
use block2::RcBlock;
use dispatch2::{DispatchQueue, DispatchQueueAttr, DispatchRetained};
use log::{debug, warn};
use objc2::{
    AllocAnyThread, DefinedClass, define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, Bool, NSObject, NSObjectProtocol, ProtocolObject},
    sel,
};
use objc2_av_foundation::{
    AVAuthorizationStatus, AVCaptureConnection, AVCaptureDevice, AVCaptureDeviceDiscoverySession,
    AVCaptureDeviceInput, AVCaptureDevicePosition, AVCaptureExposureMode, AVCaptureFocusMode,
    AVCaptureOutput, AVCapturePhoto, AVCapturePhotoCaptureDelegate, AVCapturePhotoOutput,
    AVCapturePhotoSettings, AVCaptureSession, AVCaptureSessionPresetPhoto, AVCaptureVideoDataOutput,
    AVCaptureVideoDataOutputSampleBufferDelegate, AVCaptureWhiteBalanceMode, AVMediaType, AVMediaTypeVideo,
    AVVideoCodecKey, AVVideoCodecTypeHEVC, AVVideoCodecTypeJPEG,
};
use objc2_core_foundation::CGPoint;
use objc2_core_media::{CMSampleBuffer, CMVideoDimensions};
use objc2_core_video::{
    CVPixelBufferGetBaseAddress, CVPixelBufferGetBytesPerRow, CVPixelBufferGetHeight, CVPixelBufferGetWidth,
    CVPixelBufferLockBaseAddress, CVPixelBufferLockFlags, CVPixelBufferUnlockBaseAddress,
    kCVPixelBufferPixelFormatTypeKey, kCVPixelFormatType_32BGRA, kCVReturnSuccess,
};
use objc2_foundation::{
    NSArray, NSBundle, NSDictionary, NSError, NSNumber, NSOperatingSystemVersion, NSProcessInfo, NSString,
    NSValue,
};
use parking_lot::Mutex;

#[cfg(target_os = "ios")]
use crate::camera::frame::turn;
use crate::{
    camera::{
        CameraDevice, CameraError, CameraLens, CameraLocks, CameraPosition, Photo, PhotoFormat,
        backend::{Backend, Done, Session},
        frame::{FrameSlot, sensor_point},
    },
    gm::flat::Point,
};

const USAGE_KEY: &str = "NSCameraUsageDescription";
/// The quarter turns of the live picture, see `bgra_frame`.
const QUARTERS: u32 = if cfg!(target_os = "ios") { 1 } else { 0 };
/// A focus starts some frames after the call, and ends within this.
const FOCUS_START: Duration = Duration::from_millis(150);
const FOCUS_LONGEST: Duration = Duration::from_secs(2);
const FOCUS_POLL: Duration = Duration::from_millis(30);

/// The delegates of this build that are alive. The session drops them on its
/// queue some time after the camera closed, and a hot build deletes their
/// classes at its stop, so the stop waits until this is 0.
static DELEGATES: AtomicUsize = AtomicUsize::new(0);

/// Whether a delegate of a camera of this build is still alive.
#[cfg(target_os = "ios")]
pub(crate) fn delegates_alive() -> bool {
    DELEGATES.load(Ordering::Acquire) > 0
}

pub(super) struct System;

impl Backend for System {
    fn request_access(&self, done: Done<()>) {
        if needs_usage_text() && !has_usage_text() {
            done(Err(CameraError::NoUsageText.into()));
            return;
        }
        let media = match video() {
            Ok(media) => media,
            Err(err) => return done(Err(err)),
        };
        match unsafe { AVCaptureDevice::authorizationStatusForMediaType(media) } {
            AVAuthorizationStatus::Authorized => done(Ok(())),
            AVAuthorizationStatus::NotDetermined => {
                debug!("asking the user for the camera");
                // The block is `Fn`, the answer comes once.
                let done = Mutex::new(Some(done));
                let block = RcBlock::new(move |granted: Bool| {
                    if let Some(done) = done.lock().take() {
                        done(if granted.as_bool() {
                            Ok(())
                        } else {
                            Err(CameraError::Refused.into())
                        });
                    }
                });
                unsafe { AVCaptureDevice::requestAccessForMediaType_completionHandler(media, &block) };
            }
            _ => done(Err(CameraError::Refused.into())),
        }
    }

    fn devices(&self, done: Done<Vec<CameraDevice>>) {
        done(devices());
    }

    fn open(&self, device: &CameraDevice, frames: FrameSlot, done: Done<Box<dyn Session>>) {
        let device = device.clone();
        let queue = DispatchQueue::new("hilen.camera", DispatchQueueAttr::SERIAL);
        let session_queue = queue.clone();
        // `startRunning` blocks until the camera runs, so it goes off the
        // thread of the caller.
        queue.exec_async(move || {
            done(Parts::open(&device, frames).map(|parts| {
                Box::new(AppleSession {
                    parts: Arc::new(parts),
                    queue: session_queue,
                }) as Box<dyn Session>
            }));
        });
    }
}

/// iOS stops an app that asks with no usage text, and so does macOS for a
/// packaged app. A program started from a terminal asks in the name of
/// the terminal.
fn needs_usage_text() -> bool {
    cfg!(target_os = "ios")
        || Path::new(&NSBundle::mainBundle().bundlePath().to_string())
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("app"))
}

fn has_usage_text() -> bool {
    NSBundle::mainBundle()
        .objectForInfoDictionaryKey(&NSString::from_str(USAGE_KEY))
        .is_some()
}

fn video() -> Result<&'static AVMediaType> {
    unsafe { AVMediaTypeVideo }.ok_or_else(|| anyhow!("AVFoundation has no video media type"))
}

fn ns_error(error: &NSError) -> Error {
    anyhow!("{}", error.localizedDescription())
}

fn at_least(major: isize) -> bool {
    NSProcessInfo::processInfo().isOperatingSystemAtLeastVersion(NSOperatingSystemVersion {
        majorVersion: major,
        minorVersion: 0,
        patchVersion: 0,
    })
}

/// The kinds of camera to list. They are the values of the constants of
/// `AVFoundation`, not the constants: a constant of a newer system than the
/// oldest one the app runs on stops the app at its start there.
fn device_types() -> Vec<&'static str> {
    let mut types = vec!["AVCaptureDeviceTypeBuiltInWideAngleCamera"];
    if cfg!(target_os = "ios") {
        types.push("AVCaptureDeviceTypeBuiltInTelephotoCamera");
        if at_least(13) {
            types.push("AVCaptureDeviceTypeBuiltInUltraWideCamera");
        }
        if at_least(17) {
            types.push("AVCaptureDeviceTypeExternal");
        }
    } else if at_least(14) {
        types.push("AVCaptureDeviceTypeExternal");
        types.push("AVCaptureDeviceTypeContinuityCamera");
    } else {
        types.push("AVCaptureDeviceTypeExternalUnknown");
    }
    types
}

fn devices() -> Result<Vec<CameraDevice>> {
    let types: Vec<Retained<NSString>> = device_types().into_iter().map(NSString::from_str).collect();
    let types = NSArray::from_retained_slice(&types);
    let discovery = unsafe {
        AVCaptureDeviceDiscoverySession::discoverySessionWithDeviceTypes_mediaType_position(
            &types,
            Some(video()?),
            AVCaptureDevicePosition::Unspecified,
        )
    };
    Ok(unsafe { discovery.devices() }.iter().map(|device| describe(&device)).collect())
}

fn describe(device: &AVCaptureDevice) -> CameraDevice {
    let kind = unsafe { device.deviceType() }.to_string();
    let external = kind.contains("External") || kind.contains("Continuity");
    let position = match unsafe { device.position() } {
        AVCaptureDevicePosition::Back => CameraPosition::Back,
        AVCaptureDevicePosition::Front => CameraPosition::Front,
        _ if external => CameraPosition::External,
        _ => CameraPosition::Unknown,
    };
    let lens = if kind.ends_with("UltraWideCamera") {
        CameraLens::UltraWide
    } else if kind.ends_with("TelephotoCamera") {
        CameraLens::Telephoto
    } else if kind.ends_with("WideAngleCamera") {
        CameraLens::Wide
    } else {
        CameraLens::Unknown
    };
    // iOS 15 and macOS 12 tell it, -1 when they do not know.
    let near_focus_mm = device
        .respondsToSelector(sel!(minimumFocusDistance))
        .then(|| unsafe { device.minimumFocusDistance() })
        .and_then(|mm| u32::try_from(mm).ok())
        .filter(|mm| *mm > 0);
    CameraDevice {
        id: unsafe { device.uniqueID() }.to_string(),
        name: unsafe { device.localizedName() }.to_string(),
        position,
        lens,
        near_focus_mm,
    }
}

/// The objects of 1 open camera.
struct Parts {
    device:  Retained<AVCaptureDevice>,
    session: Retained<AVCaptureSession>,
    photos:  Retained<AVCapturePhotoOutput>,
    /// Kept for the session, which holds its delegate weakly.
    frames:  Retained<FrameDelegate>,
    /// The photo output holds a delegate only while it works on its photo.
    pending: Mutex<Vec<Retained<PhotoDelegate>>>,
}

// SAFETY: AVFoundation lets any thread call a capture session, a device and
// the outputs. The engine calls them only on the serial queue of the
// session, so no 2 calls overlap.
unsafe impl Send for Parts {}
unsafe impl Sync for Parts {}

impl Parts {
    fn open(device: &CameraDevice, frames: FrameSlot) -> Result<Self> {
        let camera = unsafe { AVCaptureDevice::deviceWithUniqueID(&NSString::from_str(&device.id)) }
            .ok_or_else(|| CameraError::NotFound(device.name.clone()))?;
        let input = unsafe { AVCaptureDeviceInput::deviceInputWithDevice_error(&camera) }
            .map_err(|err| ns_error(&err))?;

        let session = unsafe { AVCaptureSession::new() };
        unsafe { session.beginConfiguration() };
        let preset = unsafe { AVCaptureSessionPresetPhoto };
        if unsafe { session.canSetSessionPreset(preset) } {
            unsafe { session.setSessionPreset(preset) };
        }
        if !unsafe { session.canAddInput(&input) } {
            return Err(anyhow!("the camera {} gives no input to a session", device.name));
        }
        unsafe { session.addInput(&input) };

        let video = unsafe { AVCaptureVideoDataOutput::new() };
        // SAFETY: a CFString is the same object as an NSString.
        let format_key: &NSString =
            unsafe { &*from_ref(kCVPixelBufferPixelFormatTypeKey).cast::<NSString>() };
        let bgra = NSNumber::new_u32(kCVPixelFormatType_32BGRA);
        let settings: Retained<NSDictionary<NSString, AnyObject>> =
            NSDictionary::from_slices(&[format_key], &[bgra.as_ref()]);
        unsafe { video.setVideoSettings(Some(&settings)) };
        unsafe { video.setAlwaysDiscardsLateVideoFrames(true) };
        let delegate = FrameDelegate::new(frames);
        let frame_queue = DispatchQueue::new("hilen.camera.frames", DispatchQueueAttr::SERIAL);
        unsafe {
            video.setSampleBufferDelegate_queue(
                Some(ProtocolObject::from_ref(&*delegate)),
                Some(&frame_queue),
            );
        };
        if !unsafe { session.canAddOutput(&video) } {
            return Err(anyhow!("the camera {} gives no live picture", device.name));
        }
        unsafe { session.addOutput(&video) };

        let photos = unsafe { AVCapturePhotoOutput::new() };
        if !unsafe { session.canAddOutput(&photos) } {
            return Err(anyhow!("the camera {} takes no photos", device.name));
        }
        unsafe { session.addOutput(&photos) };
        if let Some(largest) = largest_photo(&camera, &photos) {
            debug!(
                "camera {} takes photos of {} by {}",
                device.name, largest.width, largest.height
            );
            unsafe { photos.setMaxPhotoDimensions(largest) };
        }
        unsafe { session.commitConfiguration() };
        unsafe { session.startRunning() };
        #[cfg(target_os = "ios")]
        debug!(
            "camera {} straightens its pictures: supported {}, on {}",
            device.name,
            unsafe { camera.isGeometricDistortionCorrectionSupported() },
            unsafe { camera.isGeometricDistortionCorrectionEnabled() }
        );

        Ok(Self {
            device: camera,
            session,
            photos,
            frames: delegate,
            pending: Mutex::new(Vec::new()),
        })
    }

    fn take_photo(&self, format: PhotoFormat, done: Done<Photo>) {
        let settings = photo_settings(&self.photos, format);
        if settings.respondsToSelector(sel!(setMaxPhotoDimensions:)) {
            unsafe { settings.setMaxPhotoDimensions(self.photos.maxPhotoDimensions()) };
        }
        let delegate = PhotoDelegate::new(done);
        let mut pending = self.pending.lock();
        pending.retain(|delegate| !delegate.finished());
        pending.push(delegate.clone());
        drop(pending);
        unsafe {
            self.photos
                .capturePhotoWithSettings_delegate(&settings, ProtocolObject::from_ref(&*delegate));
        }
    }

    fn set_locks(&self, locks: CameraLocks) -> Result<CameraLocks> {
        let device = &self.device;
        unsafe { device.lockForConfiguration() }.map_err(|err| ns_error(&err))?;
        let mut held = CameraLocks::NONE;

        let focus = if locks.focus {
            AVCaptureFocusMode::Locked
        } else {
            AVCaptureFocusMode::ContinuousAutoFocus
        };
        if unsafe { device.isFocusModeSupported(focus) } {
            unsafe { device.setFocusMode(focus) };
            held.focus = locks.focus;
        }

        let exposure = if locks.exposure {
            AVCaptureExposureMode::Locked
        } else {
            AVCaptureExposureMode::ContinuousAutoExposure
        };
        if unsafe { device.isExposureModeSupported(exposure) } {
            unsafe { device.setExposureMode(exposure) };
            held.exposure = locks.exposure;
        }

        let white_balance = if locks.white_balance {
            AVCaptureWhiteBalanceMode::Locked
        } else {
            AVCaptureWhiteBalanceMode::ContinuousAutoWhiteBalance
        };
        if unsafe { device.isWhiteBalanceModeSupported(white_balance) } {
            unsafe { device.setWhiteBalanceMode(white_balance) };
            held.white_balance = locks.white_balance;
        }

        unsafe { device.unlockForConfiguration() };
        Ok(held)
    }

    /// Starts 1 focus at `point` of the sensor and meters the exposure
    /// there. False when the camera has neither point.
    fn focus_at(&self, point: Point) -> Result<bool> {
        let device = &self.device;
        let focus = unsafe { device.isFocusPointOfInterestSupported() }
            && unsafe { device.isFocusModeSupported(AVCaptureFocusMode::AutoFocus) };
        let exposure = unsafe { device.isExposurePointOfInterestSupported() };
        if !focus && !exposure {
            return Ok(false);
        }
        let point = CGPoint::new(f64::from(point.x), f64::from(point.y));
        unsafe { device.lockForConfiguration() }.map_err(|err| ns_error(&err))?;
        // A point does nothing until its mode is set again.
        if focus {
            unsafe { device.setFocusPointOfInterest(point) };
            unsafe { device.setFocusMode(AVCaptureFocusMode::AutoFocus) };
        }
        if exposure {
            unsafe { device.setExposurePointOfInterest(point) };
            // A locked exposure meters once at the point and locks again.
            let mode = match unsafe { device.exposureMode() } {
                AVCaptureExposureMode::Locked => AVCaptureExposureMode::AutoExpose,
                mode => mode,
            };
            if unsafe { device.isExposureModeSupported(mode) } {
                unsafe { device.setExposureMode(mode) };
            }
        }
        unsafe { device.unlockForConfiguration() };
        Ok(focus)
    }

    fn adjusting_focus(&self) -> bool {
        unsafe { self.device.isAdjustingFocus() }
    }
}

impl Drop for Parts {
    fn drop(&mut self) {
        unsafe { self.session.stopRunning() };
        debug!(
            "capture session of {} stopped after {} frames",
            unsafe { self.device.localizedName() },
            self.frames.ivars().slot.count()
        );
    }
}

/// The largest photo the active format of the camera takes. Before macOS 13
/// and iOS 16 a photo has the size of the format, there is nothing to set.
fn largest_photo(camera: &AVCaptureDevice, photos: &AVCapturePhotoOutput) -> Option<CMVideoDimensions> {
    if !photos.respondsToSelector(sel!(setMaxPhotoDimensions:)) {
        return None;
    }
    let sizes: Retained<NSArray<NSValue>> = unsafe { camera.activeFormat().supportedMaxPhotoDimensions() };
    sizes
        .iter()
        .map(|size| unsafe { size.get::<CMVideoDimensions>() })
        .max_by_key(|size| i64::from(size.width) * i64::from(size.height))
}

/// HEVC where the output has it, JPEG otherwise.
fn photo_settings(photos: &AVCapturePhotoOutput, format: PhotoFormat) -> Retained<AVCapturePhotoSettings> {
    let jpeg = unsafe { AVVideoCodecTypeJPEG };
    let hevc = unsafe { AVVideoCodecTypeHEVC };
    let available = unsafe { photos.availablePhotoCodecTypes() };
    let wanted = match format {
        PhotoFormat::Heic => hevc.filter(|hevc| available.containsObject(hevc)).or(jpeg),
        PhotoFormat::Jpeg => jpeg,
    };
    match (unsafe { AVVideoCodecKey }, wanted) {
        (Some(key), Some(codec)) => {
            let codec: &AnyObject = codec;
            let format: Retained<NSDictionary<NSString, AnyObject>> =
                NSDictionary::from_slices(&[key], &[codec]);
            unsafe { AVCapturePhotoSettings::photoSettingsWithFormat(Some(&format)) }
        }
        _ => unsafe { AVCapturePhotoSettings::photoSettings() },
    }
}

struct AppleSession {
    parts: Arc<Parts>,
    queue: DispatchRetained<DispatchQueue>,
}

impl Session for AppleSession {
    fn take_photo(&mut self, format: PhotoFormat, done: Done<Photo>) {
        let parts = self.parts.clone();
        self.queue.exec_async(move || parts.take_photo(format, done));
    }

    fn set_locks(&mut self, locks: CameraLocks, done: Done<CameraLocks>) {
        let parts = self.parts.clone();
        self.queue.exec_async(move || done(parts.set_locks(locks)));
    }

    fn focus_at(&mut self, point: Point, done: Done<bool>) {
        let parts = self.parts.clone();
        let queue = self.queue.clone();
        self.queue.exec_async(move || {
            match parts.focus_at(sensor_point(point, QUARTERS)) {
                // The wait asks the queue, it must not hold it.
                Ok(true) => {
                    spawn(move || {
                        wait_for_focus(&parts, &queue);
                        done(Ok(true));
                    });
                }
                answer => done(answer),
            }
        });
    }
}

/// Waits until the camera ends the focus it started, at most
/// `FOCUS_LONGEST`.
fn wait_for_focus(parts: &Arc<Parts>, queue: &DispatchQueue) {
    let started = Instant::now();
    loop {
        sleep(FOCUS_POLL);
        let adjusting = Arc::new(AtomicBool::new(true));
        let asked = parts.clone();
        let answer = adjusting.clone();
        queue.exec_sync(move || answer.store(asked.adjusting_focus(), Ordering::Release));
        let waited = started.elapsed();
        if waited >= FOCUS_START && !adjusting.load(Ordering::Acquire) {
            debug!("the focus ended after {} ms", waited.as_millis());
            return;
        }
        if waited >= FOCUS_LONGEST {
            debug!("the focus still moves after {} ms", waited.as_millis());
            return;
        }
    }
}

impl Drop for AppleSession {
    fn drop(&mut self) {
        // `stopRunning` blocks too, the last owner of the parts drops them on
        // the queue.
        let parts = self.parts.clone();
        self.queue.exec_async(move || drop(parts));
    }
}

/// RGBA of a picture of the video data output, BGRA with rows that may be
/// longer than the picture.
fn bgra_frame(sample: &CMSampleBuffer) -> Option<(Vec<u8>, u32, u32)> {
    let buffer = unsafe { sample.image_buffer() }?;
    let lock = CVPixelBufferLockFlags::ReadOnly;
    if unsafe { CVPixelBufferLockBaseAddress(&buffer, lock) } != kCVReturnSuccess {
        return None;
    }
    let width = CVPixelBufferGetWidth(&buffer);
    let height = CVPixelBufferGetHeight(&buffer);
    let row = CVPixelBufferGetBytesPerRow(&buffer);
    let base = CVPixelBufferGetBaseAddress(&buffer).cast::<u8>();
    let rgba = (!base.is_null() && row >= width * 4).then(|| {
        // SAFETY: the buffer is locked and holds `height` rows of `row`
        // bytes.
        let bytes = unsafe { slice::from_raw_parts(base, row * height) };
        let mut rgba = Vec::with_capacity(width * height * 4);
        for line in bytes.chunks_exact(row) {
            for [blue, green, red, _] in line[..width * 4].as_chunks::<4>().0 {
                rgba.extend([*red, *green, *blue, u8::MAX]);
            }
        }
        rgba
    });
    unsafe { CVPixelBufferUnlockBaseAddress(&buffer, lock) };
    let rgba = rgba?;
    // The sensor of a phone lies on its side, a quarter turn makes the
    // picture upright for a phone held up.
    #[cfg(target_os = "ios")]
    let (rgba, width, height) = turn(&rgba, width, height, 1);
    Some((rgba, u32::try_from(width).ok()?, u32::try_from(height).ok()?))
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements and the class has no
    // Drop.
    #[unsafe(super(NSObject))]
    #[name = "HilenCameraFrames"]
    #[ivars = FrameIvars]
    struct FrameDelegate;

    unsafe impl NSObjectProtocol for FrameDelegate {}

    unsafe impl AVCaptureVideoDataOutputSampleBufferDelegate for FrameDelegate {
        #[unsafe(method(captureOutput:didOutputSampleBuffer:fromConnection:))]
        fn did_output(
            &self,
            _output: &AVCaptureOutput,
            sample: &CMSampleBuffer,
            _connection: &AVCaptureConnection,
        ) {
            if let Some((rgba, width, height)) = bgra_frame(sample) {
                self.ivars().slot.put(rgba, width, height);
            }
        }
    }
);

impl FrameDelegate {
    fn new(slot: FrameSlot) -> Retained<Self> {
        DELEGATES.fetch_add(1, Ordering::AcqRel);
        let this = Self::alloc().set_ivars(FrameIvars { slot });
        unsafe { msg_send![super(this), init] }
    }
}

struct FrameIvars {
    slot: FrameSlot,
}

impl Drop for FrameIvars {
    fn drop(&mut self) {
        DELEGATES.fetch_sub(1, Ordering::AcqRel);
    }
}

struct PhotoIvars {
    done: Mutex<Option<Done<Photo>>>,
}

impl Drop for PhotoIvars {
    fn drop(&mut self) {
        DELEGATES.fetch_sub(1, Ordering::AcqRel);
    }
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements and the class has no
    // Drop.
    #[unsafe(super(NSObject))]
    #[name = "HilenCameraPhoto"]
    #[ivars = PhotoIvars]
    struct PhotoDelegate;

    unsafe impl NSObjectProtocol for PhotoDelegate {}

    unsafe impl AVCapturePhotoCaptureDelegate for PhotoDelegate {
        #[unsafe(method(captureOutput:didFinishProcessingPhoto:error:))]
        fn did_finish(
            &self,
            _output: &AVCapturePhotoOutput,
            photo: &AVCapturePhoto,
            error: Option<&NSError>,
        ) {
            let result = match error {
                Some(error) => Err(ns_error(error)),
                None => photo_of(photo),
            };
            match self.ivars().done.lock().take() {
                Some(done) => done(result),
                None => warn!("a camera gave a second photo for 1 request, it is dropped"),
            }
        }
    }
);

impl PhotoDelegate {
    fn new(done: Done<Photo>) -> Retained<Self> {
        DELEGATES.fetch_add(1, Ordering::AcqRel);
        let this = Self::alloc().set_ivars(PhotoIvars {
            done: Mutex::new(Some(done)),
        });
        unsafe { msg_send![super(this), init] }
    }

    fn finished(&self) -> bool {
        self.ivars().done.lock().is_none()
    }
}

fn photo_of(photo: &AVCapturePhoto) -> Result<Photo> {
    let bytes = unsafe { photo.fileDataRepresentation() }
        .ok_or_else(|| anyhow!("the photo has no file data"))?
        .to_vec();
    let size = unsafe { photo.resolvedSettings().photoDimensions() };
    let format = PhotoFormat::of(&bytes).ok_or_else(|| anyhow!("the photo is neither JPEG nor HEIC"))?;
    Ok(Photo {
        bytes,
        format,
        width: u32::try_from(size.width)?,
        height: u32::try_from(size.height)?,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    use super::{DELEGATES, FrameDelegate, PhotoDelegate};
    use crate::camera::frame::FrameSlot;

    /// A hot build deletes the classes of the delegates at its stop, a
    /// delegate that is not counted crashed a swap on an iPhone.
    #[test]
    fn a_delegate_is_counted_while_it_lives() {
        let before = DELEGATES.load(Ordering::Acquire);
        let frames = FrameDelegate::new(FrameSlot::default());
        let photo = PhotoDelegate::new(Box::new(drop));
        assert_eq!(DELEGATES.load(Ordering::Acquire), before + 2);
        drop(frames);
        drop(photo);
        assert_eq!(DELEGATES.load(Ordering::Acquire), before);
    }
}
