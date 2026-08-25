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
             152    4 - #0000e7
             528    4 - #597c95
             296    8 - #000096
             224   20 - #0000e7
              80   28 - #0000e7
             296   32 - #000096
             296   56 - #a6a600
             296   68 - #a6a600
             296   80 - #a6a600
              40   92 - #ffff00
             196   92 - #ffff00
             296  100 - #000096
             296  116 - #000096
             296  128 - #000096
             296  144 - #000096
             116  148 - #0000e7
             296  156 - #a6a600
             296  168 - #a6a600
             296  176 - #a6a600
               4  180 - #ffff00
             200  200 - #0000e7
             592  228 - #597c95
              80  236 - #0000e7
             232  268 - #ffff00
             156  288 - #ffff00
               8  296 - #ffff00
             300  300 - #597c95
             500  408 - #597c95
             348  544 - #597c95
             100  592 - #597c95
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
             132    4 - #0000e7
             592    4 - #597c95
              68   40 - #0000e7
             252   52 - #ffff00
             112   84 - #ffff00
             172   92 - #ffff00
              24  100 - #0000e7
             296  124 - #000096
             104  148 - #0000e7
             296  148 - #000096
             296  160 - #a6a600
             296  172 - #a6a600
             296  184 - #a6a600
               4  188 - #ffff00
             196  188 - #ffff00
             296  196 - #a6a600
             296  224 - #000096
             296  240 - #000096
              80  252 - #00ff00
             296  252 - #00a600
             224  260 - #00ff00
             296  264 - #00a600
             296  276 - #00a600
             156  288 - #00ff00
             296  288 - #00a600
               8  296 - #00ff00
             104  296 - #00ff00
             300  300 - #597c95
             592  300 - #597c95
             100  592 - #597c95
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
