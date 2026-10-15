//! The NDK camera of Android, 1 capture session with 2 image readers: YUV
//! at a small size for the live picture, JPEG at the largest size for the
//! photo. The camera permission is asked through the activity, and the
//! answer is read back when the app has the focus again.

use std::{
    cmp::Reverse,
    collections::VecDeque,
    ffi::{CStr, CString, c_int, c_void},
    ptr::null_mut,
    slice::from_raw_parts,
    sync::Arc,
    thread::{sleep, spawn},
    time::{Duration, Instant},
};

use anyhow::{Result, anyhow};
use jni::{jni_sig, jni_str, objects::JValue};
use log::{debug, error, warn};
use ndk::media::image_reader::{AcquireResult, Image, ImageFormat, ImageReader};
use ndk_sys::{
    ACameraCaptureSession, ACameraCaptureSession_capture, ACameraCaptureSession_close,
    ACameraCaptureSession_setRepeatingRequest, ACameraCaptureSession_stateCallbacks,
    ACameraCaptureSession_stopRepeating, ACameraDevice, ACameraDevice_StateCallbacks, ACameraDevice_close,
    ACameraDevice_createCaptureRequest, ACameraDevice_createCaptureSession, ACameraDevice_request_template,
    ACameraIdList, ACameraManager, ACameraManager_create, ACameraManager_delete,
    ACameraManager_deleteCameraIdList, ACameraManager_getCameraCharacteristics,
    ACameraManager_getCameraIdList, ACameraManager_openCamera, ACameraMetadata, ACameraMetadata_const_entry,
    ACameraMetadata_const_entry__bindgen_ty_1, ACameraMetadata_free, ACameraMetadata_getConstEntry,
    ACameraOutputTarget, ACameraOutputTarget_create, ACameraOutputTarget_free, ACaptureRequest,
    ACaptureRequest_addTarget, ACaptureRequest_free, ACaptureRequest_setEntry_u8, ACaptureSessionOutput,
    ACaptureSessionOutput_create, ACaptureSessionOutput_free, ACaptureSessionOutputContainer,
    ACaptureSessionOutputContainer_add, ACaptureSessionOutputContainer_create,
    ACaptureSessionOutputContainer_free, AIMAGE_FORMATS, acamera_metadata_enum_acamera_control_af_mode,
    acamera_metadata_enum_acamera_control_af_trigger, acamera_metadata_enum_acamera_lens_facing,
    acamera_metadata_tag, camera_status_t,
};
use parking_lot::Mutex;

use crate::{
    camera::{
        CameraDevice, CameraError, CameraFrame, CameraLens, CameraLocks, CameraPosition, Photo, PhotoFormat,
        backend::{Backend, Done, Session},
        frame::{FrameSlot, Plane, turn, yuv420_to_rgba},
    },
    deps::hreads::from_main,
    system::{
        AppActivity,
        android_jni::{clear_exception, with_activity},
    },
};

// The camera calls of the NDK are in their own library.
#[link(name = "camera2ndk")]
unsafe extern "C" {}

const PERMISSION: &str = "android.permission.CAMERA";
/// `PackageManager.PERMISSION_GRANTED`.
const GRANTED: i32 = 0;
const REQUEST_CODE: i32 = 4_011;
/// How often the answer of the user is looked for.
const POLL: Duration = Duration::from_millis(200);
/// Android shows its question at once. An app whose permission was refused
/// for good gets no question, its focus never goes away.
const NO_QUESTION: Duration = Duration::from_secs(2);
const ANSWER_LIMIT: Duration = Duration::from_secs(300);
const JPEG_QUALITY: u8 = 95;
const PREVIEW_IMAGES: i32 = 3;
const PHOTO_IMAGES: i32 = 2;

pub(super) struct System;

impl Backend for System {
    fn request_access(&self, done: Done<()>) {
        match granted() {
            Ok(true) => return done(Ok(())),
            Ok(false) => {}
            Err(err) => return done(Err(err)),
        }
        if let Err(err) = ask() {
            return done(Err(err));
        }
        debug!("asking the user for the camera");
        spawn(move || done(wait_for_answer()));
    }

