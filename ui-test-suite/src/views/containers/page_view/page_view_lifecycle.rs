use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    gm::Clock,
    refs::Weak,
    ui::{Setup, UIManager, ViewData, ViewTest, view},
    ui_test::step_frames,
};

use super::common::{Pages, lift, press, settle, slide};

/// The source hears of every page the page view makes and drops: the page
/// beside the open one when a finger first moves toward it, the open one
/// that a turn leaves, every page that `clear` takes, and no page when the
/// open one is opened again.
#[view]
struct PageViewLifecycle {
    #[init]
    pages: Pages,
}

impl Setup for PageViewLifecycle {
    fn setup(self: Weak<Self>) {
        self.pages.place().back();
    }
}

impl ViewTest for PageViewLifecycle {
    fn before_start() {
        UIManager::set_drag_scrolling(true);
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        let pages = view.pages;
        from_main(Clock::enter_stepped);
        ensure!(
            pages.made == ["1"] && pages.dropped.is_empty(),
            "at the start {:?} {:?}",
            pages.made,
            pages.dropped
        );

        press(330.0, 150.0);
        slide(150.0, 330.0, 90.0, 8.0);
        step_frames(5);
        lift(90.0, 150.0);
        settle();
        ensure!(pages.made == ["1", "2"], "a turn made {:?}", pages.made);
        ensure!(pages.dropped == ["1"], "a turn dropped {:?}", pages.dropped);

        from_main(move || {
            pages.pager.set_page("2");
        });
        step_frames(1);
        ensure!(
            pages.made.len() == 2 && pages.dropped.len() == 1,
            "opening the open page again made or dropped one"
        );

        from_main(move || {
            pages.pager.set_page("4");
        });
        step_frames(1);
        ensure!(
            pages.made == ["1", "2", "4"] && pages.dropped == ["1", "2"],
            "opening page 4: {:?} {:?}",
            pages.made,
            pages.dropped
        );

        from_main(move || {
            pages.pager.clear();
        });
        step_frames(1);
        ensure!(
            from_main(move || pages.pager.page().is_none()),
            "a page is open after clear"
        );
        ensure!(
            pages.dropped == ["1", "2", "4"],
            "clear dropped {:?}",
            pages.dropped
        );
        from_main(Clock::exit_stepped);
        Ok(())
    }
}
