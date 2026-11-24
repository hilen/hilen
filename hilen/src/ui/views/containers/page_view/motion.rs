//! How the pages of a `PageView` move, measured on a real
//! `UIPageViewController` in the iOS 26.3 simulator on 2026-11-24 with the
//! touches of `make sim`, see `uikit-reference/` and `docs/page-view.md`.

use std::collections::VecDeque;

use crate::gm::LossyConvert;

/// The stiffness of the spring that takes the pages into place after a
/// release, in radians per second. 4 releases fitted 16.25 to 17.
const STIFFNESS: f64 = 16.5;
/// The damping of that spring, below 1 it overshoots a little. Every fit
/// gave 0.9.
const DAMPING: f64 = 0.9;
/// The spring is done when the pages are this close to their place, then
/// they stand on it. `UIKit` ended 0.33 points before the place.
const DONE: f64 = 0.5;
/// The longest a glide can take, a guard for a broken width.
const LONGEST: f64 = 2.0;
/// The rubber band of a scroll view of iOS, fitted with 0.55 rms 0.12.
const RUBBER: f32 = 0.55;
/// A release at least this fast turns the page whatever the distance. The
/// flicks that turned measured 386 points per second and more, the ones that
/// did not 240 and less.
pub(crate) const FLING: f32 = 300.0;
/// The finger speed is told from the moves of this last part of a second.
const WINDOW: f64 = 0.05;

/// How far a page goes for a finger `past` points beyond the last page.
/// It never goes as far as `width`.
pub(crate) fn rubber(past: f32, width: f32) -> f32 {
    if width <= 0.0 {
        return 0.0;
    }
    (1.0 - 1.0 / (past * RUBBER / width + 1.0)) * width
}

/// The pages from `from` to `to`. `UIKit` starts it from rest also after a
/// flick, the speed of the finger only picks the page.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Spring {
    from: f32,
    to:   f32,
}

impl Spring {
    pub(crate) fn new(from: f32, to: f32) -> Self {
        Self { from, to }
    }

    /// Where the pages are `seconds` after the start.
    pub(crate) fn at(self, seconds: f64) -> f32 {
        let rest = f64::from(self.from - self.to);
        let damped = STIFFNESS * (1.0 - DAMPING * DAMPING).sqrt();
        let fade = (-DAMPING * STIFFNESS * seconds).exp();
        let left = fade
            * rest
            * ((damped * seconds).cos() + DAMPING * STIFFNESS / damped * (damped * seconds).sin());
        let left: f32 = left.lossy_convert();
        self.to + left
    }

    /// The seconds until the pages first stand within half a point of their
    /// place.
    pub(crate) fn seconds(self) -> f64 {
        let mut seconds = 0.0;
        while seconds < LONGEST {
            if f64::from((self.at(seconds) - self.to).abs()) < DONE {
                return seconds;
            }
            seconds += 0.001;
        }
        LONGEST
    }
}

/// The speed of a finger from its last moves, in points per second.
#[derive(Debug, Default)]
pub(crate) struct Speed {
    moves: VecDeque<(f64, f32)>,
}

impl Speed {
    pub(crate) fn clear(&mut self) {
        self.moves.clear();
    }

    pub(crate) fn add(&mut self, seconds: f64, x: f32) {
        self.moves.push_back((seconds, x));
        while self.moves.front().is_some_and(|(at, _)| seconds - at > WINDOW * 4.0) {
            self.moves.pop_front();
        }
    }

    /// The speed over the moves of the last 50 ms before the last move. A
    /// finger that rested longer than that before `released` has none.
    pub(crate) fn at_release(&self, released: f64) -> f32 {
        let Some(&(last, x)) = self.moves.back() else {
            return 0.0;
        };
        if released - last > WINDOW {
            return 0.0;
        }
        let Some(&(first, first_x)) = self.moves.iter().find(|(at, _)| last - at <= WINDOW + 0.0001) else {
            return 0.0;
        };
        if last - first <= 0.0 {
            return 0.0;
        }
        let seconds: f32 = (last - first).lossy_convert();
        (x - first_x) / seconds
    }
}

/// Which page a release goes to: -1 the page before, 0 the open one, 1 the
/// page after. `shift` is how far right of its place the open page is,
/// `speed` how fast the finger went right.
pub(crate) fn target(shift: f32, speed: f32, width: f32, before: bool, after: bool) -> i8 {
    let page = if speed >= FLING {
        // Right, toward the page before.
        -i8::from(shift > 0.0)
    } else if speed <= -FLING {
        i8::from(shift < 0.0)
    } else {
        // At rest the page past half the width on either side.
        i8::from(shift < -width / 2.0) - i8::from(shift > width / 2.0)
    };
    match page {
        -1 if before => -1,
        1 if after => 1,
        _ => 0,
    }
}

#[cfg(test)]
mod test {
    use super::{FLING, Speed, Spring, rubber, target};

    const WIDTH: f32 = 402.0;

    #[test]
    fn the_spring_matches_a_measured_release() {
        // A slow drag released 140 points forward of the open page, at rest,
        // went back. Its offsets 0.106 and 0.206 s after the release, minus
        // the 0.022 s the glide started later.
        let spring = Spring::new(-140.0, 0.0);
        assert!((spring.at(0.084) - -81.67).abs() < 2.0, "{}", spring.at(0.084));
        assert!((spring.at(0.184) - -21.33).abs() < 2.0, "{}", spring.at(0.184));
        let seconds = spring.seconds();
        assert!((0.3..0.4).contains(&seconds), "{seconds}");
    }

    #[test]
    fn rubber_band_matches_the_measured_pull() {
        // A finger 189.5 points past the first page moved it 82.67.
        assert!(
            (rubber(189.5, WIDTH) - 82.67).abs() < 1.0,
            "{}",
            rubber(189.5, WIDTH)
        );
        assert!(rubber(100_000.0, WIDTH) < WIDTH);
        assert!(rubber(0.0, WIDTH).abs() < f32::EPSILON);
    }

    #[test]
    fn speed_is_taken_from_the_last_moves() {
        // The flick of 40 points that turned the page: steps of 1/60 s.
        let mut speed = Speed::default();
        for (at, x) in [
            (0.0833, 294.0),
            (0.0998, 287.0),
            (0.1165, 279.33),
            (0.1333, 271.67),
            (0.15, 265.0),
            (0.1666, 260.0),
        ] {
            speed.add(at, x);
        }
        let flick = speed.at_release(0.1833);
        assert!(flick <= -FLING, "{flick}");
        // The same finger resting 0.3 s before it lifts has no speed.
        assert!(speed.at_release(0.4666).abs() < f32::EPSILON);
    }

    #[test]
    fn release_picks_the_page_like_uikit() {
        // At rest: past half the width turns, below it goes back.
        assert_eq!(target(-240.0, 0.0, WIDTH, true, true), 1);
        assert_eq!(target(-140.0, 0.0, WIDTH, true, true), 0);
        // A fling turns from a short distance.
        assert_eq!(target(-30.0, -386.0, WIDTH, true, true), 1);
        assert_eq!(target(-30.0, -240.0, WIDTH, true, true), 0);
        // A fling back from past half goes back.
        assert_eq!(target(-300.0, 400.0, WIDTH, true, true), 0);
        // No page there.
        assert_eq!(target(-300.0, -900.0, WIDTH, true, false), 0);
        assert_eq!(target(250.0, 0.0, WIDTH, false, true), 0);
    }
}