    fn devices(&self, done: Done<Vec<CameraDevice>>) {
        done(devices());
    }

    fn open(&self, device: &CameraDevice, frames: FrameSlot, done: Done<Box<dyn Session>>) {
        done(AndroidSession::open(device, frames).map(|session| Box::new(session) as Box<dyn Session>));
    }
}

fn granted() -> Result<bool> {
    with_activity(|env, activity| {
        let name = env.new_string(PERMISSION)?;
        let state = env
            .call_method(
                activity,
                jni_str!("checkSelfPermission"),
                jni_sig!("(Ljava/lang/String;)I"),
                &[JValue::Object(&name)],
            )
            .inspect_err(|_| clear_exception(env))?
            .i()?;
        Ok(state == GRANTED)
    })
}

fn ask() -> Result<()> {
    with_activity(|env, activity| {
        let name = env.new_string(PERMISSION)?;
        let names = env.new_object_array(1, jni_str!("java/lang/String"), &name)?;
        env.call_method(
            activity,
            jni_str!("requestPermissions"),
            jni_sig!("([Ljava/lang/String;I)V"),
            &[JValue::Object(&names), JValue::Int(REQUEST_CODE)],
        )
        .inspect_err(|_| clear_exception(env))?;
        Ok(())
    })
}

/// Only Java gets the answer, so the engine waits for the question to go
/// away. The question takes the focus from the app, the answer gives it
/// back, then the permission tells the answer.
fn wait_for_answer() -> Result<()> {
    let started = Instant::now();
    let mut asked = false;
    loop {
        sleep(POLL);
        if granted()? {
            return Ok(());
        }
        let active = from_main(AppActivity::is_active);
        if !active {
            asked = true;
        } else if asked || started.elapsed() > NO_QUESTION {
            return Err(CameraError::Refused.into());
        }
        if started.elapsed() > ANSWER_LIMIT {
            return Err(CameraError::Refused.into());
        }
    }
}

fn check(status: camera_status_t, what: &str) -> Result<()> {
    if status == camera_status_t::ACAMERA_OK {
        Ok(())
    } else {
        Err(anyhow!("{what} failed with camera status {}", status.0))
    }
}

/// The camera manager, deleted with the value.
struct Manager(*mut ACameraManager);

impl Manager {
    fn new() -> Result<Self> {
        let manager = unsafe { ACameraManager_create() };
        if manager.is_null() {
            return Err(anyhow!("Android has no camera manager"));
        }
        Ok(Self(manager))
    }

    fn ids(&self) -> Result<Vec<String>> {
        let mut list: *mut ACameraIdList = null_mut();
        check(
            unsafe { ACameraManager_getCameraIdList(self.0, &raw mut list) },
            "the camera list",
        )?;
        let ids = unsafe {
            let list = &*list;
            (0..usize::try_from(list.numCameras)?)
                .map(|index| CStr::from_ptr(*list.cameraIds.add(index)).to_string_lossy().into_owned())
                .collect()
        };
        unsafe { ACameraManager_deleteCameraIdList(list) };
        Ok(ids)
    }

    fn characteristics(&self, id: &str) -> Result<Metadata> {
        let id = CString::new(id)?;
        let mut metadata: *mut ACameraMetadata = null_mut();
        check(
            unsafe { ACameraManager_getCameraCharacteristics(self.0, id.as_ptr(), &raw mut metadata) },
            "the camera characteristics",
        )?;
        Ok(Metadata(metadata))
    }
}

impl Drop for Manager {
    fn drop(&mut self) {
        unsafe { ACameraManager_delete(self.0) };
    }
}

/// The characteristics of 1 camera, freed with the value.
struct Metadata(*mut ACameraMetadata);

