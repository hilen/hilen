//! Windows next to the main one, see `docs/windows-more.md`. Each one has its
//! own root view, surface and frame state in a window slot, and shares the
//! GPU device, the pipelines, the fonts and the pictures with the main window.
//! A desktop has them, a phone and a page have 1 window.

#[cfg(not_wasm)]
mod registry;

#[cfg(wasm)]
use std::convert::Infallible;

use anyhow::Result;
#[cfg(wasm)]
use anyhow::bail;

#[cfg(not_wasm)]
pub(crate) use self::registry::{
    any_open, create, frame, headless_frames, request_redraws, slot_of_id, window_of,
};
use crate::{
    deps::{hreads::assert_main_thread, refs::Own},
    gm::{
        LossyConvert,
        flat::{Rect, Size},
    },
    ui::View,
    window::{Screenshot, Window},
};

/// A screen of the system, in points, with the origin at the top left of
/// the main screen.
#[derive(Clone, Debug, PartialEq)]
pub struct Display {
    pub name:  Option<String>,
    pub frame: Rect,
    pub scale: f32,
}

impl Display {
    /// Every screen, the main one first. A headless run has 1 screen of the
    /// size of its main window.
    pub fn all() -> Vec<Self> {
        assert_main_thread();
        let Some(window) = Window::winit_window() else {
            let size = Window::inner_size();
            return vec![Self {
                name:  None,
                frame: (0.0, 0.0, size.width, size.height).into(),
                scale: 1.0,
            }];
        };
        let primary = window.primary_monitor();
        let mut displays: Vec<(bool, Self)> = window
            .available_monitors()
            .map(|monitor| {
                let scale = monitor.scale_factor();
                let pos = monitor.position().to_logical::<f64>(scale);
                let size = monitor.size().to_logical::<f64>(scale);
                let (x, y, width, height): (f32, f32, f32, f32) = (
                    pos.x.lossy_convert(),
                    pos.y.lossy_convert(),
                    size.width.lossy_convert(),
                    size.height.lossy_convert(),
                );
                let display = Self {
                    name:  monitor.name(),
                    frame: (x, y, width, height).into(),
                    scale: scale.lossy_convert(),
                };
                (primary.as_ref() == Some(&monitor), display)
            })
            .collect();
        displays.sort_by_key(|(main, _)| !main);
        displays.into_iter().map(|(_, display)| display).collect()
    }
}

/// Where an extra window opens.
#[derive(Clone, Debug, PartialEq)]
pub enum WindowPlace {
    /// A frame in points, in the space of `Display::frame`.
    Frame(Rect),
    /// The whole of 1 screen, its menu bar and dock included.
    Cover(Display),
}

impl WindowPlace {
    #[cfg(not_wasm)]
    fn frame(&self) -> Rect {
        match self {
            Self::Frame(frame) => *frame,
            Self::Cover(display) => display.frame,
        }
    }
}

/// How an extra window stands among the windows of every app.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WindowLevel {
    #[default]
    Normal,
    /// Over every other window, the menu bar and the dock of a Mac too.
    /// The app that was in front gets the focus back when the last such
    /// window closes.
    AboveAll,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WindowSpec {
    pub title:       String,
    pub place:       WindowPlace,
    pub level:       WindowLevel,
    /// A title bar and an edge from the system.
    pub decorations: bool,
}

/// A window next to the main one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AppWindow {
    #[cfg(not_wasm)]
    slot:       usize,
    #[cfg(not_wasm)]
    generation: u32,
    /// A page has 1 window, so there is no other one to hold.
    #[cfg(wasm)]
    never:      Infallible,
}

/// The main slot and every extra window that is open.
pub(crate) fn open_slots() -> Vec<usize> {
    #[cfg(not_wasm)]
    return registry::open_slots();
    #[cfg(wasm)]
    vec![0]
}

#[cfg(not_wasm)]
impl AppWindow {
    /// Opens a window with the view `make_root` gives as its content. The
    /// view is made once the window is up, on a desktop in the next turn of
    /// the loop, headless at once. A phone has 1 window and answers with an
    /// error.
    pub fn open(spec: WindowSpec, make_root: impl FnOnce() -> Own<dyn View> + 'static) -> Result<Self> {
        registry::open(spec, Box::new(make_root))
    }

    /// The window in `slot` right now.
    pub(crate) fn of_slot(slot: usize) -> Option<Self> {
        registry::generation_of(slot).map(|generation| Self { slot, generation })
    }

    /// The window is open, also while it is still on its way up.
    pub fn is_open(self) -> bool {
        registry::is_open(self.slot, self.generation)
    }

    /// The window is up and its root view is made.
    pub fn is_ready(self) -> bool {
        registry::is_ready(self.slot, self.generation)
    }

    /// Runs `action` as if the window handled one of its own events, so
    /// `UIManager::root_view`, a modal or an alert land in this window.
    pub fn within<R>(self, action: impl FnOnce() -> R) -> Result<R> {
        registry::within_window(self.slot, self.generation, action)
    }

    /// The size of the content in points.
    pub fn size(self) -> Result<Size> {
        use crate::ui::ViewFrame;
        self.within(|| crate::ui::UIManager::root_view().size())
    }

    /// The window comes to the front and takes the keys.
    pub fn focus(self) {
        registry::focus(self.slot, self.generation);
    }

    /// Closes the window. Its views go at the next frame, so a button of the
    /// window can close it from its own action.
    pub fn close(self) {
        registry::close(self.slot, self.generation);
    }

    /// The next frame of the window as a picture. Not on the main thread,
    /// it waits for the frame.
    pub fn take_screenshot(self) -> Result<Screenshot> {
        let receiver = crate::deps::hreads::from_main(move || {
            self.within(|| {
                let receiver = Window::current().request_screenshot();
                crate::window::request_frame();
                receiver
            })
        })?;
        Ok(receiver.recv()?)
    }
}

#[cfg(wasm)]
impl AppWindow {
    pub fn open(spec: WindowSpec, make_root: impl FnOnce() -> Own<dyn View> + 'static) -> Result<Self> {
        drop(make_root);
        bail!("A page has 1 window, {} did not open", spec.title)
    }

    pub fn is_open(self) -> bool {
        match self.never {}
    }

    pub fn is_ready(self) -> bool {
        match self.never {}
    }

    pub fn within<R>(self, action: impl FnOnce() -> R) -> Result<R> {
        drop(action);
        match self.never {}
    }

    pub fn size(self) -> Result<Size> {
        match self.never {}
    }

    pub fn focus(self) {
        match self.never {}
    }

    pub fn close(self) {
        match self.never {}
    }

    pub fn take_screenshot(self) -> Result<Screenshot> {
        match self.never {}
    }
}
