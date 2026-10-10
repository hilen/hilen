use std::hint::spin_loop;

use anyhow::{Result, ensure};

use crate::{
    self as hilen,
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    gm::{
        Clock, LossyConvert, STEP_MS,
        color::{LIGHT_BLUE, ORANGE},
    },
    time::{frame_seconds, monotonic_seconds},
    ui::{Label, Setup, ViewCallbacks, ViewData, ViewFrame, ViewTest, view},
    ui_test::{checkpoint, step_frames},
};

/// Points a box moves in 1 second of frame time.
const SPEED: f64 = 300.0;

/// Where a box starts, and how far it goes before it starts again.
const LEFT: f64 = 20.0;
const TRACK: f64 = 400.0;

/// The frames the real time part looks at.
const REAL_FRAMES: usize = 30;

/// The frames the stepped part draws.
const STEPPED_FRAMES: u32 = 30;

/// How long the second box works before it reads the time, in seconds.
const WORK: f64 = 0.002;

/// A turn of the loop gives up after this many frames.
const GIVE_UP_AFTER: u32 = 100_000;

#[derive(Clone, Copy)]
struct Reading {
    frame: f64,
    real:  f64,
}

/// A box that stands where the frame time says.
#[view]
struct TimedBox {
    /// The frame time the box counts from, None while it stands.
    from:  Option<f64>,
    /// Works for a while before it reads the time, so it reads it late in
    /// the frame.
    works: bool,
    /// Keeps what it read in each frame.
    keeps: bool,
    seen:  Vec<Reading>,

    #[init]
    role: Label,
}

impl Setup for TimedBox {
    fn setup(self: Weak<Self>) {
        self.role.set_text_size(14).place().back();
    }
}

impl ViewCallbacks for TimedBox {
    fn update(&mut self) {
        let Some(from) = self.from else {
            return;
        };

        if self.works {
            let until = monotonic_seconds() + WORK;
            while monotonic_seconds() < until {
                spin_loop();
            }
        }

        let frame = frame_seconds();
        if self.keeps {
            self.seen.push(Reading {
                frame,
                real: monotonic_seconds(),
            });
        }

        let x: f32 = (LEFT + (frame - from) * SPEED % TRACK).lossy_convert();
        self.set_x(x);
    }
}

/// 2 boxes that move by `time::frame_seconds`.
///
/// In real time the test proves what does not depend on a screen: both
/// boxes read the same value in a frame, also the one that reads it 2 ms
/// later, the value never goes back and it stays at the real time. A
/// headless run has no refresh, so even steps in real time are proven by
/// the unit tests of the follower, not here.
///
/// In frame stepped time a frame moves the time by exactly 1 frame of 60
/// a second, so the boxes take the same step in every frame.
#[view]
struct EvenFrameTime {
    #[init]
    title:  Label,
    state:  Label,
    first:  TimedBox,
    second: TimedBox,
}

impl Setup for EvenFrameTime {
    fn setup(mut self: Weak<Self>) {
        self.title.set_text("2 boxes stand where the frame time says").set_text_size(20);
        self.title.place().t(20).lr(10).h(40);

        self.state.set_text("the boxes stand").set_text_size(20);
        self.state.place().t(70).lr(10).h(40);

        self.first.set_color(LIGHT_BLUE).set_frame((20, 140, 120, 40));
        self.first.role.set_text("reads first");

        self.second.set_color(ORANGE).set_frame((20, 200, 120, 40));
        self.second.role.set_text("reads 2 ms later");
        self.second.works = true;
    }
}

impl EvenFrameTime {
    fn start(self: Weak<Self>, keeps: bool) {
        let from = frame_seconds();
        for mut timed in [self.first, self.second] {
            timed.from = Some(from);
            timed.keeps = keeps;
            timed.seen.clear();
        }
        let first = self.first;
        self.first.keep_frames_while(move || first.from.is_some());
    }

    fn stop(mut self: Weak<Self>) {
        self.first.from = None;
        self.second.from = None;
    }
}

fn check_real_time(first: &[Reading], second: &[Reading]) -> Result<()> {
    ensure!(
        first.len() == second.len(),
        "the boxes were updated in different frames, {} and {}",
        first.len(),
        second.len()
    );

    for (index, (first, second)) in first.iter().zip(second).enumerate() {
        ensure!(
            first.frame.to_bits() == second.frame.to_bits(),
            "frame {index}: the boxes read {} and {}",
            first.frame,
            second.frame
        );
        ensure!(
            second.real - first.real >= WORK,
            "frame {index}: the second box must read {WORK} s later, it read {} s later",
            second.real - first.real
        );
        let behind = first.real - first.frame;
        ensure!(
            (-0.05..0.1).contains(&behind),
            "frame {index}: the frame time is {behind} s behind the real time"
        );
    }

    for (index, pair) in first.windows(2).enumerate() {
        ensure!(
            pair[1].frame >= pair[0].frame,
            "frame {index}: the frame time went back from {} to {}",
            pair[0].frame,
            pair[1].frame
        );
    }

    let passed = first[first.len() - 1].frame - first[0].frame;
    ensure!(
        passed > 0.0,
        "the frame time did not move in {} frames",
        first.len()
    );

    Ok(())
}

impl ViewTest for EvenFrameTime {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(move || {
            view.start(true);
            view.state.set_text("real time, both boxes read the same time");
        });

        let mut frames = 0;
        while from_main(move || view.second.seen.len()) < REAL_FRAMES {
            wait_for_next_frame();
            frames += 1;
            ensure!(frames < GIVE_UP_AFTER, "the boxes got no {REAL_FRAMES} updates");
        }

        let (first, second) = from_main(move || {
            view.stop();
            (view.first.seen.clone(), view.second.seen.clone())
        });
        check_real_time(&first, &second)?;
        checkpoint("real time, the boxes moved and stand at the same place")?;

        let from = from_main(move || {
            Clock::enter_stepped();
            view.start(false);
            view.state.set_text("frame stepped time, 5 points in every frame");
            frame_seconds()
        });

        let step = STEP_MS / 1000.0;
        let mut time_before = from;
        let mut x_before: f64 = LEFT;
        for frame in 1..=STEPPED_FRAMES {
            step_frames(1);

            let (time, first, second) = from_main(move || (frame_seconds(), view.first.x(), view.second.x()));

            ensure!(
                (time - time_before - step).abs() < 1e-9,
                "stepped frame {frame} moved the time by {} s, not by {step}",
                time - time_before
            );
            ensure!(
                first.to_bits() == second.to_bits(),
                "stepped frame {frame}: the boxes stand at {first} and {second}"
            );
            let moved = f64::from(first) - x_before;
            ensure!(
                (moved - step * SPEED).abs() < 0.001,
                "stepped frame {frame} moved the boxes by {moved} points, not by {}",
                step * SPEED
            );

            time_before = time;
            x_before = f64::from(first);
        }
        checkpoint("30 stepped frames, both boxes moved 150 points")?;

        let (frame, real) = from_main(move || {
            view.stop();
            Clock::exit_stepped();
            (frame_seconds(), monotonic_seconds())
        });
        ensure!(
            (-0.05..0.1).contains(&(real - frame)),
            "back in real time the frame time is {} s behind",
            real - frame
        );

        Ok(())
    }
}
