use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    gm::Clock,
    refs::Weak,
    ui::{Setup, UIManager, ViewData, ViewTest, view},
    ui_test::{check_colors, step_frames},
};

use super::common::{Pages, lift, press, settle, slide};

/// A release past half the width at rest turns the page, and so does a short
/// flick of 300 points a second or more, while a slower one goes back. The
/// limits were measured on `UIPageViewController`.
#[view]
struct PageViewTurn {
    #[init]
    pages: Pages,
}

impl Setup for PageViewTurn {
    fn setup(self: Weak<Self>) {
        self.pages.place().back();
    }
}

impl ViewTest for PageViewTurn {
    fn before_start() {
        UIManager::set_drag_scrolling(true);
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        let pages = view.pages;
        from_main(Clock::enter_stepped);
        // Past half the width, at rest.
        press(330.0, 150.0);
        slide(150.0, 330.0, 90.0, 8.0);
        step_frames(5);
        lift(90.0, 150.0);
        settle();
        ensure!(
            from_main(move || pages.pager.page() == Some("2")),
            "a release past half did not turn"
        );
        check_colors(
            r"
               4    4 - #f27333
             184    4 - #f27333
             396    4 - #f27333
             592   60 - #1a1a1f
             260  136 - #fffefe
             144  140 - #f27333
             156  144 - #ffffff
             204  144 - #fffefe
             136  148 - #fffbf9
             148  148 - #f27333
             172  148 - #f27333
             220  148 - #f27333
             260  148 - #fffefe
             188  152 - #ffffff
             228  152 - #ffffff
             172  156 - #f27333
             200  156 - #f27333
             212  156 - #ffffff
             396  156 - #f27333
             136  160 - #fffbf9
             248  160 - #fffefe
             168  164 - #fffefe
             220  164 - #fffefe
             256  164 - #fffefe
             196  168 - #f27333
             592  264 - #1a1a1f
               4  296 - #f27333
             396  296 - #f27333
             116  444 - #1a1a1f
             300  476 - #1a1a1f
               4  592 - #1a1a1f
             592  592 - #1a1a1f
            ",
        )?;

        // 40 points at 480 points a second, lifted while it moves.
        press(300.0, 150.0);
        slide(150.0, 300.0, 260.0, 8.0);
        lift(260.0, 150.0);
        settle();
        ensure!(
            from_main(move || pages.pager.page() == Some("3")),
            "a flick did not turn"
        );

        // 40 points at 240 points a second.
        press(300.0, 150.0);
        slide(150.0, 300.0, 260.0, 4.0);
        lift(260.0, 150.0);
        settle();
        ensure!(
            from_main(move || pages.pager.page() == Some("3")),
            "a slow flick turned"
        );
        let completed: Vec<bool> = pages.turns.iter().map(|turn| turn.completed).collect();
        ensure!(completed == [true, true, false], "turned told {:?}", pages.turns);
        from_main(Clock::exit_stepped);
        Ok(())
    }
}
