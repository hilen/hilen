//! Which ways the screen of a phone or a tablet may turn. A player asks for
//! landscape while a film shows and gives the choice back after it.

use log::debug;

use crate::{deps::refs::main_lock::MainLock, window::Window};

/// The ways the app lets the screen turn.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Orientations {
    /// Every way the device and the app's own settings allow.
    #[default]
    Any,
    Portrait,
    Landscape,
}

static ORIENTATIONS: MainLock<Orientations> = MainLock::new();

impl Window {
    /// Limits the ways the screen may turn. A screen that stands another
    /// way turns at once. On iOS the app's `Info.plist` has to list a way
    /// before the app can ask for it. A desktop window and a browser page
    /// do not turn, there the call only keeps the value. Android does not
    /// turn yet, see docs/roadmap.md. Main thread only.
    pub fn set_orientations(orientations: Orientations) {
        debug!("screen orientations: {orientations:?}");
        *ORIENTATIONS.get_mut() = orientations;
        #[cfg(all(ios, not(tvos)))]
        if let Some(window) = Self::winit_window() {
            use winit::platform::ios::{ValidOrientations, WindowExtIOS};

            window.set_valid_orientations(match orientations {
                Orientations::Any => ValidOrientations::LandscapeAndPortrait,
                Orientations::Portrait => ValidOrientations::Portrait,
                Orientations::Landscape => ValidOrientations::Landscape,
            });
        }
    }

    /// What the last `set_orientations` asked for, `Any` before the first.
    pub fn orientations() -> Orientations {
        *ORIENTATIONS
    }
}

/// A test that turned the screen and failed must not leave it turned for
/// the next test.
pub(crate) fn reset_orientations() {
    if *ORIENTATIONS != Orientations::Any {
        Window::set_orientations(Orientations::Any);
    }
}
