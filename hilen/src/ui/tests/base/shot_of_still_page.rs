use std::{
    sync::mpsc::{Receiver, channel},
    thread::sleep,
    time::Duration,
};

use anyhow::{Result, anyhow, ensure};
use web_time::Instant;

use crate::{
    self as hilen, AppRunner,
    deps::refs::Weak,
    ui::{Label, Setup, ViewData, ViewTest, view},
    ui_test::checkpoint,
    window::Screenshot,
};

/// How long the screen stays still before the shot.
const STILL_FOR: Duration = Duration::from_secs(1);

/// A shot that takes longer waits for a frame that nothing asks for.
const SHOT_WITHIN: Duration = Duration::from_secs(5);

/// A screenshot of a still screen comes back with no input. On a page drawn
/// by WebGL the copy of the frame reached the shot only in the next frame,
/// and a still page draws none, so the shot and every inspector request
/// after it waited for a key press. The browser lane shows it with
/// `--webgl`.
#[view]
struct ShotOfStillPage {
    #[init]
    title: Label,
}

impl Setup for ShotOfStillPage {
    fn setup(self: Weak<Self>) {
        self.title.set_text("a still screen, then a screenshot").set_text_size(20);
        self.title.place().t(20).lr(10).h(40);
    }
}

/// The shot, none when it took longer than `SHOT_WITHIN`. `recv_timeout`
/// needs the std clock, which panics in a browser, so this polls.
fn shot_in_time(shot: &Receiver<Result<Screenshot>>) -> Option<Result<Screenshot>> {
    let start = Instant::now();
    loop {
        if let Ok(result) = shot.try_recv() {
            return Some(result);
        }
        if start.elapsed() > SHOT_WITHIN {
            return None;
        }
        sleep(Duration::from_millis(16));
    }
}

impl ViewTest for ShotOfStillPage {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        sleep(STILL_FOR);

        let (send, shot) = channel();
        let take = move || {
            // The test stopped waiting when the send fails, it says why.
            let _ = send.send(AppRunner::take_screenshot());
        };
        #[cfg(wasm)]
        crate::deps::hreads::spawn_thread(take);
        #[cfg(not_wasm)]
        std::thread::spawn(take);
        let shot = shot_in_time(&shot)
            .ok_or_else(|| anyhow!("no screenshot of a still screen in {SHOT_WITHIN:?}"))??;
        ensure!(shot.size.width > 0 && shot.size.height > 0, "the shot is empty");
        checkpoint("a still screen gave its screenshot")?;

        Ok(())
    }
}
