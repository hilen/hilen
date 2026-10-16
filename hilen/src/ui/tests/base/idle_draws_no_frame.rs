use std::{
    sync::mpsc::{Sender, channel},
    thread::sleep,
    time::Duration,
};

use anyhow::{Result, ensure};
use log::error;

use super::keep_frames_while::reached_in_time;
use crate::{
    self as hilen,
    deps::{
        hreads::{after, from_main, on_main, spawn},
        refs::Weak,
    },
    gm::color::LIGHT_BLUE,
    ui::{Label, Setup, ViewCallbacks, ViewData, ViewSubviews, ViewTest, view},
    ui_test::checkpoint,
    window::Window,
};

/// How long the screen stays still while the updates are counted.
const STILL_FOR: Duration = Duration::from_secs(1);

/// The frames a `from_main` may draw: the one that runs it and the one its
/// label change asks for.
const WAKE_FRAMES: u32 = 2;

/// Counts the frames it was updated in, and says so once when an update
/// sees the change that a timer or a task made.
#[view]
struct WakeCounter {
    updates: u32,
    changed: bool,
    seen:    Option<Sender<()>>,

    #[init]
    count: Label,
}

impl Setup for WakeCounter {
    fn setup(self: Weak<Self>) {
        self.set_color(LIGHT_BLUE);
        self.count.set_text_size(24).place().back();
    }
}

impl ViewCallbacks for WakeCounter {
    fn update(&mut self) {
        self.updates += 1;
        self.count.set_text(format!("update ran {} times", self.updates));

        if !self.changed {
            return;
        }
        if let Some(seen) = self.seen.take()
            && seen.send(()).is_err()
        {
            error!("the counter saw the change after the test stopped waiting for it");
        }
    }
}

/// A still screen draws no frame, also in a browser, where the loop used to
/// draw on every refresh. Work that changes the screen with no input, a
/// timer and a task, still gets its frame with nothing else asking.
#[view]
struct IdleDrawsNoFrame {
    counter: Weak<WakeCounter>,

    #[init]
    title: Label,
    state: Label,
}

impl Setup for IdleDrawsNoFrame {
    fn setup(mut self: Weak<Self>) {
        self.title
            .set_text("the blue view counts its updates, no input")
            .set_text_size(20);
        self.title.place().t(20).lr(10).h(40);

        self.state.set_text("nothing asks for frames").set_text_size(20);
        self.state.place().t(70).lr(10).h(40);

        self.counter = self.add_view::<WakeCounter>();
        self.counter.place().t(140).lr(100).h(120);
    }
}

/// Arms the counter for 1 change and returns the end of its channel. The
/// caller makes the change from somewhere the loop does not see.
fn arm(mut counter: Weak<WakeCounter>) -> std::sync::mpsc::Receiver<()> {
    let (send, seen) = channel();
    counter.changed = false;
    counter.seen = Some(send);
    seen
}

impl ViewTest for IdleDrawsNoFrame {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // A headless run draws in a plain loop on purpose, it has no screen
        // to wait for.
        if !Window::headless() {
            let before = from_main(move || view.counter.updates);
            sleep(STILL_FOR);
            let after_still = from_main(move || view.counter.updates);
            ensure!(
                after_still - before <= WAKE_FRAMES,
                "a still screen drew {} frames in {STILL_FOR:?}",
                after_still - before
            );
            checkpoint("a still screen, no frame drawn")?;
        }

        let seen = from_main(move || {
            let mut counter = view.counter;
            let seen = arm(counter);
            view.state.set_text("a timer changes the counter");
            after(0.3, move || counter.changed = true);
            seen
        });
        ensure!(reached_in_time(&seen), "no frame drew the change of a timer");
        checkpoint("a timer woke the loop")?;

        let seen = from_main(move || {
            let mut counter = view.counter;
            let seen = arm(counter);
            view.state.set_text("a task changes the counter");
            spawn(async move {
                crate::deps::hreads::sleep(0.3).await;
                on_main(move || counter.changed = true);
            });
            seen
        });
        ensure!(reached_in_time(&seen), "no frame drew the change of a task");
        checkpoint("a task woke the loop")?;

        Ok(())
    }
}
