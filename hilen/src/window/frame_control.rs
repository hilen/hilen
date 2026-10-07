//! What the frame loop asks the inspector's frame record and frame step.
//! Without the `inspect` feature every answer is the normal run.

use crate::window::Screenshot;
#[cfg(feature = "inspect")]
use crate::window::{frame_record, frame_step};

/// Takes the picture of the frame that was just drawn.
pub(crate) type FrameReader = Box<dyn FnOnce(Screenshot) + Send>;

/// Before a frame that is not held. A record in a window with nothing to
/// pace its frames waits here for the next step of a screen.
#[cfg(not_wasm)]
pub(crate) fn pace() {
    #[cfg(feature = "inspect")]
    frame_record::pace();
}

/// Before the update of a frame that is not held.
pub(crate) fn begin_frame() {
    #[cfg(feature = "inspect")]
    frame_step::begin_frame();
}

/// After the draw of a frame that is not held.
pub(crate) fn end_frame() {
    #[cfg(feature = "inspect")]
    frame_step::end_frame();
}

/// The app is paused and no step is left. The loop then runs only the
/// queued callbacks, so the inspector still gets its answers, and draws
/// what it has again for a screenshot. It never updates, a layout pass
/// here would eat a frame the user wants to step.
pub(crate) fn holding() -> bool {
    #[cfg(feature = "inspect")]
    {
        frame_step::holding()
    }
    #[cfg(not(feature = "inspect"))]
    {
        false
    }
}

/// A step or a record needs its frames even from a covered window, they
/// are drawn offscreen like a screenshot of one.
#[cfg(not_wasm)]
pub(crate) fn offscreen_wanted() -> bool {
    #[cfg(feature = "inspect")]
    {
        frame_step::stepping() || frame_record::running()
    }
    #[cfg(not(feature = "inspect"))]
    {
        false
    }
}

/// A press, a key or a wheel turn came in. An armed record starts here.
pub(crate) fn input_arrived() {
    #[cfg(feature = "inspect")]
    frame_record::start();
}

/// The reader of a running record for the frame being drawn.
pub(crate) fn frame_reader() -> Option<FrameReader> {
    #[cfg(feature = "inspect")]
    {
        frame_record::take_frame()
    }
    #[cfg(not(feature = "inspect"))]
    {
        None
    }
}
