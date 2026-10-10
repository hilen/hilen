//! The time of the frame that is drawn, see `frame_seconds`.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::{deps::refs::main_lock::MainLock, gm::Clock, monotonic::monotonic_seconds};

mod follower;
#[cfg(test)]
mod follower_test;

use follower::FrameFollower;

/// The frame time of the newest frame, f64 seconds as raw bits.
static SECONDS: AtomicU64 = AtomicU64::new(0);

/// Between the begin and the end of a frame.
static IN_FRAME: AtomicBool = AtomicBool::new(false);

static FOLLOWER: MainLock<FrameFollower> = MainLock::new();

/// The time of the frame that is drawn now, in the seconds of
/// [`monotonic_seconds`]. It is the engine's best guess of when the frame
/// began on the even ticks of the screen.
///
/// Use it for everything that moves on screen by a clock: read it in
/// `update` and place the thing for that time. `monotonic_seconds` is the
/// moment the code runs, which is a few milliseconds early or late in each
/// frame, so a thing placed by it moves in uneven steps. This time goes on
/// by the refresh period of the screen, which the engine measures from the
/// frames, and it takes only a tenth of each new difference to the real
/// time. A dropped frame moves it by 2 or 3 periods. After a hold of the
/// loop of 0.1 seconds or more, like a covered window or a loop that
/// slept, it is at the real time at once. It never goes back, and it is
/// never more than 8 ms from the real time the frame began at.
///
/// Every read in 1 frame gives the same value, however late in the frame.
/// Outside of a frame, like in a touch handler or on another thread, it is
/// the real time, and never less than the time of the frame before.
/// `Animation` runs on this time.
///
/// It is the time the frame began, not the time the screen shows it. The
/// screen is later by a delay the engine does not know, about the same in
/// every frame.
///
/// In frame stepped time, see `Clock::enter_stepped` and `hilen-inspect
/// pause`, it stands still and moves by exactly 1 frame of 60 a second
/// with each stepped frame.
///
/// ```ignore
/// fn update(&mut self) {
///     let beat = self.clock.beats_at(frame_seconds());
///     self.line.set_x(beat * POINTS_PER_BEAT);
/// }
/// ```
pub fn frame_seconds() -> f64 {
    if Clock::is_stepped() {
        return Clock::stepped_seconds();
    }
    real_seconds()
}

fn real_seconds() -> f64 {
    let frame = f64::from_bits(SECONDS.load(Ordering::Relaxed));
    if IN_FRAME.load(Ordering::Relaxed) {
        frame
    } else {
        frame.max(monotonic_seconds())
    }
}

/// Before the update of a frame, on the main thread.
pub(crate) fn begin_frame() {
    let time = FOLLOWER.get_mut().advance(frame_start());
    SECONDS.store(time.to_bits(), Ordering::Relaxed);
    IN_FRAME.store(true, Ordering::Relaxed);
}

/// After the draw of a frame.
pub(crate) fn end_frame() {
    IN_FRAME.store(false, Ordering::Relaxed);
}

#[cfg(not_wasm)]
fn frame_start() -> f64 {
    monotonic_seconds()
}

/// A browser gives every frame the time of its refresh, which is already
/// even. The follower still gets it, so a browser that rounds this time
/// is evened out too.
#[cfg(wasm)]
fn frame_start() -> f64 {
    let now = monotonic_seconds();
    match crate::web::refresh_seconds() {
        // The page time is the one of the frame the browser runs now only
        // inside its frame callback. An old one is of a frame before.
        Some(refresh) if (-0.02..0.05).contains(&(now - refresh)) => refresh,
        _ => now,
    }
}