impl Metadata {
    fn entry(&self, tag: acamera_metadata_tag) -> Option<ACameraMetadata_const_entry> {
        let mut entry = ACameraMetadata_const_entry {
            tag:   0,
            type_: 0,
            count: 0,
            data:  ACameraMetadata_const_entry__bindgen_ty_1 { u8_: null_mut() },
        };
        let status = unsafe { ACameraMetadata_getConstEntry(self.0, tag.0, &raw mut entry) };
        (status == camera_status_t::ACAMERA_OK).then_some(entry)
    }

    /// The entry of `tag` read as `T`, the type the tag has.
    fn values<T: Copy>(
        &self,
        tag: acamera_metadata_tag,
        data: fn(&ACameraMetadata_const_entry) -> *const T,
    ) -> Vec<T> {
        self.entry(tag)
            .and_then(|entry| {
                let count = usize::try_from(entry.count).ok()?;
                Some(unsafe { from_raw_parts(data(&entry), count) }.to_vec())
            })
            .unwrap_or_default()
    }

    fn bytes(&self, tag: acamera_metadata_tag) -> Vec<u8> {
        self.values(tag, |entry| unsafe { entry.data.u8_ })
    }

    fn ints(&self, tag: acamera_metadata_tag) -> Vec<i32> {
        self.values(tag, |entry| unsafe { entry.data.i32_ })
    }

    fn floats(&self, tag: acamera_metadata_tag) -> Vec<f32> {
        self.values(tag, |entry| unsafe { entry.data.f })
    }

    /// The sizes the camera gives for `format`, largest first.
    fn sizes(&self, format: AIMAGE_FORMATS) -> Vec<(i32, i32)> {
        let Ok(format) = i32::try_from(format.0) else {
            return Vec::new();
        };
        let mut sizes: Vec<(i32, i32)> = self
            .ints(acamera_metadata_tag::ACAMERA_SCALER_AVAILABLE_STREAM_CONFIGURATIONS)
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|[kind, _, _, input]| *kind == format && *input == 0)
            .map(|[_, width, height, _]| (*width, *height))
            .collect();
        sizes.sort_by_key(|(width, height)| Reverse(i64::from(*width) * i64::from(*height)));
        sizes
    }
}

impl Drop for Metadata {
    fn drop(&mut self) {
        unsafe { ACameraMetadata_free(self.0) };
    }
}

fn devices() -> Result<Vec<CameraDevice>> {
    let manager = Manager::new()?;
    let mut found = Vec::new();
    for id in manager.ids()? {
        let metadata = manager.characteristics(&id)?;
        let facing = metadata.bytes(acamera_metadata_tag::ACAMERA_LENS_FACING).first().copied();
        let position = match facing.map(u32::from) {
            Some(face) if face == acamera_metadata_enum_acamera_lens_facing::ACAMERA_LENS_FACING_BACK.0 => {
                CameraPosition::Back
            }
            Some(face) if face == acamera_metadata_enum_acamera_lens_facing::ACAMERA_LENS_FACING_FRONT.0 => {
                CameraPosition::Front
            }
            Some(_) => CameraPosition::External,
            None => CameraPosition::Unknown,
        };
        let focal = metadata
            .floats(acamera_metadata_tag::ACAMERA_LENS_INFO_AVAILABLE_FOCAL_LENGTHS)
            .first()
            .copied();
        found.push((id, position, focal));
    }

    // Android names no lens. The first back camera is the main one, a
    // shorter focal length than it is the ultra wide, a much longer one
    // the telephoto.
    let main = found
        .iter()
        .find(|(_, position, _)| *position == CameraPosition::Back)
        .and_then(|(_, _, focal)| *focal);
    Ok(found
        .into_iter()
        .map(|(id, position, focal)| {
            let lens = match (main, focal, position) {
                (Some(main), Some(focal), CameraPosition::Back) if focal < main * 0.8 => {
                    CameraLens::UltraWide
                }
                (Some(main), Some(focal), CameraPosition::Back) if focal > main * 1.5 => {
                    CameraLens::Telephoto
                }
                (_, _, CameraPosition::Back | CameraPosition::Front) => CameraLens::Wide,
                _ => CameraLens::Unknown,
            };
            let side = match position {
                CameraPosition::Back => "Back",
                CameraPosition::Front => "Front",
                CameraPosition::External => "External",
                CameraPosition::Unknown => "Other",
            };
            CameraDevice {
                name: format!("{side} camera {id}"),
                id,
                position,
                lens,
            }
        })
        .collect())
}

