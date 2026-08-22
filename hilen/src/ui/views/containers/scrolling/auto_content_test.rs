use anyhow::Result;

use crate::{
    self as hilen,
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    gm::{
        LossyConvert,
        color::{BLUE, GREEN, YELLOW},
    },
    ui::{Container, ScrollView, Setup, ViewData, ViewSubviews, ViewTest, view},
    ui_test::{check_colors, inject_scroll, inject_touches},
};

/// Covers the automatic content size: a scroll view whose content size
/// was never set gets its width from the viewport and its height from
/// the lowest subview edge, hidden subviews not counting.
#[view]
struct AutoContentTest {
    #[init]
    scroll: ScrollView,
}

impl Setup for AutoContentTest {
    fn setup(self: Weak<Self>) {
        self.scroll.place().tl(0).size(300, 300);

        // The last row gets its own color, so the bottom clamped state
        // is visually distinct from the top in a repeating stripe
        // pattern.
        for i in 0..10_u32 {
            let row = self.scroll.add_view::<Container>();
            row.set_color(if i == 9 {
                GREEN
            } else if i.is_multiple_of(2) {
                BLUE
            } else {
                YELLOW
            });
            row.place().t(50.0 * i.lossy_convert()).lr(0).h(50);
        }

        let hidden = self.scroll.add_view::<Container>();
        hidden.place().t(500).lr(0).h(300);
        hidden.set_hidden(true);
    }
}

impl ViewTest for AutoContentTest {
    fn perform_test(mut view: Weak<Self>) -> Result<()> {
        wait_for_next_frame();
        wait_for_next_frame();

        from_main(move || {
            assert_eq!(view.scroll.content.content_size, (300, 500).into());
            assert!(view.scroll.get_scroll_content_offset().abs() < f32::EPSILON);
        });

        check_colors(
            r"
               4    4 - #0000e7
             220    4 - #0000e7
             592    4 - #597c95
             296    8 - #0000e7
             444    8 - #597c95
             148   12 - #0000e7
              76   16 - #0000e7
              20   76 - #ffff00
             160   80 - #ffff00
             232   84 - #ffff00
              92   88 - #ffff00
              16  148 - #0000e7
             480  152 - #597c95
              84  156 - #ffff00
             160  156 - #ffff00
             296  164 - #ffff00
             228  168 - #ffff00
               4  220 - #0000e7
             168  224 - #0000e7
              88  228 - #0000e7
             296  228 - #0000e7
             236  236 - #0000e7
              12  296 - #ffff00
             164  296 - #ffff00
             592  296 - #597c95
             304  300 - #597c95
             488  444 - #597c95
               4  456 - #597c95
             200  460 - #597c95
             340  552 - #597c95
              88  592 - #597c95
             592  592 - #597c95
            ",
        )?;

        // Way past the content: the offset clamps so the last row's
        // bottom lands exactly on the viewport bottom.
        inject_touches("150 150 m");
        inject_scroll(-1000);

        from_main(move || {
            assert!((view.scroll.get_scroll_content_offset() + 200.0).abs() < f32::EPSILON);
        });

        check_colors(
            r"
               4    4 - #0000e7
             136    4 - #0000e7
             440    4 - #597c95
             204    8 - #0000e7
              68   16 - #0000e7
             256   52 - #ffff00
             164   56 - #ffff00
             104   72 - #ffff00
              32   92 - #ffff00
             188  108 - #0000e7
             592  116 - #597c95
             296  124 - #0000e7
             108  148 - #0000e7
               4  184 - #ffff00
             236  184 - #ffff00
             296  192 - #ffff00
             164  196 - #ffff00
              84  236 - #0000e7
              36  252 - #00ff00
             132  252 - #00ff00
             180  252 - #00ff00
             280  252 - #00ff00
             228  268 - #00ff00
              12  296 - #00ff00
              76  296 - #00ff00
             152  296 - #00ff00
             304  300 - #597c95
             540  352 - #597c95
             404  504 - #597c95
               4  524 - #597c95
             220  592 - #597c95
             592  592 - #597c95
            ",
        )?;

        // A manual height wins over the automatic one and stays.
        from_main(move || {
            view.scroll.set_content_height(1000);
        });

        wait_for_next_frame();
        wait_for_next_frame();

        from_main(move || {
            assert!((view.scroll.content.content_size.height - 1000.0).abs() < f32::EPSILON);
        });

        Ok(())
    }
}
