use std::{thread::sleep, time::Duration};

use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    gm::Clock,
    refs::Weak,
    ui::{Setup, UIManager, ViewData, ViewTest, view},
    ui_test::{inject_scroll_x, inject_touches, step_frames},
};

use super::common::{Pages, progress, settle};

/// A sideways wheel or trackpad scroll over the pages moves them like a
/// finger, and they go into place once it rests.
#[view]
struct PageViewWheel {
    #[init]
    pages: Pages,
}

impl Setup for PageViewWheel {
    fn setup(self: Weak<Self>) {
        self.pages.place().back();
    }
}

impl ViewTest for PageViewWheel {
    fn before_start() {
        UIManager::set_drag_scrolling(true);
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        let pages = view.pages;
        from_main(Clock::enter_stepped);
        // A tap puts the cursor over the pages.
        inject_touches("200 150 b\n200 150 e");
        step_frames(1);
        for _ in 0..20 {
            inject_scroll_x(-12);
            step_frames(1);
        }
        let moved = progress(pages);
        ensure!(
            (moved - 240.0).abs() < 0.01,
            "the pages moved {moved} points for 240 points of wheel"
        );
        // The wheel rests 0.12 s of real time before the pages go.
        sleep(Duration::from_millis(200));
        settle();
        ensure!(
            from_main(move || pages.pager.page() == Some("2")),
            "a wheel past half did not turn"
        );
        let offset = from_main(move || pages.pager.offset());
        ensure!(offset.abs() < f32::EPSILON, "the pages stand at {offset}");
        from_main(Clock::exit_stepped);
        Ok(())
    }
}