/// What the camera can lock.
#[derive(Clone, Copy)]
struct Can {
    focus:         bool,
    exposure:      bool,
    white_balance: bool,
}

/// The parts of the live picture and the photo the callbacks of the
/// readers share.
struct Shared {
    name:     String,
    frames:   FrameSlot,
    /// Quarter turns that make the picture of the sensor upright.
    quarters: u32,
    photos:   Mutex<VecDeque<Done<Photo>>>,
}

struct AndroidSession {
    shared:    Arc<Shared>,
    can:       Can,
    locks:     CameraLocks,
    // Dropped last, after the session and the device are closed.
    manager:   Manager,
    device:    *mut ACameraDevice,
    container: *mut ACaptureSessionOutputContainer,
    outputs:   Vec<*mut ACaptureSessionOutput>,
    preview:   *mut ACameraOutputTarget,
    photo:     *mut ACameraOutputTarget,
    session:   *mut ACameraCaptureSession,
    readers:   Vec<ImageReader>,
}

// SAFETY: the NDK camera objects may be used from any thread, the session
// is only used behind the lock of its `Camera`.
unsafe impl Send for AndroidSession {}

unsafe extern "C" fn device_disconnected(_context: *mut c_void, _device: *mut ACameraDevice) {
    warn!("a camera was disconnected");
}

unsafe extern "C" fn device_error(_context: *mut c_void, _device: *mut ACameraDevice, error: c_int) {
    error!("a camera failed with error {error}");
}

unsafe extern "C" fn session_state(_context: *mut c_void, _session: *mut ACameraCaptureSession) {}

impl AndroidSession {
    fn open(device: &CameraDevice, frames: FrameSlot) -> Result<Self> {
        let manager = Manager::new()?;
        let metadata = manager.characteristics(&device.id)?;
        let photo_size = *metadata
            .sizes(AIMAGE_FORMATS::AIMAGE_FORMAT_JPEG)
            .first()
            .ok_or_else(|| anyhow!("camera {} takes no JPEG", device.name))?;
        let preview_size = preview_size(
            &metadata.sizes(AIMAGE_FORMATS::AIMAGE_FORMAT_YUV_420_888),
            photo_size,
        )
        .ok_or_else(|| anyhow!("camera {} gives no YUV picture", device.name))?;
        let can = Can {
            focus:         metadata
                .bytes(acamera_metadata_tag::ACAMERA_CONTROL_AF_AVAILABLE_MODES)
                .contains(&u8::try_from(
                    acamera_metadata_enum_acamera_control_af_mode::ACAMERA_CONTROL_AF_MODE_AUTO.0,
                )?),
            exposure:      metadata.bytes(acamera_metadata_tag::ACAMERA_CONTROL_AE_LOCK_AVAILABLE).first()
                == Some(&1),
            white_balance: metadata.bytes(acamera_metadata_tag::ACAMERA_CONTROL_AWB_LOCK_AVAILABLE).first()
                == Some(&1),
        };
        let orientation = metadata
            .ints(acamera_metadata_tag::ACAMERA_SENSOR_ORIENTATION)
            .first()
            .copied()
            .unwrap_or(0);
        drop(metadata);
        debug!(
            "camera {} photo {photo_size:?}, preview {preview_size:?}, sensor at {orientation} degrees",
            device.name
        );

        let shared = Arc::new(Shared {
            name: device.name.clone(),
            frames,
            quarters: u32::try_from(orientation / 90).unwrap_or(0),
            photos: Mutex::new(VecDeque::new()),
        });
        let preview_reader = preview_reader(preview_size, shared.clone())?;
        let photo_reader = photo_reader(photo_size, shared.clone())?;

        let mut this = Self {
            shared,
            can,
            locks: CameraLocks::NONE,
            manager,
            device: null_mut(),
            container: null_mut(),
            outputs: Vec::new(),
            preview: null_mut(),
            photo: null_mut(),
            session: null_mut(),
            readers: Vec::new(),
        };
        // From here a failure drops `this`, which frees what was made.
        this.start(&device.id, &preview_reader, &photo_reader)?;
        this.readers = vec![preview_reader, photo_reader];
        this.repeat()?;
        Ok(this)
    }

