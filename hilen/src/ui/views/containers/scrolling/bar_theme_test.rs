use anyhow::Result;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    dispatch::wait_for_next_frame,
    gm::color::{Color, WHITE},
    ui::{ScrollView, Setup, Theme, ThemeMode, ViewData, ViewTest, view},
    ui_test::check_colors,
};

// The scroll bar thumb must stay visible on both themes: translucent
// black over the light theme, translucent white over the dark one. A
// black thumb on a dark background is what this test protects against.
#[view]
struct ScrollBarTheme {
    #[init]
    scroll: ScrollView,
}

impl Setup for ScrollBarTheme {
    fn setup(mut self: Weak<Self>) {
        self.scroll.set_content_size((300, 1200));
        self.scroll.place().tl(0).size(300, 300);
        self.scroll.set_color(WHITE);
    }
}

impl ViewTest for ScrollBarTheme {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(|| {
            Theme::set_mode(ThemeMode::System);
            Theme::set_system(Theme::Light);
        });
        wait_for_next_frame();

        check_colors(
            r"
               4    4 - #ffffff
             152    4 - #ffffff
             496    4 - #597c95
             296    8 - #a6a6a6
             296   16 - #a6a6a6
             296   24 - #a6a6a6
             296   32 - #a6a6a6
             296   40 - #a6a6a6
             296   48 - #a6a6a6
             296   56 - #a6a6a6
             296   64 - #a6a6a6
             296   68 - #a6a6a6
             296   72 - #a6a6a6
              48   88 - #ffffff
             200   88 - #ffffff
             124  148 - #ffffff
               4  172 - #ffffff
             592  180 - #597c95
             424  184 - #597c95
             244  188 - #ffffff
             168  212 - #ffffff
              80  232 - #ffffff
             156  292 - #ffffff
               8  296 - #ffffff
             300  300 - #597c95
             504  388 - #597c95
               8  444 - #597c95
             360  448 - #597c95
             152  480 - #597c95
               4  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
            ",
        )?;

        from_main(move || {
            view.scroll.set_color(Color::hex("#16161a"));
            Theme::set_system(Theme::Dark);
        });
        wait_for_next_frame();

        check_colors(
            r"
               4    4 - #16161a
             152    4 - #16161a
             496    4 - #597c95
             296    8 - #68686a
             296   16 - #68686a
             296   24 - #68686a
             296   32 - #68686a
             296   40 - #68686a
             296   48 - #68686a
             296   56 - #68686a
             296   64 - #68686a
             296   68 - #68686a
             296   72 - #68686a
              48   88 - #16161a
             200   88 - #16161a
             124  148 - #16161a
               4  172 - #16161a
             592  180 - #597c95
             424  184 - #597c95
             244  188 - #16161a
             168  212 - #16161a
              80  232 - #16161a
             156  292 - #16161a
               8  296 - #16161a
             300  300 - #597c95
             504  388 - #597c95
               8  444 - #597c95
             360  448 - #597c95
             152  480 - #597c95
               4  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
            ",
        )?;

        from_main(|| {
            Theme::set_system(Theme::Light);
        });

        Ok(())
    }
}
