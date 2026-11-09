//! What any thread reads of the window: the frame being drawn and whether
//! it draws offscreen. A build with no `gpu` feature has no window, there
//! `Window` is an empty type that answers frame 0 and headless.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

static RENDER_FRAME: AtomicU64 = AtomicU64::new(0);
/// Mirrors `Screen::Headless` so any thread can check it. Set once at
/// startup, never changes.
static HEADLESS: AtomicBool = AtomicBool::new(!cfg!(feature = "gpu"));

/// No window, this build has no GPU.
#[cfg(not(feature = "gpu"))]
pub struct Window;

impl super::Window {
    /// Rendering goes to an offscreen texture, there is no window and no
    /// display. Decided at startup. Callable from any thread.
    pub fn headless() -> bool {
        HEADLESS.load(Ordering::Relaxed)
    }

    #[cfg(all(not_wasm, feature = "gpu"))]
    pub(crate) fn set_headless() {
        HEADLESS.store(true, Ordering::Relaxed);
    }

    /// Index of the frame currently being rendered. Bumps once per rendered
    /// frame, before any draw code runs.
    pub fn render_frame() -> u64 {
        RENDER_FRAME.load(Ordering::Relaxed)
    }

    #[cfg(feature = "gpu")]
    pub(crate) fn next_render_frame() {
        RENDER_FRAME.fetch_add(1, Ordering::Relaxed);
    }
}