    fn start(&mut self, id: &str, preview: &ImageReader, photo: &ImageReader) -> Result<()> {
        let id = CString::new(id)?;
        let mut callbacks = ACameraDevice_StateCallbacks {
            context:        null_mut(),
            onDisconnected: Some(device_disconnected),
            onError:        Some(device_error),
        };
        check(
            unsafe {
                ACameraManager_openCamera(
                    self.manager.0,
                    id.as_ptr(),
                    &raw mut callbacks,
                    &raw mut self.device,
                )
            },
            "opening the camera",
        )?;
        check(
            unsafe { ACaptureSessionOutputContainer_create(&raw mut self.container) },
            "the outputs",
        )?;
        for (reader, target) in [(preview, &mut self.preview), (photo, &mut self.photo)] {
            let window = reader.window()?.ptr().as_ptr();
            let mut output: *mut ACaptureSessionOutput = null_mut();
            check(
                unsafe { ACaptureSessionOutput_create(window, &raw mut output) },
                "an output",
            )?;
            self.outputs.push(output);
            check(
                unsafe { ACaptureSessionOutputContainer_add(self.container, output) },
                "adding an output",
            )?;
            check(unsafe { ACameraOutputTarget_create(window, target) }, "a target")?;
        }
        let callbacks = ACameraCaptureSession_stateCallbacks {
            context:  null_mut(),
            onClosed: Some(session_state),
            onReady:  Some(session_state),
            onActive: Some(session_state),
        };
        check(
            unsafe {
                ACameraDevice_createCaptureSession(
                    self.device,
                    self.container,
                    &raw const callbacks,
                    &raw mut self.session,
                )
            },
            "the capture session",
        )
    }

    /// A request with the locks of the session on `targets`. Freed by the
    /// caller.
    fn request(
        &self,
        template: ACameraDevice_request_template,
        targets: &[*mut ACameraOutputTarget],
        trigger: Option<acamera_metadata_enum_acamera_control_af_trigger>,
    ) -> Result<*mut ACaptureRequest> {
        let mut request: *mut ACaptureRequest = null_mut();
        check(
            unsafe { ACameraDevice_createCaptureRequest(self.device, template, &raw mut request) },
            "a capture request",
        )?;
        let set = |tag: acamera_metadata_tag, value: u8| {
            check(
                unsafe { ACaptureRequest_setEntry_u8(request, tag.0, 1, &raw const value) },
                "a request value",
            )
        };
        let built = (|| {
            for target in targets {
                check(
                    unsafe { ACaptureRequest_addTarget(request, *target) },
                    "a request target",
                )?;
            }
            let focus = if self.locks.focus && self.can.focus {
                acamera_metadata_enum_acamera_control_af_mode::ACAMERA_CONTROL_AF_MODE_AUTO
            } else {
                acamera_metadata_enum_acamera_control_af_mode::ACAMERA_CONTROL_AF_MODE_CONTINUOUS_PICTURE
            };
            set(
                acamera_metadata_tag::ACAMERA_CONTROL_AF_MODE,
                u8::try_from(focus.0)?,
            )?;
            if self.can.exposure {
                set(
                    acamera_metadata_tag::ACAMERA_CONTROL_AE_LOCK,
                    u8::from(self.locks.exposure),
                )?;
            }
            if self.can.white_balance {
                set(
                    acamera_metadata_tag::ACAMERA_CONTROL_AWB_LOCK,
                    u8::from(self.locks.white_balance),
                )?;
            }
            if let Some(trigger) = trigger {
                set(
                    acamera_metadata_tag::ACAMERA_CONTROL_AF_TRIGGER,
                    u8::try_from(trigger.0)?,
                )?;
            }
            set(acamera_metadata_tag::ACAMERA_JPEG_QUALITY, JPEG_QUALITY)
        })();
        match built {
            Ok(()) => Ok(request),
            Err(err) => {
                unsafe { ACaptureRequest_free(request) };
                Err(err)
            }
        }
    }

