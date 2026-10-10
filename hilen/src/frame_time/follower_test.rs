//! The follower is fed made up frame times, so no window and no real clock
//! is part of these tests.

use super::follower::FrameFollower;

const HZ_60: f64 = 1.0 / 60.0;
const HZ_120: f64 = 1.0 / 120.0;
const HZ_144: f64 = 1.0 / 144.0;
const HZ_50: f64 = 1.0 / 50.0;

const MS: f64 = 0.001;

/// Numbers from -1 to 1 that are the same in every run.
struct Noise(u64);

impl Noise {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let top = u32::try_from(self.0 >> 40).expect("24 bits fit");
        f64::from(top) / f64::from(1_u32 << 23) - 1.0
    }
}

/// A display and a loop that begins each frame a bit early or late.
struct Screen {
    follower: FrameFollower,
    noise:    Noise,
    /// The even time of the newest frame.
    tick:     f64,
    /// The frame time of the newest frame.
    time:     f64,
    /// The real time of the newest frame.
    now:      f64,
}

impl Screen {
    fn new() -> Self {
        let mut follower = FrameFollower::default();
        let time = follower.advance(5.0);
        Self {
            follower,
            noise: Noise(7),
            tick: 5.0,
            time,
            now: 5.0,
        }
    }

    /// The next frame, `gap` after the even time of the one before and up
    /// to `jitter` early or late. Gives the step of the frame time.
    fn frame(&mut self, gap: f64, jitter: f64) -> f64 {
        self.tick += gap;
        self.now = self.tick + self.noise.next() * jitter;
        let before = self.time;
        self.time = self.follower.advance(self.now);
        assert!(self.time >= before, "the frame time went back");
        self.time - before
    }

    fn frames(&mut self, count: u32, gap: f64, jitter: f64) -> Vec<f64> {
        (0..count).map(|_| self.frame(gap, jitter)).collect()
    }

    /// How far the frame time is from the even time of the frame.
    fn off(&self) -> f64 {
        (self.time - self.tick).abs()
    }
}

fn farthest(steps: &[f64], period: f64) -> f64 {
    steps.iter().map(|step| (step - period).abs()).fold(0.0, f64::max)
}

#[test]
fn even_frames_give_even_steps() {
    let mut screen = Screen::new();
    let steps = screen.frames(600, HZ_60, 0.0);
    assert!(
        farthest(&steps, HZ_60) < 1e-9,
        "a step was {} s off",
        farthest(&steps, HZ_60)
    );
    assert!(screen.off() < 1e-9);
}

#[test]
fn first_frame_is_the_real_time() {
    let mut follower = FrameFollower::default();
    assert!((follower.advance(123.456) - 123.456).abs() < f64::EPSILON);
}

/// The measured case: a 60 Hz display, each frame begins up to 4 ms early
/// or late, so the gap between 2 frames goes from 8.7 to 24.7 ms.
#[test]
fn jitter_at_60_hz_is_evened_out() {
    let mut screen = Screen::new();
    screen.frames(120, HZ_60, 4.0 * MS);

    let steps = screen.frames(6000, HZ_60, 4.0 * MS);
    let farthest = farthest(&steps, HZ_60);
    println!(
        "60 Hz, 4 ms jitter: a step is at most {:.3} ms off",
        farthest / MS
    );
    assert!(farthest < 1.0 * MS, "a step was {} ms off", farthest / MS);
}

#[test]
fn jitter_at_120_hz_is_evened_out() {
    let mut screen = Screen::new();
    screen.frames(240, HZ_120, 2.0 * MS);

    let steps = screen.frames(6000, HZ_120, 2.0 * MS);
    let farthest = farthest(&steps, HZ_120);
    println!(
        "120 Hz, 2 ms jitter: a step is at most {:.3} ms off",
        farthest / MS
    );
    assert!(farthest < 0.5 * MS, "a step was {} ms off", farthest / MS);
}

#[test]
fn no_drift_over_many_frames() {
    let mut screen = Screen::new();
    let mut farthest: f64 = 0.0;
    for _ in 0..200_000 {
        screen.frame(HZ_60, 4.0 * MS);
        farthest = farthest.max(screen.off());
    }
    println!("200000 frames: the time is at most {:.3} ms off", farthest / MS);
    assert!(farthest < 5.0 * MS, "the time was {} ms off", farthest / MS);
    assert!(screen.off() < 5.0 * MS);
}

#[test]
fn dropped_frame_moves_by_the_whole_gap() {
    for dropped in [2.0, 3.0] {
        let mut screen = Screen::new();
        screen.frames(300, HZ_60, 2.0 * MS);

        let step = screen.frame(HZ_60 * dropped, 2.0 * MS);
        assert!(
            (step - HZ_60 * dropped).abs() < 0.5 * MS,
            "a gap of {dropped} periods moved the time by {} ms",
            step / MS
        );

        let steps = screen.frames(300, HZ_60, 2.0 * MS);
        assert!(farthest(&steps, HZ_60) < 0.5 * MS);
    }
}

#[test]
fn hold_lands_on_the_real_time() {
    for hold in [0.1, 0.5, 7.0, 3600.0] {
        let mut screen = Screen::new();
        screen.frames(300, HZ_60, 2.0 * MS);

        screen.frame(hold, 2.0 * MS);
        assert!(
            (screen.time - screen.now).abs() < f64::EPSILON,
            "after a hold of {hold} s the time was {} ms off",
            (screen.time - screen.now) / MS
        );

        // The period is kept over a hold, the steps are even at once.
        let steps = screen.frames(300, HZ_60, 2.0 * MS);
        assert!(farthest(&steps, HZ_60) < 0.5 * MS);
    }
}

