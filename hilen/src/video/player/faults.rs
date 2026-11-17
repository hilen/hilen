//! What goes wrong while a video plays, written to the log. A sound that
//! dies and pictures that come late show nothing on screen but silence and
//! a stutter, and the log file is the only place to see why. A sound that
//! died is also opened again from here.

use kira::sound::PlaybackState;
use log::{error, warn};

use crate::{gm::Clock, video::player::Player};

/// Dropped pictures are counted over this long and logged as 1 line, a
/// line per picture would fill the file.
const DROP_WINDOW_MS: f64 = 5000.0;

/// A sound that stops this far before the end did not just play out.
const EARLY_STOP: f64 = 1.0;

/// A sound that died is opened again no more often than this. A source
/// that fails every time would open in a loop with no pause.
const REVIVE_EVERY_MS: f64 = 2000.0;

/// What the log already said, so each fault is written once.
#[derive(Default)]
pub(super) struct Faults {
    /// The sound was stopped at the last look.
    sound_stopped: bool,
    /// `Clock` milliseconds when a dead sound was last opened again.
    revived_ms:    Option<f64>,
    drops:         DropWindow,
}

/// True when a dead sound may be opened again now.
fn revive_due(revived_ms: Option<f64>, now_ms: f64) -> bool {
    revived_ms.is_none_or(|then| now_ms - then >= REVIVE_EVERY_MS)
}

/// The dropped pictures of 1 window of time.
#[derive(Default)]
struct DropWindow {
    /// When the window opened and the dropped count then.
    opened: Option<(f64, u64)>,
}

impl DropWindow {
    /// How many pictures the window lost, once it is over and lost any.
    fn closed(&mut self, dropped: u64, now_ms: f64) -> Option<u64> {
        let Some((since_ms, count_then)) = self.opened else {
            self.opened = Some((now_ms, dropped));
            return None;
        };
        if now_ms - since_ms < DROP_WINDOW_MS {
            return None;
        }
        self.opened = Some((now_ms, dropped));
        let lost = dropped.saturating_sub(count_then);
        (lost > 0).then_some(lost)
    }
}

impl Player {
    /// A sound that died got a fresh decoder at least once.
    #[cfg(test)]
    pub(super) fn sound_opened_again(&self) -> bool {
        self.faults.revived_ms.is_some()
    }

    pub(super) fn watch_faults(&mut self) {
        self.mend_sound();
        self.log_dropped();
    }

    /// kira keeps the error of a decoder to itself and stops the sound for
    /// good. The picture then plays on by the engine clock, and a pause and
    /// a play do not bring the sound back, only a fresh decoder does.
    fn mend_sound(&mut self) {
        let position = self.position();
        let duration = self.duration();
        let Some(sound) = &mut self.sound else {
            self.faults.sound_stopped = false;
            return;
        };
        // kira calls a decoder that failed again with no pause until it
        // stops the sound. A line per error here gives kira the time to
        // fail again, a cut stream wrote 70 of them.
        let mut failed = None;
        while let Some(err) = sound.pop_error() {
            failed = Some(err);
        }
        let erred = failed.is_some();
        if let Some(err) = failed {
            error!(
                "video {}: the sound decoder failed at {position:.1} s, {err}",
                self.source.location()
            );
        }
        let stopped = sound.state() == PlaybackState::Stopped;
        // A decoder that failed is replaced at once. kira plays what it
        // has decoded and stops only then, and until it stops the sound is
        // still the clock, so the place the fresh decoder starts at is
        // exact.
        let died = (stopped || erred) && position + EARLY_STOP < duration;
        if died && stopped && !self.faults.sound_stopped {
            warn!(
                "video {}: the sound stopped at {position:.1} of {duration:.1} s, the picture plays on without it",
                self.source.location()
            );
        }
        self.faults.sound_stopped = stopped;

        let now_ms = Clock::now_ms();
        if died && revive_due(self.faults.revived_ms, now_ms) {
            warn!(
                "video {}: the sound opens again at {position:.1} s",
                self.source.location()
            );
            self.faults.revived_ms = Some(now_ms);
            self.reopen_sound();
        }
    }

    fn log_dropped(&mut self) {
        let Some(lost) = self.faults.drops.closed(self.counters.dropped, Clock::now_ms()) else {
            return;
        };
        warn!(
            "video {}: {lost} late pictures dropped in {:.0} s, at {:.1} s, {} dropped of {} shown so far",
            self.source.location(),
            DROP_WINDOW_MS / 1000.0,
            self.position(),
            self.counters.dropped,
            self.counters.presented
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{DROP_WINDOW_MS, DropWindow, REVIVE_EVERY_MS, revive_due};

    /// A sound that died plays again only from a fresh decoder. A source
    /// that fails every time must not open one on every frame.
    #[test]
    fn a_dead_sound_opens_again_at_once_and_then_with_a_pause() {
        assert!(revive_due(None, 100.0));
        assert!(!revive_due(Some(100.0), 100.0 + REVIVE_EVERY_MS - 1.0));
        assert!(revive_due(Some(100.0), 100.0 + REVIVE_EVERY_MS));
    }

    #[test]
    fn a_window_reports_what_it_lost_once_it_is_over() {
        let mut window = DropWindow::default();

        assert_eq!(window.closed(10, 0.0), None);
        assert_eq!(window.closed(14, DROP_WINDOW_MS - 1.0), None);
        assert_eq!(window.closed(17, DROP_WINDOW_MS), Some(7));
        // The next window counts from the end of this one.
        assert_eq!(window.closed(18, DROP_WINDOW_MS * 2.0), Some(1));
    }

    #[test]
    fn a_window_that_lost_nothing_says_nothing() {
        let mut window = DropWindow::default();

        assert_eq!(window.closed(3, 0.0), None);
        assert_eq!(window.closed(3, DROP_WINDOW_MS), None);
        assert_eq!(window.closed(3, DROP_WINDOW_MS * 2.0), None);
    }
}
