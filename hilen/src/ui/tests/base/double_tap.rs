use std::{thread::sleep, time::Duration};

use anyhow::{Result, ensure};

use crate::{
    deps::{hreads::from_main, refs::Weak},
    gm::{
        Clock,
        color::{BLUE, GRAY, GREEN, RED},
    },
    ui::{Container, Label, Setup, ViewData, ViewFrame, ViewTest, ViewTouch, view},
    ui_test::{checkpoint, inject_touches, step_frames, system_input::wait_until},
};

/// A third of a second, more than the time 2 taps of a double tap may be
/// apart.
const PAST_THE_WAIT: u32 = 20;

/// 2 taps soon after each other at one place fire `double_tap` on a view,
/// and a tap that gets no second one fires `single_tap` after the wait.
/// `up_inside` still fires for every tap at once, on a view that listens
/// to the new events and on a plain one.
#[view]
struct DoubleTap {
    taps:       u32,
    singles:    u32,
    doubles:    u32,
    plain_taps: u32,

    #[init]
    counts:        Label,
    caption:       Label,
    pad:           Container,
    plain_counts:  Label,
    plain_caption: Label,
    plain:         Container,
    marker:        Container,
}

impl Setup for DoubleTap {
    fn setup(mut self: Weak<Self>) {
        self.counts.set_text_size(24).place().t(10).lr(10).h(40);
        self.caption
            .set_text("pad with single and double tap, green after a double tap")
            .set_text_size(18);
        self.caption.place().t(50).lr(10).h(30);

        self.pad.set_color(BLUE).place().t(85).lr(100).h(200);
        self.pad.enable_touch();
        self.pad.touch().up_inside.sub(self, move || {
            self.taps += 1;
            self.show();
        });
        self.pad.touch().single_tap.sub(self, move || {
            self.singles += 1;
            self.show();
        });
        self.pad.touch().double_tap.sub(self, move || {
            self.doubles += 1;
            self.show();
        });

        self.plain_counts.set_text_size(24).place().t(310).lr(10).h(40);
        self.plain_caption
            .set_text("plain pad, it listens to up_inside only")
            .set_text_size(18);
        self.plain_caption.place().t(350).lr(10).h(30);

        self.plain.set_color(GRAY).place().t(385).lr(100).h(200);
        self.plain.enable_touch();
        self.plain.touch().up_inside.sub(self, move || {
            self.plain_taps += 1;
            self.show();
        });

        self.marker.set_color(RED).set_corner_radius(8);
        self.marker.set_hidden(true);
        self.show();
    }
}

impl DoubleTap {
    fn show(self: Weak<Self>) {
        self.counts.set_text(format!(
            "up_inside {}, single taps {}, double taps {}",
            self.taps, self.singles, self.doubles
        ));
        self.plain_counts.set_text(format!("up_inside {}", self.plain_taps));
        let color = if self.doubles.is_multiple_of(2) {
            BLUE
        } else {
            GREEN
        };
        self.pad.set_color(color);
    }

    /// Taps a point, with a red dot where the tap lands.
    fn tap(self: Weak<Self>, x: u32, y: u32) {
        from_main(move || {
            self.marker.set_hidden(false);
            self.marker.set_frame((x - 8, y - 8, 16, 16));
        });
        inject_touches(format!("{x} {y} b\n{x} {y} e"));
    }

    /// Moves the clock past the wait for a second tap and lets the single
    /// tap that was due land. A single tap that must not fire gets the
    /// same time to show up.
    fn pass_the_wait(self: Weak<Self>, singles: u32) -> Result<()> {
        step_frames(PAST_THE_WAIT);
        if from_main(move || self.singles) < singles {
            return wait_until("the single tap", move || self.singles >= singles);
        }
        sleep(Duration::from_millis(150));
        Ok(())
    }

    fn expect(self: Weak<Self>, expected: (u32, u32, u32), label: &str) -> Result<()> {
        let got = from_main(move || (self.taps, self.singles, self.doubles));
        ensure!(
            got == expected,
            "{label}: expected up_inside, single and double taps {expected:?}, got {got:?}"
        );
        checkpoint(label)
    }
}

impl ViewTest for DoubleTap {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // The clock stands between the taps and moves only by `step_frames`.
        from_main(Clock::enter_stepped);

        view.tap(300, 185);
        view.expect((1, 0, 0), "1 tap: up_inside at once, the single tap still waits")?;

        view.pass_the_wait(1)?;
        view.expect((1, 1, 0), "no second tap came: the single tap fires")?;

        view.tap(300, 185);
        view.tap(300, 185);
        view.expect((3, 1, 1), "2 taps at once: a double tap, the pad is green")?;

        view.pass_the_wait(1)?;
        view.expect((3, 1, 1), "after the wait: a double tap fires no single tap")?;

        view.tap(300, 185);
        view.tap(300, 185);
        view.tap(300, 185);
        view.expect(
            (6, 1, 2),
            "3 taps at once: 1 double tap, the third starts a new pair",
        )?;

        view.pass_the_wait(2)?;
        view.expect((6, 2, 2), "after the wait: the third tap is a single tap")?;

        view.tap(200, 185);
        view.tap(400, 185);
        view.pass_the_wait(4)?;
        view.expect((8, 4, 2), "2 taps at once but 200 points apart: 2 single taps")?;

        view.tap(300, 485);
        view.tap(300, 485);
        let plain = from_main(move || view.plain_taps);
        ensure!(
            plain == 2,
            "the plain pad counts each of 2 fast taps at once, got {plain}"
        );
        view.expect(
            (8, 4, 2),
            "2 fast taps on the plain pad: 2 up_inside at once, no wait",
        )?;

        from_main(Clock::exit_stepped);

        Ok(())
    }
}
