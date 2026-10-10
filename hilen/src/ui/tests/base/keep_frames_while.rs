#[cfg(wasm)]
use std::thread::sleep;
use std::{
    sync::mpsc::{Receiver, Sender, channel},
    time::Duration,
};

use anyhow::{Result, ensure};
use log::error;
#[cfg(wasm)]
use web_time::Instant;

use crate::{
    self as hilen,
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    gm::color::LIGHT_BLUE,
    ui::{Label, Setup, ViewCallbacks, ViewData, ViewSubviews, ViewTest, view},
    ui_test::checkpoint,
    window::continuous_render_active,
};

/// The updates the counter has to see with nothing asking for a frame but
/// the view itself.
const UPDATES: u32 = 30;

/// A stalled loop never gets there, so this only has to be long enough not
/// to be flaky on a slow simulator.
const GIVE_UP_AFTER: Duration = Duration::from_secs(10);

/// Counts the frames it was updated in, and says so once when it has seen
/// enough of them.
#[view]
struct FrameCounter {
    wanted:    bool,
    updates:   u32,
    report_at: u32,
    reached:   Option<Sender<()>>,

    #[init]
    count: Label,
}

impl Setup for FrameCounter {
    fn setup(self: Weak<Self>) {
        self.set_color(LIGHT_BLUE);
        self.count.set_text_size(24).place().back();
    }
}

impl ViewCallbacks for FrameCounter {
    fn update(&mut self) {
        self.updates += 1;
        self.count.set_text(format!("update ran {} times", self.updates));

        if self.updates < self.report_at {
            return;
        }
        if let Some(reached) = self.reached.take()
            && reached.send(()).is_err()
        {
            error!("the counter got there after the test stopped waiting for it");
        }
    }
}

/// A view that asks for frames with `keep_frames_while` and nothing
/// injecting input. Render on demand draws no frame by itself, so `update`
/// of the counter runs on and on only when that call keeps the loop
/// drawing. The loop has to sleep again when the condition ends, when a
/// new condition that is off replaces one that is on, and when the view
/// that asked is gone.
#[view]
struct KeepFramesWhile {
    counter: Weak<FrameCounter>,

    #[init]
    title: Label,
    state: Label,
}

impl Setup for KeepFramesWhile {
    fn setup(mut self: Weak<Self>) {
        self.title
            .set_text("the blue view counts its updates, no input")
            .set_text_size(20);
        self.title.place().t(20).lr(10).h(40);

        self.state.set_text("nothing asks for frames").set_text_size(20);
        self.state.place().t(70).lr(10).h(40);

        self.counter = self.add_view::<FrameCounter>();
        self.counter.place().t(140).lr(100).h(120);
    }
}

/// Waits with no touch of the main thread. A `from_main` here would wake
/// the loop, hand the view the frame it has to ask for itself and hide a
/// stall.
#[cfg(not_wasm)]
fn reached_in_time(reached: &Receiver<()>) -> bool {
    reached.recv_timeout(GIVE_UP_AFTER).is_ok()
}

/// `recv_timeout` needs the std `Instant`, which panics in a browser, so
/// this polls the channel in short sleeps.
#[cfg(wasm)]
fn reached_in_time(reached: &Receiver<()>) -> bool {
    let start = Instant::now();
    loop {
        if reached.try_recv().is_ok() {
            return true;
        }
        if start.elapsed() > GIVE_UP_AFTER {
            return false;
        }
        sleep(Duration::from_millis(16));
    }
}

fn sleeps_after_two_frames() -> bool {
    wait_for_next_frame();
    wait_for_next_frame();
    !from_main(continuous_render_active)
}

impl ViewTest for KeepFramesWhile {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let (send, reached) = channel();

        // The state check rides along in this one call on purpose, a second
        // `from_main` would wake the loop.
        let continuous = from_main(move || {
            let mut counter = view.counter;
            counter.wanted = true;
            counter.report_at = counter.updates + UPDATES;
            counter.reached = Some(send);
            counter.keep_frames_while(move || counter.wanted);
            view.state.set_text("the view asks for frames while its flag is on");
            continuous_render_active()
        });
        ensure!(
            continuous,
            "a view that asks for frames must keep the loop drawing"
        );
        ensure!(
            reached_in_time(&reached),
            "update did not run {UPDATES} times, the loop stopped drawing frames for the view"
        );
        checkpoint("the counter runs on its own, no input")?;

        from_main(move || {
            let mut counter = view.counter;
            counter.wanted = false;
            view.state.set_text("the flag is off, the loop sleeps");
        });
        ensure!(
            sleeps_after_two_frames(),
            "the loop must sleep when the condition ends"
        );
        checkpoint("the flag is off, the loop sleeps")?;

        let continuous = from_main(move || {
            view.counter.keep_frames_while(|| true);
            view.state.set_text("the view asks for frames with no end");
            continuous_render_active()
        });
        ensure!(continuous, "a view that asks again must wake the loop");
        from_main(move || {
            let counter = view.counter;
            counter.keep_frames_while(move || counter.wanted);
            view.state.set_text("a new condition that is off took its place");
        });
        ensure!(
            sleeps_after_two_frames(),
            "a new condition must replace the one before it"
        );
        checkpoint("a new condition that is off, the loop sleeps")?;

        let continuous = from_main(move || {
            view.counter.keep_frames_while(|| true);
            view.state.set_text("the view asks for frames with no end");
            continuous_render_active()
        });
        ensure!(continuous, "a view that asks again must wake the loop");
        from_main(move || {
            let mut counter = view.counter;
            counter.remove_from_superview();
            view.state.set_text("the view that asked is gone, the loop sleeps");
        });
        ensure!(
            sleeps_after_two_frames(),
            "the loop must sleep when the view that asked is gone"
        );
        checkpoint("the view that asked is gone, the loop sleeps")?;

        Ok(())
    }
}
