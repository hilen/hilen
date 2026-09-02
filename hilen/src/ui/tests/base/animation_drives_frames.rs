use std::{sync::mpsc::channel, time::Duration};

use anyhow::{Result, bail, ensure};
use log::error;

use crate::{
    deps::{hreads::from_main, refs::Weak},
    gm::Animation,
    ui::{Container, Setup, UIAnimation, ViewData, ViewFrame, ViewTest, view},
    window::continuous_render_active,
};

const TARGET_Y: f32 = 120.0;
const DURATION: f32 = 0.2;

/// Generous next to the animation itself. A stalled loop never finishes at all,
/// so this only has to be long enough not to be flaky on a slow simulator.
const GIVE_UP_AFTER: Duration = Duration::from_secs(10);

// Regression test. Render on demand lets the loop sleep unless a frame is
// requested. An animation started from code, with nothing injecting input to
// keep waking the loop, used to draw one frame and then stall forever, which
// hung the whole run. Nothing here injects input, so the animation has to drive
// its own frames to reach the end.
#[view]
struct AnimationDrivesFrames {
    #[init]
    square: Container,
}

impl Setup for AnimationDrivesFrames {
    fn setup(self: Weak<Self>) {
        self.square.set_frame((20, 0, 40, 40));
    }
}

impl ViewTest for AnimationDrivesFrames {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let (send, finished) = channel();

        // The state check rides along in this one call on purpose. Every
        // `from_main` wakes the loop, so a separate call here would hand the
        // animation the frame it is supposed to ask for itself and hide a
        // stall.
        let continuous = from_main(move || {
            let anim = UIAnimation::new(|square, y| {
                square.set_y(y);
            })
            .animation(Animation::new(0.0, TARGET_Y, DURATION));

            anim.on_finish.sub(move || {
                if send.send(()).is_err() {
                    error!("animation finished after the test stopped waiting for it");
                }
            });

            view.square.add_animation(anim);

            continuous_render_active()
        });

        // The loop only keeps drawing while it knows continuous work is live.
        // Break that and the animation stalls on every platform that sleeps.
        ensure!(continuous, "a live animation must keep the loop drawing");

        // Nothing may touch the main thread until this returns.
        #[cfg(not_wasm)]
        let finished_in_time = finished.recv_timeout(GIVE_UP_AFTER).is_ok();

        // recv_timeout needs std Instant, which panics in the browser.
        // The only timed block wasm has is Atomics.wait with a timeout,
        // which is what thread::sleep is on a worker, so the timeout
        // arm polls the channel in short sleeps.
        #[cfg(wasm)]
        let finished_in_time = {
            let start = web_time::Instant::now();
            loop {
                if finished.try_recv().is_ok() {
                    break true;
                }
                if start.elapsed() > GIVE_UP_AFTER {
                    break false;
                }
                std::thread::sleep(Duration::from_millis(16));
            }
        };

        if !finished_in_time {
            bail!("animation never finished, the loop stopped drawing frames for it");
        }

        let y = from_main(move || view.square.y());

        // The last commit lands just before the animation expires, so the exact
        // end value is never written. Past halfway is what proves frames kept
        // coming. A loop that stalled leaves the square near where it started.
        ensure!(
            y > TARGET_Y / 2.0,
            "animation only reached y {y} of {TARGET_Y}, so it barely drew a frame"
        );

        Ok(())
    }
}
