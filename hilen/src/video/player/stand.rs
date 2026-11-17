//! A clock that stopped. The sound is the clock of a video, and its own read
//! can stall with the stream.

/// Seconds the next frame may be late before playback holds for it. A slow
/// frame or 2 drop as before, a stalled stream holds the clock.
pub(super) const STALL: f64 = 0.25;

/// A place of the clock and the time it was first seen there.
#[derive(Clone, Copy)]
pub(super) struct Stand {
    position: f64,
    since_ms: f64,
}

/// True once the clock has stood at `position` for `STALL` seconds. The
/// sound is the clock, and its own read can stall with the stream. Then
/// the position never gets late against the next frame, it just stops.
pub(super) fn clock_stands(stand: &mut Option<Stand>, position: f64, now_ms: f64) -> bool {
    match stand {
        Some(stand) if (stand.position - position).abs() < f64::EPSILON => {
            now_ms - stand.since_ms >= STALL * 1000.0
        }
        _ => {
            *stand = Some(Stand {
                position,
                since_ms: now_ms,
            });
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{STALL, clock_stands};

    /// The sound is the clock, and on a stalled stream its own read can run
    /// dry before the picture is late. Playback then stood still in the
    /// Playing state, with no way for an app to show that it waits.
    #[test]
    fn a_clock_that_stopped_counts_as_stalled_after_the_stall_time() {
        let mut stand = None;
        let stall_ms = STALL * 1000.0;

        assert!(!clock_stands(&mut stand, 2.5, 1000.0));
        assert!(!clock_stands(&mut stand, 2.5, 1000.0 + stall_ms - 1.0));
        assert!(clock_stands(&mut stand, 2.5, 1000.0 + stall_ms));
    }

    #[test]
    fn a_clock_that_moves_is_never_stalled() {
        let mut stand = None;

        assert!(!clock_stands(&mut stand, 2.0, 1000.0));
        assert!(!clock_stands(&mut stand, 2.2, 2000.0));
        assert!(!clock_stands(&mut stand, 2.4, 3000.0));
        // It stopped only now, the time before does not count.
        assert!(!clock_stands(&mut stand, 2.4, 3100.0));
    }
}
