use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    gm::Clock,
    refs::Weak,
    ui::{Setup, UIManager, ViewData, ViewTest, view},
    ui_test::{check_colors, step_frames},
};

use super::common::{Pages, lift, press, settle, slide};

/// The pages follow the finger 1 to 1 once it moved 10 points sideways,
/// with no jump, the page after the open one comes in beside it, and a
/// release before half the width at rest takes them back, like
/// `UIPageViewController`.
#[view]
struct PageViewDrag {
    #[init]
    pages: Pages,
}

impl Setup for PageViewDrag {
    fn setup(self: Weak<Self>) {
        self.pages.place().back();
    }
}

impl ViewTest for PageViewDrag {
    fn before_start() {
        UIManager::set_drag_scrolling(true);
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        let pages = view.pages;
        from_main(Clock::enter_stepped);
        press(300.0, 150.0);
        // 8 points a frame: the swipe starts at 16 points, the pages follow
        // from there.
        slide(150.0, 300.0, 200.0, 8.0);
        let offset = from_main(move || pages.pager.offset());
        ensure!(
            (offset + 84.0).abs() < 0.01,
            "the pages are at {offset}, not 16 points behind the finger"
        );
        ensure!(pages.wills == ["2"], "will turn told {:?}", pages.wills);
        check_halfway()?;

        // The finger rests 5 frames, it has no speed then.
        step_frames(5);
        lift(200.0, 150.0);
        settle();
        let offset = from_main(move || pages.pager.offset());
        ensure!(
            offset.abs() < f32::EPSILON,
            "the pages did not go back, they are at {offset}"
        );
        ensure!(
            from_main(move || pages.pager.page() == Some("1")),
            "page 1 is not open"
        );
        ensure!(
            pages.turns.len() == 1 && pages.turns[0].key == "1" && !pages.turns[0].completed,
            "turned told {:?}",
            pages.turns
        );
        check_back()?;
        from_main(Clock::exit_stepped);
        Ok(())
    }
}

/// Page 1 84 points to the left, page 2 beside it.
fn check_halfway() -> Result<()> {
    check_colors(
        r"
           4    4 - #3378f5
         204    4 - #3378f5
         396    4 - #f27333
         316   84 - #f27333
         592   84 - #1a1a1f
          68  136 - #ffffff
         168  136 - #ffffff
          68  144 - #3378f5
          92  144 - #fefeff
         120  144 - #fefeff
          60  148 - #3378f5
         140  148 - #3378f5
          84  152 - #fefeff
         108  152 - #3378f5
         116  156 - #3378f5
         128  156 - #ffffff
          52  160 - #f9fbff
          84  164 - #fefeff
         120  164 - #ffffff
         136  164 - #fefeff
         172  164 - #ffffff
         396  164 - #f27333
         108  168 - #3378f5
         316  204 - #f27333
         396  256 - #f27333
           4  296 - #3378f5
         156  296 - #3378f5
         312  296 - #3378f5
         592  332 - #1a1a1f
         300  588 - #1a1a1f
           4  592 - #1a1a1f
         592  592 - #1a1a1f
        ",
    )
}

/// Page 1 back in its place.
fn check_back() -> Result<()> {
    check_colors(
        r"
           4    4 - #3378f5
         200    4 - #3378f5
         396    4 - #3378f5
         256  136 - #ffffff
         144  140 - #3378f5
         156  140 - #ffffff
         176  144 - #fefeff
         136  148 - #f9fbff
         148  148 - #3378f5
         192  148 - #3378f5
         200  148 - #3378f5
         212  148 - #fefeff
         224  148 - #3378f5
         396  148 - #3378f5
         256  152 - #ffffff
         592  152 - #1a1a1f
         164  156 - #fefeff
         192  156 - #3378f5
         136  160 - #f9fbff
         176  160 - #ffffff
         204  160 - #fefeff
         220  164 - #ffffff
         256  164 - #ffffff
         192  168 - #3378f5
         196  172 - #fefeff
           4  296 - #3378f5
         396  296 - #3378f5
         592  380 - #1a1a1f
         196  384 - #1a1a1f
         296  572 - #1a1a1f
           4  592 - #1a1a1f
         592  592 - #1a1a1f
        ",
    )
}
