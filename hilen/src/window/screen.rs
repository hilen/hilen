use std::sync::Arc;

use crate::{gm::flat::Size, window::surface::Surface};

/// Where rendered frames go. `Windowed` presents to a real window,
/// `Headless` renders to an offscreen texture and never touches a display —
/// no winit window, no surface, no compositor.
pub(crate) enum Screen {
    Windowed {
        winit_window: Arc<winit::window::Window>,
        surface:      Option<Surface>,
        /// The inner size the last `Resized` event reported, in physical
        /// pixels. On X11 winit answers `inner_size` with a live round trip
        /// to the server, so during a drag two queries in one frame can
        /// disagree and the surface, the attachments and the scissor rects
        /// end up with different sizes. Every size query in a frame reads
        /// this one value instead.
        size:         Size<u32>,
    },
    #[cfg(not_wasm)]
    Headless { size: Size<u32> },
}

impl Screen {
    #[cfg_attr(target_arch = "wasm32", allow(clippy::unnecessary_wraps))]
    pub(crate) fn winit_window(&self) -> Option<&winit::window::Window> {
        match self {
            Self::Windowed { winit_window, .. } => Some(winit_window),
            #[cfg(not_wasm)]
            Self::Headless { .. } => None,
        }
    }
}
