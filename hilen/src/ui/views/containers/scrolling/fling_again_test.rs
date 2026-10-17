use anyhow::{Result, bail, ensure};

use crate::{
    self as hilen,
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    ui::{ScrollView, Setup, UIManager, ViewData, ViewTest, view},
    ui_test::inject_touches,
};

/// A fling started while the last one still runs moves at the speed of
/// its own drag. Before the fix every release added 1 more fling
/// animation, all of them moved the content in every frame, so a second
/// fling went twice as fast. A press during a fling did not stop it.
#[view]
struct FlingAgain {
    #[init]
    scroll: ScrollView,
}

impl Setup for FlingAgain {
    fn setup(mut self: Weak<Self>) {
        self.scroll.set_content_size((600, 100_000));
        self.scroll.place().back();
    }
}

/// A drag up by 40 points that lets go while it still moves.
fn flick() {
    inject_touches(
        "
        300 300 b
        300 280 m
        300 260 m
        300 260 e
    ",
    );
}

/// The frames until the content stops.
fn frames_until_stop(view: Weak<FlingAgain>) -> Result<u32> {
    for frame in 0..2000 {
        if !from_main(move || view.scroll.is_scrolling()) {
            return Ok(frame);
        }
        wait_for_next_frame();
    }
    bail!("the fling did not stop in 2000 frames")
}

fn offset(view: Weak<FlingAgain>) -> f32 {
    from_main(move || view.scroll.get_scroll_content_offset())
}

impl ViewTest for FlingAgain {
    // Flings come from finger drags, which scroll only with drag
    // scrolling on, the touch platform default.
    fn before_start() {
        UIManager::set_drag_scrolling(true);
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        flick();
        let alone = frames_until_stop(view)?;
        ensure!(alone > 20, "a flick flew only {alone} frames");

        // The same flick again, while the one before still flies.
        flick();
        for _ in 0..10 {
            wait_for_next_frame();
        }
        flick();
        let again = frames_until_stop(view)?;
        ensure!(
            again.abs_diff(alone) <= 2,
            "a flick alone flew {alone} frames, a flick over a running fling flew {again}"
        );

        // A finger that lands on a fling holds the content still.
        flick();
        for _ in 0..10 {
            wait_for_next_frame();
        }
        inject_touches("300 300 b");
        let held = offset(view);
        for _ in 0..10 {
            wait_for_next_frame();
        }
        let after = offset(view);
        ensure!(
            (after - held).abs() < 0.01,
            "a finger held on a fling, the content still moved from {held} to {after}"
        );
        inject_touches("300 300 e");
        ensure!(
            frames_until_stop(view)? == 0,
            "a press on a fling and a release with no move started a new fling"
        );

        Ok(())
    }
}