    /// The live picture, with the locks of the session.
    fn repeat(&self) -> Result<()> {
        let mut request = self.request(
            ACameraDevice_request_template::TEMPLATE_PREVIEW,
            &[self.preview],
            None,
        )?;
        let status = unsafe {
            ACameraCaptureSession_setRepeatingRequest(
                self.session,
                null_mut(),
                1,
                &raw mut request,
                null_mut(),
            )
        };
        unsafe { ACaptureRequest_free(request) };
        check(status, "the live picture")
    }

    fn capture(
        &self,
        template: ACameraDevice_request_template,
        target: *mut ACameraOutputTarget,
        trigger: Option<acamera_metadata_enum_acamera_control_af_trigger>,
    ) -> Result<()> {
        let mut request = self.request(template, &[target], trigger)?;
        let status = unsafe {
            ACameraCaptureSession_capture(self.session, null_mut(), 1, &raw mut request, null_mut())
        };
        unsafe { ACaptureRequest_free(request) };
        check(status, "a capture")
    }
}

impl Session for AndroidSession {
    fn take_photo(&mut self, format: PhotoFormat, done: Done<Photo>) {
        if format == PhotoFormat::Heic {
            debug!("the NDK camera gives no HEIC, the photo is a JPEG");
        }
        self.shared.photos.lock().push_back(done);
        if let Err(err) = self.capture(
            ACameraDevice_request_template::TEMPLATE_STILL_CAPTURE,
            self.photo,
            None,
        ) && let Some(done) = self.shared.photos.lock().pop_back()
        {
            done(Err(err));
        }
    }

    fn set_locks(&mut self, locks: CameraLocks, done: Done<CameraLocks>) {
        let focus_was = self.locks.focus;
        self.locks = locks;
        let result = self.repeat().and_then(|()| {
            // Auto focus locks after 1 trigger, a cancel frees it again.
            let trigger = match (focus_was, locks.focus) {
                (false, true) => {
                    Some(acamera_metadata_enum_acamera_control_af_trigger::ACAMERA_CONTROL_AF_TRIGGER_START)
                }
                (true, false) => {
                    Some(acamera_metadata_enum_acamera_control_af_trigger::ACAMERA_CONTROL_AF_TRIGGER_CANCEL)
                }
                _ => None,
            };
            match trigger {
                Some(trigger) if self.can.focus => self.capture(
                    ACameraDevice_request_template::TEMPLATE_PREVIEW,
                    self.preview,
                    Some(trigger),
                ),
                _ => Ok(()),
            }
        });
        done(result.map(|()| CameraLocks {
            focus:         locks.focus && self.can.focus,
            exposure:      locks.exposure && self.can.exposure,
            white_balance: locks.white_balance && self.can.white_balance,
        }));
    }
}

impl Drop for AndroidSession {
    fn drop(&mut self) {
        unsafe {
            if !self.session.is_null() {
                if ACameraCaptureSession_stopRepeating(self.session) != camera_status_t::ACAMERA_OK {
                    warn!("camera {} did not stop its live picture", self.shared.name);
                }
                ACameraCaptureSession_close(self.session);
            }
            if !self.device.is_null() && ACameraDevice_close(self.device) != camera_status_t::ACAMERA_OK {
                warn!("camera {} did not close", self.shared.name);
            }
            for target in [self.preview, self.photo] {
                if !target.is_null() {
                    ACameraOutputTarget_free(target);
                }
            }
            for output in self.outputs.drain(..) {
                ACaptureSessionOutput_free(output);
            }
            if !self.container.is_null() {
                ACaptureSessionOutputContainer_free(self.container);
            }
        }
        for done in self.shared.photos.lock().drain(..) {
            done(Err(anyhow!("the camera closed before the photo came")));
        }
        debug!(
            "camera {} closed after {} frames",
            self.shared.name,
            self.shared.frames.count()
        );
    }
}

