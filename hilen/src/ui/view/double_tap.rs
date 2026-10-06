use crate::{
    deps::hreads::after,
    gm::{Clock, LossyConvert, STEP_MS, flat::Point},
    ui::{Touch, ViewData, WeakView},
};

/// 2 taps this close in time are a double tap. A single tap is known only
/// after this long, so a longer wait makes a click feel late.
const INTERVAL_MS: f64 = 250.0;

/// And this close in place. A finger lands less exactly than a pointer.
#[cfg(mobile)]
const DISTANCE: f32 = 40.0;
#[cfg(not(mobile))]
const DISTANCE: f32 = 6.0;

/// A tap that may still get a second one.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FirstTap {
    /// In milliseconds of `Clock`.
    at:       f64,
    position: Point,
    number:   u64,
}

/// Tells a double tap from 2 single taps, for a view and for the word
/// selection of a text field.
#[derive(Default, Debug)]
pub(crate) struct DoubleTap {
    last:    Option<FirstTap>,
    /// Counts the taps, so each one has its own number.
    count:   u64,
    /// The numbers of the taps that wait to become a single tap. A second
    /// tap takes the number of its first tap out.
    waiting: Vec<u64>,
}

impl DoubleTap {
    fn follows(&self, now: f64, position: Point) -> bool {
        self.last
            .is_some_and(|last| now - last.at < INTERVAL_MS && (last.position - position).length() < DISTANCE)
    }

    fn first(&mut self, now: f64, position: Point) -> FirstTap {
        self.count += 1;
        let first = FirstTap {
            at: now,
            position,
            number: self.count,
        };
        self.last = Some(first);
        first
    }

    /// Takes a tap. None when it is the second of a pair, a double tap.
    /// The tap after a pair starts a new one, so 3 fast taps are 1 double
    /// tap, not 2.
    pub(crate) fn tap(&mut self, position: Point) -> Option<FirstTap> {
        let now = Clock::now_ms();
        if self.follows(now, position) {
            if let Some(last) = self.last.take() {
                self.waiting.retain(|number| *number != last.number);
            }
            return None;
        }
        Some(self.first(now, position))
    }

    /// Takes a tap, true when it comes soon after the one before, so every
    /// tap of a fast row but the first is true.
    pub(crate) fn tap_in_row(&mut self, position: Point) -> bool {
        let now = Clock::now_ms();
        let double = self.follows(now, position);
        self.first(now, position);
        double
    }
}

/// Fires `single_tap` on `view` once no second tap can follow `first`.
pub(crate) fn wait_for_single_tap(view: WeakView, first: FirstTap, touch: Touch) {
    view.__base_view().taps.waiting.push(first.number);
    look_for_single_tap(view, first, touch);
}

fn look_for_single_tap(view: WeakView, first: FirstTap, touch: Touch) {
    // A stepped clock stands until a test moves it, so look every frame.
    let left = if Clock::is_stepped() {
        STEP_MS
    } else {
        (first.at + INTERVAL_MS - Clock::now_ms()).max(STEP_MS)
    };
    let wait: f32 = (left / 1000.0).lossy_convert();

    after(wait, move || {
        if view.is_null() {
            return;
        }
        let waiting = &mut view.__base_view().taps.waiting;
        if !waiting.contains(&first.number) {
            return;
        }
        if Clock::now_ms() - first.at < INTERVAL_MS {
            look_for_single_tap(view, first, touch);
            return;
        }
        waiting.retain(|number| *number != first.number);
        if !view.is_hidden_in_tree() {
            view.__base_view().events.touch.single_tap.trigger(touch);
        }
    });
}
