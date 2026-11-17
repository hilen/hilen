//! A camera with no hardware behind it, for the tests of the engine and of
//! an app. Its picture is whatever the test shows.

use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::anyhow;
use image::ExtendedColorType;
use log::debug;
use parking_lot::Mutex;

use crate::{
    camera::{
        CameraDevice, CameraError, CameraLens, CameraLocks, CameraPosition, Photo, PhotoFormat,
        backend::{Backend, Done, Session},
        frame::FrameSlot,
        photo::encode_jpeg,
    },
    gm::flat::Point,
};

static INSTALLED: AtomicBool = AtomicBool::new(false);
static STATE: Mutex<State> = Mutex::new(State::new());

struct State {
    picture: Option<Picture>,
    refuse:  bool,
    locks:   CameraLocks,
    focus:   Option<Point>,
    open:    Vec<FrameSlot>,
}

impl State {
    const fn new() -> Self {
        Self {
            picture: None,
            refuse:  false,
            locks:   CameraLocks::NONE,
            focus:   None,
            open:    Vec::new(),
        }
    }
}

#[derive(Clone)]
struct Picture {
    rgba:   Vec<u8>,
    width:  u32,
    height: u32,
}

/// While installed, `Camera` lists and opens only this camera, with the id
/// `FakeCamera::ID`.
pub struct FakeCamera;

impl FakeCamera {
    pub const ID: &str = "fake";

    pub fn install() {
        *STATE.lock() = State::new();
        INSTALLED.store(true, Ordering::Relaxed);
    }

    /// The system cameras come back.
    pub fn remove() {
        INSTALLED.store(false, Ordering::Relaxed);
        *STATE.lock() = State::new();
    }

    /// The picture the camera sees from now on, RGBA with 4 bytes a pixel.
    /// An open camera gets it as a new frame at once.
    pub fn show(rgba: Vec<u8>, width: u32, height: u32) {
        let mut state = STATE.lock();
        for slot in &state.open {
            slot.put(rgba.clone(), width, height);
        }
        state.picture = Some(Picture { rgba, width, height });
    }

    /// The user says no to the camera, or yes again.
    pub fn refuse_access(refuse: bool) {
        STATE.lock().refuse = refuse;
    }

    /// The locks the last `set_locks` left.
    pub fn locks() -> CameraLocks {
        STATE.lock().locks
    }

    /// The point the last `focus_at` focused at.
    pub fn focus_point() -> Option<Point> {
        STATE.lock().focus
    }

    fn device() -> CameraDevice {
        CameraDevice {
            id:            Self::ID.to_string(),
            name:          "Fake camera".to_string(),
            position:      CameraPosition::Back,
            lens:          CameraLens::Wide,
            near_focus_mm: Some(100),
        }
    }
}

pub(super) fn is_installed() -> bool {
    INSTALLED.load(Ordering::Relaxed)
}

pub(super) struct Fake;

impl Backend for Fake {
    fn request_access(&self, done: Done<()>) {
        let refused = STATE.lock().refuse;
        done(if refused {
            Err(CameraError::Refused.into())
        } else {
            Ok(())
        });
    }

    fn devices(&self, done: Done<Vec<CameraDevice>>) {
        done(Ok(vec![FakeCamera::device()]));
    }

    fn open(&self, device: &CameraDevice, frames: FrameSlot, done: Done<Box<dyn Session>>) {
        if device.id != FakeCamera::ID {
            done(Err(CameraError::NotFound(device.name.clone()).into()));
            return;
        }
        let mut state = STATE.lock();
        if let Some(picture) = state.picture.clone() {
            frames.put(picture.rgba, picture.width, picture.height);
        }
        state.open.push(frames.clone());
        drop(state);
        done(Ok(Box::new(FakeSession { frames })));
    }
}

struct FakeSession {
    frames: FrameSlot,
}

impl Session for FakeSession {
    fn take_photo(&mut self, format: PhotoFormat, done: Done<Photo>) {
        let picture = STATE.lock().picture.clone();
        let Some(picture) = picture else {
            done(Err(anyhow!("the fake camera shows nothing yet")));
            return;
        };
        if format == PhotoFormat::Heic {
            debug!("the fake camera has no HEIC, it takes a JPEG");
        }
        let rgb: Vec<u8> = picture
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|[red, green, blue, _]| [*red, *green, *blue])
            .collect();
        done(encode_jpeg(
            &rgb,
            picture.width,
            picture.height,
            ExtendedColorType::Rgb8,
        ));
    }

    fn set_locks(&mut self, locks: CameraLocks, done: Done<CameraLocks>) {
        STATE.lock().locks = locks;
        done(Ok(locks));
    }

    fn focus_at(&mut self, point: Point, done: Done<bool>) {
        STATE.lock().focus = Some(point);
        done(Ok(true));
    }
}

impl Drop for FakeSession {
    fn drop(&mut self) {
        STATE.lock().open.retain(|slot| !slot.is(&self.frames));
    }
}
