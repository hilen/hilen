use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    gm::Clock,
    refs::Weak,
    ui::{Setup, UIManager, ViewData, ViewTest, view},
    ui_test::{check_colors, step_frames},
};

use super::common::{Pages, WIDTH, progress, settle};

/// The app turns to a page with the move of a turn, or opens one at once.
#[view]
struct PageViewTurnTo {
    #[init]
    pages: Pages,
}

impl Setup for PageViewTurnTo {
    fn setup(self: Weak<Self>) {
        self.pages.place().back();
    }
}

impl ViewTest for PageViewTurnTo {
    fn before_start() {
        UIManager::set_drag_scrolling(true);
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        let pages = view.pages;
        from_main(Clock::enter_stepped);
        from_main(move || {
            pages.pager.turn_to("3", true);
        });
        step_frames(8);
        // Page 3 comes in where page 2 would, so it is 1 page from page 1.
        let moved = progress(pages);
        let open = from_main(move || pages.pager.page().map(str::to_string));
        let toward = if open.as_deref() == Some("3") {
            moved - WIDTH
        } else {
            moved
        };
        ensure!(
            toward > 0.0 && toward < WIDTH,
            "a turn by code is {moved} points along with {open:?} open"
        );
        ensure!(pages.wills == ["3"], "will turn told {:?}", pages.wills);
        check_colors(
            r"
              12    4 - #3378f5
             128    4 - #2eb26b
             368    4 - #2eb26b
             248   16 - #2eb26b
             592   44 - #1a1a1f
               4  116 - #3378f5
             264  136 - #ffffff
             384  136 - #fefffe
             372  140 - #ffffff
             328  144 - #fefffe
             272  148 - #2eb26b
             296  148 - #2eb26b
             352  148 - #fefffe
             388  148 - #2eb26b
             324  152 - #2eb26b
             316  156 - #2eb26b
             264  160 - #fefffe
             296  160 - #2eb26b
             328  160 - #ffffff
             372  160 - #ffffff
             304  164 - #fefffe
             344  164 - #fefffe
             320  172 - #fefffe
             124  176 - #3378f5
              96  268 - #3378f5
               4  284 - #3378f5
             184  296 - #2eb26b
             396  296 - #2eb26b
             592  324 - #1a1a1f
             300  568 - #1a1a1f
               4  592 - #1a1a1f
             592  592 - #1a1a1f
            ",
        )?;
        settle();
        ensure!(
            from_main(move || pages.pager.page() == Some("3")),
            "page 3 is not open"
        );
        ensure!(
            pages.turns.len() == 1 && pages.turns[0].completed,
            "turned told {:?}",
            pages.turns
        );

        from_main(move || {
            pages.pager.set_page("5");
        });
        step_frames(1);
        ensure!(
            from_main(move || pages.pager.page() == Some("5")),
            "page 5 is not open"
        );
        let offset = from_main(move || pages.pager.offset());
        ensure!(offset.abs() < f32::EPSILON, "an open with no move is at {offset}");
        ensure!(pages.turns.len() == 1, "an open with no move told a turn");
        from_main(Clock::exit_stepped);
        Ok(())
    }
}
