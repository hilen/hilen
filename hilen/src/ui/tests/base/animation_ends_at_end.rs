use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use anyhow::{Result, ensure};

use crate::{
    deps::{hreads::from_main, refs::Weak},
    gm::{Animation, Clock},
    ui::{Container, Label, Setup, UIAnimation, ViewData, ViewFrame, ViewTest, view},
    ui_test::{checkpoint, step_frames},
};

const TARGET_Y: f32 = 200.0;
/// 0.26 s lies between frame 15 and frame 16 of the stepped 60 fps clock,
/// so no frame lands on the end. Frame 15 draws 192 of the 200 points, and
/// frame 16 finds the animation over.
const DURATION: f32 = 0.26;
const FRAMES: u32 = 15;

/// A timed animation ends exactly at its end value, also when no frame lands
/// on the end, and before its finish handler runs. A sheet that slid up
/// with no finish handler stayed 16 points under its place without it.
#[view]
struct AnimationEndsAtEnd {
    #[init]
    line:   Container,
    square: Container,
    note:   Label,
}

impl Setup for AnimationEndsAtEnd {
    fn setup(self: Weak<Self>) {
        self.line.set_color("#2f9e44");
        self.line.set_frame((10, TARGET_Y + 40.0, 120, 2));
        self.square.set_color("#1971c2");
        self.square.set_frame((50, 0, 40, 40));
        self.note.set_text("the blue square rests on the green line").set_text_size(16);
        self.note.set_frame((150, TARGET_Y + 20.0, 320, 30));
    }
}

impl ViewTest for AnimationEndsAtEnd {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // The y of the square when the finish handler ran, in hundredths.
        let at_finish = Arc::new(AtomicU32::new(u32::MAX));
        let seen = at_finish.clone();

        from_main(move || {
            Clock::enter_stepped();
            let anim = UIAnimation::new(|square, y| {
                square.set_y(y);
            })
            .animation(Animation::new(0.0, TARGET_Y, DURATION));
            anim.on_finish.sub(move || {
                let y = (view.square.y() * 100.0).round();
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                seen.store(y.max(0.0) as u32, Ordering::SeqCst);
            });
            view.square.add_animation(anim);
        });

        step_frames(FRAMES - 1);
        let before = from_main(move || view.square.y());
        ensure!(
            before < TARGET_Y,
            "1 frame before the end the square is still on its way, it is at {before} of {TARGET_Y}"
        );

        step_frames(2);
        let end = from_main(move || view.square.y());
        ensure!(
            (end - TARGET_Y).abs() < 0.01,
            "a finished animation rests on its end value, the square is at {end} of {TARGET_Y}"
        );
        let finish = at_finish.load(Ordering::SeqCst);
        ensure!(finish != u32::MAX, "the finish handler ran");
        #[allow(clippy::cast_precision_loss)]
        let finish = finish as f32 / 100.0;
        ensure!(
            (finish - TARGET_Y).abs() < 0.01,
            "the end value is set before the finish handler, it saw {finish} of {TARGET_Y}"
        );
        checkpoint("the blue square rests on the green line")?;

        from_main(Clock::exit_stepped);
        Ok(())
    }
}
