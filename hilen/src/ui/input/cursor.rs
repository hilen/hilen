use std::mem::take;

use log::{debug, warn};
use plat::Platform;
use winit::window::CursorGrabMode;

use crate::{deps::refs::main_lock::MainLock, gm::flat::Point, ui::UIEvent, window::Window};

#[derive(Default)]
struct CursorState {
    captured:   bool,
    /// Raw mouse motion since `take_motion` last ran.
    motion:     Point,
    on_capture: UIEvent<bool>,
}

static CURSOR: MainLock<CursorState> = MainLock::new();

/// The mouse of a game. Captured, it is hidden, kept inside the window
/// and reported as raw motion, the way a first person game turns its
/// camera. Escape, a lost window focus or `release` give it back.
pub struct Cursor;

impl Cursor {
    /// Hides the mouse and holds it in the window. A phone has no
    /// mouse, so there nothing changes and `captured` stays false.
    pub fn capture() {
        if Platform::MOBILE {
            return;
        }
        Self::set_captured(true);
    }

    pub fn release() {
        Self::set_captured(false);
    }

    pub fn captured() -> bool {
        CURSOR.captured
    }

    /// Fires with `true` when the mouse is captured and `false` when it
    /// is released, by `release`, Escape or a lost focus alike.
    pub fn on_capture() -> &'static UIEvent<bool> {
        &CURSOR.on_capture
    }

    /// The raw mouse motion since the last call, in the units the
    /// system reports, so a player turns by it once per step. Zero
    /// while the mouse is free.
    pub fn take_motion() -> Point {
        take(&mut CURSOR.get_mut().motion)
    }

    pub(crate) fn add_motion(delta: Point) {
        if Self::captured() {
            CURSOR.get_mut().motion += delta;
        }
    }

    /// A test that captured the mouse and failed must not leave it
    /// captured for the next test.
    pub(crate) fn reset() {
        Self::release();
        CURSOR.get_mut().motion = Point::default();
    }

    fn set_captured(captured: bool) {
        if Self::captured() == captured {
            return;
        }
        let state = CURSOR.get_mut();
        state.captured = captured;
        state.motion = Point::default();
        if let Some(window) = Window::winit_window() {
            grab(window, captured);
        }
        Self::on_capture().trigger(captured);
    }
}

fn grab(window: &winit::window::Window, captured: bool) {
    let mode = if captured {
        CursorGrabMode::Locked
    } else {
        CursorGrabMode::None
    };
    // Windows and X11 cannot lock the cursor in place, only keep it in
    // the window. Hidden and read as raw motion that looks the same.
    let result = window.set_cursor_grab(mode).or_else(|error| {
        if captured {
            debug!("Locking the cursor failed, confining it: {error}");
            window.set_cursor_grab(CursorGrabMode::Confined)
        } else {
            Err(error)
        }
    });
    if let Err(error) = result {
        warn!("Failed to grab the cursor: {error}");
    }
    window.set_cursor_visible(!captured);
}
