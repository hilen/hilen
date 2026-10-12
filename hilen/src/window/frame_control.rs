//! What the frame loop asks the inspector's frame record and frame step.
//! Without the `inspect` feature every answer is the normal run.

use crate::window::Screenshot;
#[cfg(feature = "inspect")]
use crate::window::{frame_record, frame_ring, frame_step};

/// Takes the picture of the frame that was just drawn.
pub(crate) type FrameReader = Box<dyn FnOnce(Screenshot) + Send>;

/// Before a frame that is not held. A record in a window with nothing to
/// pace its frames waits here for the next step of a screen.
#[cfg(not_wasm)]
pub(crate) fn pace() {
    #[cfg(feature = "inspect")]
    {
        frame_record::pace();
        frame_ring::pace();
    }
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
        frame_step::stepping() || frame_record::running() || frame_ring::running()
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

/// The frame being drawn goes to the screen now. A running record wants
/// to know when it gets there: the handler gets the seconds since then,
/// or `None` for a frame the screen never showed. Only Metal tells that.
#[cfg(all(feature = "inspect", apple))]
pub(crate) fn presented_handler() -> Option<Box<dyn FnOnce(Option<f64>) + Send>> {
    frame_ring::presented_handler()
}

/// The reader of the running records for the frame being drawn. A record
/// after an input and a record from a start to a stop can run together.
pub(crate) fn frame_reader() -> Option<FrameReader> {
    #[cfg(feature = "inspect")]
    {
        match (frame_record::take_frame(), frame_ring::take_frame()) {
            (Some(first), Some(second)) => Some(Box::new(move |shot: Screenshot| {
                first(shot.clone());
                second(shot);
            })),
            (first, second) => first.or(second),
        }
    }
    #[cfg(not(feature = "inspect"))]
    {
        None
    }
}
