use std::sync::LazyLock;

use web_time::Instant;

static START: LazyLock<Instant> = LazyLock::new(Instant::now);

/// Seconds on a clock that only goes forward, counted from its first use in
/// this process. A `SoundClock` and the time of a MIDI message are both
/// measured on it, so one can be compared with the other.
pub fn monotonic_seconds() -> f64 {
    START.elapsed().as_secs_f64()
}
