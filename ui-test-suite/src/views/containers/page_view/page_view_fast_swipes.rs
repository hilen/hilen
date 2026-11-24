use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    gm::Clock,
    refs::Weak,
    ui::{Setup, UIManager, ViewData, ViewTest, view},
    ui_test::{check_colors, step_frames},
};

use super::common::{Pages, WIDTH, lift, press, settle, slide};

/// A swipe on pages that still glide goes on from where they are, to the
/// page after the next one, with no wait for the glide to end and nothing
/// empty beside the pages.
#[view]
struct PageViewFastSwipes {
    #[init]
    pages: Pages,
}

impl Setup for PageViewFastSwipes {
    fn setup(self: Weak<Self>) {
        self.pages.place().back();
    }
}

impl ViewTest for PageViewFastSwipes {
    fn before_start() {
        UIManager::set_drag_scrolling(true);
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        let pages = view.pages;
        from_main(Clock::enter_stepped);
        // 3 flicks of 100 points at 960 points a second, each lands 12 frames
        // into the glide of the one before.
        for _ in 0..3 {
            press(300.0, 150.0);
            slide(150.0, 300.0, 200.0, 16.0);
            lift(200.0, 150.0);
            step_frames(12);
            let open = from_main(move || pages.pager.page().map(str::to_string));
            let offset = from_main(move || pages.pager.offset());
            ensure!(
                offset.abs() < WIDTH / 2.0,
                "the pages at {offset} are not on the page nearest them, {open:?}"
            );
        }
        ensure!(
            from_main(move || pages.pager.page() == Some("4")),
            "3 fast flicks did not reach page 4"
        );
        check_colors(
            r"
               4    4 - #2eb26b
             252    4 - #9e54d9
             396    4 - #9e54d9
             592    4 - #1a1a1f
              24   80 - #2eb26b
             172  136 - #fffeff
             288  136 - #ffffff
             196  144 - #ffffff
             228  144 - #ffffff
             176  148 - #9e54d9
             208  148 - #ffffff
             248  148 - #9e54d9
             284  148 - #9e54d9
               4  152 - #2eb26b
             204  156 - #9e54d9
             220  156 - #9e54d9
             240  156 - #fffeff
             192  160 - #fffeff
             252  160 - #9e54d9
             208  164 - #fffeff
             232  164 - #fffeff
             288  164 - #ffffff
             224  172 - #ffffff
              24  192 - #2eb26b
               4  228 - #2eb26b
              32  296 - #9e54d9
             168  296 - #9e54d9
             396  296 - #9e54d9
             592  380 - #1a1a1f
               4  532 - #1a1a1f
             232  592 - #1a1a1f
             592  592 - #1a1a1f
            ",
        )?;
        settle();
        ensure!(
            from_main(move || pages.pager.page() == Some("4")),
            "the pages did not rest on page 4"
        );
        let offset = from_main(move || pages.pager.offset());
        ensure!(offset.abs() < f32::EPSILON, "the pages rest at {offset}");
        ensure!(pages.wills == ["2", "3", "4"], "will turn told {:?}", pages.wills);
        ensure!(
            pages.turns.len() == 1 && pages.turns[0].key == "4" && pages.turns[0].completed,
            "turned told {:?}",
            pages.turns
        );

        // A flick right back while they glide to page 5 goes back.
        press(300.0, 150.0);
        slide(150.0, 300.0, 200.0, 16.0);
        lift(200.0, 150.0);
        step_frames(4);
        press(100.0, 150.0);
        slide(150.0, 100.0, 140.0, 8.0);
        lift(140.0, 150.0);
        settle();
        let open = from_main(move || pages.pager.page().map(str::to_string));
        ensure!(
            open.as_deref() == Some("4"),
            "a flick back on the way to page 5 ended on {open:?}"
        );
        from_main(Clock::exit_stepped);
        Ok(())
    }
}