/// The largest YUV size up to `CameraFrame::MAX_SIDE` with the shape of the
/// photo, or the nearest shape when none has it.
fn preview_size(sizes: &[(i32, i32)], photo: (i32, i32)) -> Option<(i32, i32)> {
    let max = i32::try_from(CameraFrame::MAX_SIDE).ok()?;
    let shape = |(width, height): (i32, i32)| f64::from(width) / f64::from(height.max(1));
    let wanted = shape(photo);
    sizes
        .iter()
        .copied()
        .filter(|(width, height)| (*width).max(*height) <= max)
        .min_by(|a, b| {
            let off = |size| (shape(size) - wanted).abs();
            off(*a)
                .total_cmp(&off(*b))
                .then((i64::from(b.0) * i64::from(b.1)).cmp(&(i64::from(a.0) * i64::from(a.1))))
        })
}

fn preview_reader((width, height): (i32, i32), shared: Arc<Shared>) -> Result<ImageReader> {
    let mut reader = ImageReader::new(width, height, ImageFormat::YUV_420_888, PREVIEW_IMAGES)?;
    let mut failed = false;
    reader.set_image_listener(Box::new(move |reader| {
        let picture = match reader.acquire_latest_image() {
            Ok(AcquireResult::Image(image)) => rgba_of(&image, shared.quarters),
            Ok(_) => return,
            Err(err) => Err(anyhow!("{err}")),
        };
        match picture {
            Ok((rgba, width, height)) => shared.frames.put(rgba, width, height),
            // The next pictures fail the same way, 1 line is enough.
            Err(err) if !failed => {
                failed = true;
                warn!("a picture of camera {} failed: {err}", shared.name);
            }
            Err(_) => {}
        }
    }))?;
    Ok(reader)
}

fn rgba_of(image: &Image, quarters: u32) -> Result<(Vec<u8>, u32, u32)> {
    let width = usize::try_from(image.width()?)?;
    let height = usize::try_from(image.height()?)?;
    let plane = |index: i32| -> Result<Plane<'_>> {
        Ok(Plane {
            data:         image.plane_data(index)?,
            row_stride:   usize::try_from(image.plane_row_stride(index)?)?,
            pixel_stride: usize::try_from(image.plane_pixel_stride(index)?)?,
        })
    };
    let rgba = yuv420_to_rgba(&plane(0)?, &plane(1)?, &plane(2)?, width, height)
        .ok_or_else(|| anyhow!("a YUV picture is shorter than its size"))?;
    let (rgba, width, height) = turn(&rgba, width, height, quarters);
    Ok((rgba, u32::try_from(width)?, u32::try_from(height)?))
}

fn photo_reader((width, height): (i32, i32), shared: Arc<Shared>) -> Result<ImageReader> {
    let mut reader = ImageReader::new(width, height, ImageFormat::JPEG, PHOTO_IMAGES)?;
    reader.set_image_listener(Box::new(move |reader| {
        let photo = match reader.acquire_next_image() {
            Ok(AcquireResult::Image(image)) => photo_of(&image),
            Ok(_) => return,
            Err(err) => Err(anyhow!("{err}")),
        };
        match shared.photos.lock().pop_front() {
            Some(done) => done(photo),
            None => warn!("camera {} gave a photo nobody asked for", shared.name),
        }
    }))?;
    Ok(reader)
}

/// The JPEG buffer of the camera is longer than the file, it ends at the
/// last end marker.
fn photo_of(image: &Image) -> Result<Photo> {
    let data = image.plane_data(0)?;
    let end = data
        .windows(2)
        .rposition(|marker| marker == [0xFF, 0xD9])
        .map_or(data.len(), |at| at + 2);
    Ok(Photo {
        bytes:  data[..end].to_vec(),
        format: PhotoFormat::Jpeg,
        width:  u32::try_from(image.width()?)?,
        height: u32::try_from(image.height()?)?,
    })
}
