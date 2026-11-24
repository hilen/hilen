use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    gm::Clock,
    refs::Weak,
    ui::{Setup, UIManager, ViewData, ViewTest, view},
    ui_test::step_frames,
};

use super::common::{Pages, WIDTH, lift, press, settle, slide};

/// Past the first and the last page the pages pull like the rubber band of
/// a scroll view of iOS, and go back at the release with nothing told.
#[view]
struct PageViewRubber {
    #[init]
    pages: Pages,
}

impl Setup for PageViewRubber {
    fn setup(self: Weak<Self>) {
        self.pages.place().back();
    }
}

impl ViewTest for PageViewRubber {
    fn before_start() {
        UIManager::set_drag_scrolling(true);
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        let pages = view.pages;
        from_main(Clock::enter_stepped);
        press(100.0, 150.0);
        slide(150.0, 100.0, 300.0, 8.0);
        // The finger is 184 points past the swipe start.
        let pull = (1.0 - 1.0 / (184.0 * 0.55 / WIDTH + 1.0)) * WIDTH;
        let offset = from_main(move || pages.pager.offset());
        ensure!(
            (offset - pull).abs() < 0.01,
            "the first page is at {offset}, the rubber band gives {pull}"
        );
        step_frames(5);
        lift(300.0, 150.0);
        settle();
        let offset = from_main(move || pages.pager.offset());
        ensure!(
            offset.abs() < f32::EPSILON,
            "the first page did not come back, it is at {offset}"
        );

        from_main(move || {
            pages.pager.set_page("5");
        });
        step_frames(1);
        press(300.0, 150.0);
        slide(150.0, 300.0, 100.0, 8.0);
        let offset = from_main(move || pages.pager.offset());
        ensure!(
            (offset + pull).abs() < 0.01,
            "the last page is at {offset}, the rubber band gives -{pull}"
        );
        step_frames(5);
        lift(100.0, 150.0);
        settle();
        ensure!(
            pages.wills.is_empty() && pages.turns.is_empty(),
            "a pull told {:?} {:?}",
            pages.wills,
            pages.turns
        );
        from_main(Clock::exit_stepped);
        Ok(())
    }
}
