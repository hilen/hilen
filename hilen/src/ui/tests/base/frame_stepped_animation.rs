use std::sync::mpsc::channel;

use anyhow::{Result, ensure};
use log::error;

use crate::{
    deps::{hreads::from_main, refs::Weak},
    gm::{Animation, Clock},
    ui::{Container, Setup, UIAnimation, ViewData, ViewFrame, ViewTest, view},
    ui_test::step_frames,
};

const TARGET_Y: f32 = 300.0;
const DURATION: f32 = 0.5;

// 0.5 s at the 60 fps stepped timeline is 30 frames. Half way is 15 frames,
// where the linear animation must sit at exactly half of TARGET_Y no matter how
// fast the machine runs. That exactness is the whole point of stepped time.
const HALF_FRAMES: u32 = 15;

/// Frame stepped time makes an animation deterministic. Under real time a test
/// can only wait for the end and check the settled value. Here the clock only
/// moves when the test asks for a frame, so a mid animation value lands on an
/// exact number and `on_finish` fires on an exact frame count.
#[view]
struct FrameSteppedAnimation {
    #[init]
    square: Container,
}

impl Setup for FrameSteppedAnimation {
    fn setup(self: Weak<Self>) {
        self.square.set_frame((20, 0, 40, 40));
    }
}

impl ViewTest for FrameSteppedAnimation {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let (send, finished) = channel();

        from_main(move || {
            Clock::enter_stepped();

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
        });

        step_frames(HALF_FRAMES);

        let mid = from_main(move || view.square.y());
        ensure!(
            (mid - TARGET_Y / 2.0).abs() < 0.5,
            "at half the frames the value must be exactly half, got {mid} of {TARGET_Y}"
        );

        // The animation has not finished at the half way point.
        ensure!(
            finished.try_recv().is_err(),
            "animation finished before its duration elapsed"
        );

        // Step the rest, plus one so the frame that crosses the end runs and
        // fires on_finish.
        step_frames(HALF_FRAMES + 1);

        ensure!(
            finished.try_recv().is_ok(),
            "animation never finished after its full duration of stepped frames"
        );

        let end = from_main(move || view.square.y());
        ensure!(
            end > TARGET_Y * 0.9,
            "the last committed frame should be near the end, got {end} of {TARGET_Y}"
        );

        from_main(Clock::exit_stepped);

        Ok(())
    }
}