/// A window that moves to a display of another rate.
#[test]
fn change_of_period_is_followed() {
    let rates = [
        (HZ_60, HZ_120),
        (HZ_120, HZ_60),
        (HZ_60, HZ_144),
        (HZ_144, HZ_60),
        (HZ_60, HZ_50),
        (HZ_50, HZ_60),
    ];
    for (from, to) in rates {
        let mut screen = Screen::new();
        screen.frames(300, from, 0.0);

        let mut farthest_off: f64 = 0.0;
        for _ in 0..120 {
            screen.frame(to, 0.0);
            farthest_off = farthest_off.max(screen.off());
        }
        println!(
            "{:.0} to {:.0} Hz: the time was at most {:.2} ms off on the way",
            1.0 / from,
            1.0 / to,
            farthest_off / MS
        );
        assert!(
            farthest_off < 1.5 * from.max(to),
            "{from} to {to}: the time was {} ms off",
            farthest_off / MS
        );

        let steps = screen.frames(300, to, 0.0);
        assert!(
            farthest(&steps, to) < 0.1 * MS,
            "{from} to {to}: a step was {} ms off",
            farthest(&steps, to) / MS
        );
        // A slower screen whose period is 2 old periods keeps the old
        // one, every frame then moves the time by 2 of them.
        let period = screen.follower.period().expect("a period after even frames");
        let periods = to / period;
        assert!(
            (periods - periods.round()).abs() < 0.01,
            "{from} to {to}: the period is {period}"
        );
    }
}

/// An app that draws only every second refresh.
#[test]
fn every_second_refresh_is_even() {
    let mut screen = Screen::new();
    screen.frames(300, HZ_60, 1.0 * MS);
    screen.frames(120, HZ_60 * 2.0, 1.0 * MS);

    let steps = screen.frames(600, HZ_60 * 2.0, 1.0 * MS);
    assert!(farthest(&steps, HZ_60 * 2.0) < 0.5 * MS);
}

/// A loop that draws now and then, like one that only follows the mouse,
/// has no period. Its frames get the real time.
#[test]
fn frames_with_no_rhythm_get_the_real_time() {
    let mut screen = Screen::new();
    let mut noise = Noise(99);
    for _ in 0..2000 {
        let gap = (50.0 + noise.next() * 45.0) * MS;
        screen.frame(gap, 0.0);
        assert!((screen.time - screen.now).abs() < f64::EPSILON);
    }
    assert!(screen.follower.period().is_none());
}

/// After frames with no rhythm, a run of even frames is followed.
#[test]
fn rhythm_after_no_rhythm_is_found() {
    let mut screen = Screen::new();
    let mut noise = Noise(99);
    for _ in 0..200 {
        let gap = (50.0 + noise.next() * 45.0) * MS;
        screen.frame(gap, 0.0);
    }
    screen.frames(60, HZ_60, 3.0 * MS);
    let steps = screen.frames(600, HZ_60, 3.0 * MS);
    assert!(farthest(&steps, HZ_60) < 1.0 * MS);
}

/// A known period is kept while the loop draws now and then, so the first
/// frames of the next animation are even.
#[test]
fn period_is_kept_over_frames_with_no_rhythm() {
    let mut screen = Screen::new();
    screen.frames(300, HZ_60, 2.0 * MS);
    let mut noise = Noise(99);
    for _ in 0..500 {
        // Whole refresh periods, a frame on demand is still shown on a
        // refresh.
        let periods = (3.0 + noise.next() * 2.0).round();
        screen.frame(HZ_60 * periods, 2.0 * MS);
        assert!(
            screen.off() < 2.5 * MS,
            "the time was {} ms off",
            screen.off() / MS
        );
    }
    let period = screen.follower.period().expect("the period is kept");
    assert!((period / HZ_60 - 1.0).abs() < 0.01, "the period is {period}");
    let steps = screen.frames(300, HZ_60, 2.0 * MS);
    assert!(farthest(&steps, HZ_60) < 0.5 * MS);
}

/// However the frames come, the frame time stays within 8 ms of the real
/// time of the frame.
#[test]
fn never_far_from_the_real_time() {
    let mut screen = Screen::new();
    let mut noise = Noise(3);
    for round in 0..200 {
        let gap = [HZ_60, HZ_120, HZ_144, HZ_50, HZ_60 * 2.0][round % 5];
        for _ in 0..100 {
            let gap = if noise.next() > 0.8 { gap * 3.0 } else { gap };
            // Under half of the shortest gap, so the real time never goes
            // back.
            screen.frame(gap, 3.0 * MS);
            let off = (screen.time - screen.now).abs();
            assert!(off <= 8.0 * MS, "the time was {} ms from the real time", off / MS);
        }
        for _ in 0..20 {
            let gap = (40.0 + noise.next() * 39.0) * MS;
            screen.frame(gap, 0.0);
            let off = (screen.time - screen.now).abs();
            assert!(off <= 8.0 * MS, "the time was {} ms from the real time", off / MS);
        }
    }
}

#[test]
fn same_time_twice_does_not_move() {
    let mut follower = FrameFollower::default();
    let first = follower.advance(1.0);
    assert!((follower.advance(1.0) - first).abs() < f64::EPSILON);
    assert!(follower.advance(0.5) >= first);
}
