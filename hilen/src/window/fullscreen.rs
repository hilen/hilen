//! Fullscreen for the app window. The app enters and leaves it, reads it and
//! hears about every change, also when the user does it with the system
//! button on the title bar.

use plat::Platform;
use winit::window::Fullscreen;

use crate::{deps::refs::main_lock::MainLock, ui::UIEvent, window::Window};

#[derive(Default)]
struct FullscreenState {
    on:      bool,
    changed: UIEvent<bool>,
}

static FULLSCREEN: MainLock<FullscreenState> = MainLock::new();

impl Window {
    /// Enters or leaves fullscreen, without a border and on the screen the
    /// window is on. A phone app always fills its screen, so there nothing
    /// changes.
    pub fn set_fullscreen(fullscreen: bool) {
        if Platform::MOBILE {
            return;
        }
        if let Some(window) = Self::winit_window() {
            window.set_fullscreen(fullscreen.then_some(Fullscreen::Borderless(None)));
        }
        record(fullscreen);
    }

    pub fn is_fullscreen() -> bool {
        Platform::MOBILE || FULLSCREEN.on
    }

    /// Fires with `true` when the window enters fullscreen and `false` when
    /// it leaves, by `set_fullscreen` or by the system alike.
    pub fn on_fullscreen() -> &'static UIEvent<bool> {
        &FULLSCREEN.changed
    }
}

/// Reads the state back from the system after the window changed its size,
/// that is how a press of the system's own fullscreen button is seen.
pub(crate) fn sync_fullscreen() {
    if let Some(window) = Window::winit_window() {
        record(window.fullscreen().is_some());
    }
}

/// A test that entered fullscreen and failed must not leave the window there
/// for the next test.
pub(crate) fn reset_fullscreen() {
    if FULLSCREEN.on {
        Window::set_fullscreen(false);
    }
}

fn record(fullscreen: bool) {
    if FULLSCREEN.on == fullscreen {
        return;
    }
    FULLSCREEN.get_mut().on = fullscreen;
    FULLSCREEN.changed.trigger(fullscreen);
}
