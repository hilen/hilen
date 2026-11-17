use std::{sync::mpsc::channel, time::Duration};

use anyhow::{Result, bail, ensure};
use log::error;

use crate::{
    deps::{hreads::from_main, refs::Weak},
    gm::Animation,
    ui::{Container, Setup, UIAnimation, ViewData, ViewFrame, ViewTest, view},
};

const TARGET_X: f32 = 140.0;
const DURATION: f32 = 0.15;
const GIVE_UP_AFTER: Duration = Duration::from_secs(10);

// Regression test. The finish of an animation ran while the list of animations
// was locked, so an animation started from `on_finish`, like the second leg of
// a slide, waited for that lock forever and the app froze. banda froze this way
// after every swipe to the next session.
#[view]
struct AnimationFromFinish {
    #[init]
    square: Container,
}

impl Setup for AnimationFromFinish {
    fn setup(self: Weak<Self>) {
        self.square.set_frame((0, 20, 40, 40));
    }
}

impl ViewTest for AnimationFromFinish {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let (send, finished) = channel();

        from_main(move || {
            let first = UIAnimation::new(|square, y| {
                square.set_y(y);
            })
            .animation(Animation::new(20.0, 60.0, DURATION));

            first.on_finish.sub(move || {
                let second = UIAnimation::new(|square, x| {
                    square.set_x(x);
                })
                .animation(Animation::new(0.0, TARGET_X, DURATION));
                let send = send.clone();
                second.on_finish.sub(move || {
                    if send.send(()).is_err() {
                        error!("the second animation finished after the test stopped waiting");
                    }
                });
                view.square.add_animation(second);
            });

            view.square.add_animation(first);
        });

        #[cfg(not_wasm)]
        let finished_in_time = finished.recv_timeout(GIVE_UP_AFTER).is_ok();

        // recv_timeout needs std Instant, which panics in the browser, see
        // `animation_drives_frames`.
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
            bail!("an animation started from on_finish never finished, the main thread is stuck");
        }

        let x = from_main(move || view.square.x());
        ensure!(
            x > TARGET_X / 2.0,
            "the second animation only reached x {x} of {TARGET_X}"
        );

        Ok(())
    }
}
