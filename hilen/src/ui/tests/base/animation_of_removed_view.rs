use std::{
    sync::atomic::{AtomicBool, Ordering},
    thread::sleep,
    time::Duration,
};

use anyhow::{Result, ensure};

use crate::{
    deps::{hreads::from_main, refs::Weak},
    gm::Animation,
    ui::{Container, Setup, UIAnimation, ViewData, ViewFrame, ViewSubviews, ViewTest, view},
};

const DURATION: f32 = 0.3;

/// Past the end of the animation.
const PAST_THE_END: Duration = Duration::from_millis(800);

static FINISHED: AtomicBool = AtomicBool::new(false);

/// A view removed while it slides takes the finish of its animation with
/// it. The handler of a gone view reaches for views of a gone screen: a
/// test that failed mid slide on iOS left one that panicked the next test.
#[view]
struct AnimationOfRemovedView {
    square: Weak<Container>,
}

impl Setup for AnimationOfRemovedView {
    fn setup(mut self: Weak<Self>) {
        self.square = self.add_view::<Container>();
        self.square.set_frame((0, 20, 40, 40));
    }
}

impl ViewTest for AnimationOfRemovedView {
    fn before_start() {
        FINISHED.store(false, Ordering::Relaxed);
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(move || {
            let slide = UIAnimation::new(|square, x| {
                square.set_x(x);
            })
            .animation(Animation::new(0.0, 200.0, DURATION));
            slide.on_finish.sub(|| FINISHED.store(true, Ordering::Relaxed));
            view.square.add_animation(slide);
        });
        from_main(move || {
            let mut square = view.square;
            square.remove_from_superview();
        });
        sleep(PAST_THE_END);
        // A frame runs the list of animations once more.
        from_main(|| {});
        ensure!(
            !FINISHED.load(Ordering::Relaxed),
            "the animation of a removed view ran its finish"
        );
        Ok(())
    }
}
