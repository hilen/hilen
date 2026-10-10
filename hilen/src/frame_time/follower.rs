//! Evens out the times at which frames begin. It knows no window and no
//! clock, a caller hands it the real time of each frame.
//!
//! A screen shows frames at even ticks, 1 refresh period apart. The code
//! of a frame starts a few milliseconds early or late each time. The
//! follower measures the period from the frames themselves, goes on by
//! whole periods and takes only a small share of the difference to the
//! real time.
//!
//! The period is measured and not read from the system. The system value
//! is missing in a browser and on some Linux setups, and it is not what
//! happens when an app draws every second refresh or a display changes
//! its rate by itself. A measured period is right in all of these.

/// The share of the difference to the real time that 1 frame takes.
const SHARE: f64 = 0.1;

/// The share the newest measured period has in the period.
const PERIOD_SHARE: f64 = 0.02;

/// A gap of this many seconds or more is a hold of the loop, not a slow
/// frame: a covered window, an idle loop, a sleeping app, a paused one.
const HOLD: f64 = 0.1;

/// Frames in a row that must come in an even rhythm to give a period.
const RUN: usize = 16;

/// How far a frame of a run may be from its even place, as a part of the
/// period of the run. Measured gaps on a 60 Hz display went from 9 to 24
/// ms, so a frame began up to 0.23 of a period early or late.
const RUN_SPREAD: f64 = 0.3;

/// A measured period agrees with the period when it is the period, or a
/// whole number of periods, and not farther off than this part of the
/// period. The period of a run is up to 0.03 off under the jitter above.
const PERIOD_OFF: f64 = 0.1;

/// Runs in a row that must give a longer period that does not agree,
/// before it replaces the period. Frames on demand come a whole number of
/// periods apart, and now and then 16 of them look like a slow rhythm.
const DOUBT: u32 = 30;

/// A frame fits the rhythm when it begins no farther from its even place
/// than this part of the period, and no farther than `FIT_MAX` seconds. A
/// frame that does not fit gets the real time. So the frame time is never
/// more than `FIT_MAX` from the real time.
const FIT: f64 = 0.4;
const FIT_MAX: f64 = 0.008;

#[derive(Default)]
pub(super) struct FrameFollower {
    /// The real time of the frame before.
    last:   Option<f64>,
    time:   f64,
    /// None until a run of even frames was seen.
    period: Option<f64>,
    /// The real times of the newest frames with no hold between them.
    run:    Vec<f64>,
    /// Runs in a row that gave a longer period that does not agree.
    doubt:  u32,
}

impl FrameFollower {
    /// The frame time of a frame that begins at the real time `now`. It
    /// never goes back.
    pub(super) fn advance(&mut self, now: f64) -> f64 {
        let Some(last) = self.last else {
            self.last = Some(now);
            self.time = now;
            self.run.push(now);
            return now;
        };

        let gap = now - last;
        // A timer of a browser can give 2 frames the same time.
        if gap.is_nan() || gap <= 0.0 {
            return self.time;
        }
        self.last = Some(now);

        if gap >= HOLD {
            self.run.clear();
            self.run.push(now);
            self.time = self.time.max(now);
            return self.time;
        }

        match (self.period, self.measure(now)) {
            (None, None) => self.time = self.time.max(now),
            (None, Some(measured)) => self.take_period(measured, now),
            (Some(period), None) => {
                self.doubt = 0;
                self.follow(now, period);
            }
            (Some(period), Some(measured)) => self.compare(now, period, measured),
        }

        self.time
    }

    /// A known period meets the period of the newest run.
    fn compare(&mut self, now: f64, period: f64, measured: f64) {
        let ratio = measured / period;
        // An app that draws every second refresh has a run of 2 periods.
        let whole = ratio.round().max(1.0);

        if (ratio - whole).abs() <= PERIOD_OFF {
            let period = period + PERIOD_SHARE * (measured / whole - period);
            self.period = Some(period);
            self.doubt = 0;
            self.follow(now, period);
            return;
        }

        // Frames cannot come in an even rhythm faster than the screen
        // shows them, so a shorter period is the truth at once.
        if ratio < 1.0 {
            self.take_period(measured, now);
            return;
        }

        self.doubt += 1;
        if self.doubt >= DOUBT {
            self.take_period(measured, now);
        } else {
            self.follow(now, period);
        }
    }

    #[cfg(test)]
    pub(super) fn period(&self) -> Option<f64> {
        self.period
    }

    /// The period of the newest frames, when there are enough of them and
    /// they come in an even rhythm. Only such a run teaches a period. A
    /// loop that draws now and then, like one that follows the mouse, has
    /// none.
    ///
    /// The period is the slope of the straight line that fits the frame
    /// times best. The frames have a rhythm when each of them is near that
    /// line. The jitter of the frames does not add up along a run, the
    /// gaps of a loop with no rhythm do.
    fn measure(&mut self, now: f64) -> Option<f64> {
        if self.run.len() == RUN {
            self.run.remove(0);
        }
        self.run.push(now);
        if self.run.len() < RUN {
            return None;
        }

        // Counted from the first frame, a sum of raw times would lose the
        // low bits after hours.
        let first = self.run[0];
        let mut sum_time = 0.0;
        let mut sum_index = 0.0;
        let mut sum_index_time = 0.0;
        let mut sum_index_index = 0.0;
        let mut index = 0.0;
        for time in &self.run {
            sum_time += time - first;
            sum_index += index;
            sum_index_time += index * (time - first);
            sum_index_index += index * index;
            index += 1.0;
        }
        let count = index;
        let period = (count * sum_index_time - sum_index * sum_time)
            / (count * sum_index_index - sum_index * sum_index);
        if period.is_nan() || period <= 0.0 {
            return None;
        }
        let start = (sum_time - period * sum_index) / count;

        let mut place = start;
        let even = self.run.iter().all(|time| {
            let off = (time - first - place) / period;
            place += period;
            off.abs() <= RUN_SPREAD
        });

        even.then_some(period)
    }

    fn take_period(&mut self, period: f64, now: f64) {
        self.period = Some(period);
        self.doubt = 0;
        self.time = self.time.max(now);
    }

    fn follow(&mut self, now: f64, period: f64) {
        // A dropped frame spans 2 periods or more.
        let periods = ((now - self.time) / period).round().max(1.0);
        let even = self.time + periods * period;
        let off = now - even;

        self.time = if off.abs() > (FIT * period).min(FIT_MAX) {
            self.time.max(now)
        } else {
            (even + SHARE * off).max(self.time)
        };
    }
}
