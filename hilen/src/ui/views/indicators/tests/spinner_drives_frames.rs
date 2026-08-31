use anyhow::{Result, ensure};

use crate::{
    self as hilen,
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    gm::color::LIGHT_BLUE,
    ui::{ScrollView, Setup, Spinner, ViewData, ViewFrame, ViewSubviews, ViewTest, view},
    ui_test::checkpoint,
    window::continuous_render_active,
};

/// Two spinners and nothing injecting input. One is in plain view, the
/// other sits in a short scroll view past its bottom edge. Render on
/// demand used to leave the dots frozen until a mouse move asked for a
/// frame. A spinner on screen has to keep the loop drawing itself, and a
/// hidden or scrolled out one has to let it sleep.
#[view]
struct SpinnerDrivesFrames {
    shown:   Weak<Spinner>,
    clipped: Weak<Spinner>,

    #[init]
    scroll: ScrollView,
}

impl Setup for SpinnerDrivesFrames {
    fn setup(mut self: Weak<Self>) {
        self.shown = self.add_view::<Spinner>();
        self.shown.dot_color = LIGHT_BLUE;
        self.shown.place().tl(40).size(200, 200);

        self.scroll.place().t(300).l(40).size(200, 120);
        self.clipped = self.scroll.add_view::<Spinner>();
        self.clipped.dot_color = LIGHT_BLUE;
        self.clipped.place().t(200).l(0).size(120, 120);
    }
}

impl ViewTest for SpinnerDrivesFrames {
    fn perform_test(mut view: Weak<Self>) -> Result<()> {
        wait_for_next_frame();
        let continuous = from_main(continuous_render_active);
        ensure!(continuous, "a visible spinner must keep the loop drawing");

        // The first dot moves along its circle every frame, so its position
        // has to differ between two frames without any input.
        let dot = move || from_main(move || view.shown.subviews_weak()[0].frame().origin);
        let before = dot();
        wait_for_next_frame();
        wait_for_next_frame();
        let after = dot();
        ensure!(
            before != after,
            "the spinner dots did not move between frames, stayed at {before}"
        );
        checkpoint("spinning on its own, no input")?;

        // Only the scrolled out spinner is left. It is not on screen, so it
        // must not hold the loop awake.
        from_main(move || {
            view.shown.set_hidden(true);
        });
        wait_for_next_frame();
        wait_for_next_frame();
        let continuous = from_main(continuous_render_active);
        ensure!(
            !continuous,
            "a hidden spinner and a scrolled out one must let the loop sleep"
        );
        checkpoint("top spinner hidden, loop asleep")?;

        from_main(move || {
            view.shown.set_hidden(false);
        });
        wait_for_next_frame();
        let continuous = from_main(continuous_render_active);
        ensure!(continuous, "a spinner shown again must wake the loop");
        checkpoint("shown again, loop awake")?;

        Ok(())
    }
}
