use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    gm::Clock,
    refs::Weak,
    ui::{Setup, UIManager, ViewData, ViewTest, view},
    ui_test::step_frames,
};

use super::common::{Pages, WIDTH, lift, press, progress, settle, slide};

/// A finger on gliding pages stops them where they are, the way a scroll
/// view of iOS stops, and a lift with no swipe takes them to the nearest
/// page.
#[view]
struct PageViewCatch {
    #[init]
    pages: Pages,
}

impl Setup for PageViewCatch {
    fn setup(self: Weak<Self>) {
        self.pages.place().back();
    }
}

impl ViewTest for PageViewCatch {
    fn before_start() {
        UIManager::set_drag_scrolling(true);
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        let pages = view.pages;
        from_main(Clock::enter_stepped);
        press(300.0, 150.0);
        slide(150.0, 300.0, 260.0, 8.0);
        lift(260.0, 150.0);
        step_frames(6);
        let gliding = progress(pages);
        ensure!(
            gliding > 24.0 && gliding < WIDTH,
            "the pages are {gliding} points from page 1, not on their way"
        );
        press(200.0, 150.0);
        step_frames(10);
        let held = progress(pages);
        ensure!(
            (held - gliding).abs() < 1.0,
            "the pages went on from {gliding} to {held} under the finger"
        );
        lift(200.0, 150.0);
        settle();
        let near = if held > WIDTH / 2.0 { "2" } else { "1" };
        ensure!(
            from_main(move || pages.pager.page() == Some(near)),
            "the pages stopped at {held} did not go to page {near}"
        );
        let offset = from_main(move || pages.pager.offset());
        ensure!(offset.abs() < f32::EPSILON, "the pages stand at {offset}");
        from_main(Clock::exit_stepped);
        Ok(())
    }
}
