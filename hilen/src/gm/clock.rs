use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use chrono::Utc;

use crate::gm::LossyConvert;

/// Virtual milliseconds one rendered frame adds in stepped mode, a 60 fps
/// timeline. Fractional on purpose, so a 0.4 s span lands on exactly 24 steps
/// instead of drifting with an integer rounding of 16 or 17.
pub const STEP_MS: f64 = 1000.0 / 60.0;

static STEPPED: AtomicBool = AtomicBool::new(false);

/// Virtual time in stepped mode, f64 milliseconds kept as raw bits so the
/// counter stays lock free. Only meaningful while `STEPPED` is set.
static VIRTUAL_MS: AtomicU64 = AtomicU64::new(0);

/// The one time source every animation reads. In a normal run it is the wall
/// clock. A test can switch it to frame stepped time, where it only moves when
/// the test asks for a frame, so an animation samples the same value on every
/// machine and its mid-flight frames become deterministic.
pub struct Clock;

impl Clock {
    /// Milliseconds the animation math measures against.
    pub(crate) fn now_ms() -> f64 {
        if STEPPED.load(Ordering::Relaxed) {
            f64::from_bits(VIRTUAL_MS.load(Ordering::Relaxed))
        } else {
            Utc::now().timestamp_millis().lossy_convert()
        }
    }

    /// Move virtual time on by one frame. A no-op in real time, so only a
    /// stepped test advances the clock and nothing else does, not even the free
    /// running headless render loop. That keeps the frame count a test drives
    /// the only thing that moves an animation.
    pub(crate) fn advance_frame() {
        if STEPPED.load(Ordering::Relaxed) {
            let now = f64::from_bits(VIRTUAL_MS.load(Ordering::Relaxed));
            VIRTUAL_MS.store((now + STEP_MS).to_bits(), Ordering::Relaxed);
        }
    }

    /// Switch to frame stepped time. Opt in per test, defaults untouched. The
    /// virtual clock starts at the current wall clock so an animation created
    /// before or after the switch still measures from a sane base.
    pub fn enter_stepped() {
        let now: f64 = Utc::now().timestamp_millis().lossy_convert();
        VIRTUAL_MS.store(now.to_bits(), Ordering::Relaxed);
        STEPPED.store(true, Ordering::Relaxed);
    }

    /// Back to real wall clock time. The runner also calls this before every
    /// test, so a stepped test never leaks its clock into the next one.
    pub fn exit_stepped() {
        STEPPED.store(false, Ordering::Relaxed);
    }

    pub fn is_stepped() -> bool {
        STEPPED.load(Ordering::Relaxed)
    }
}
