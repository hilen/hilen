use anyhow::Result;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    gm::color::{BLUE, GRAY, GREEN, LIGHT_BLUE, RED},
    ui::{Button, Setup, SplitView, ViewData, ViewFrame, ViewSubviews, ViewTest, view},
    ui_test::{check_colors, inject_touches},
};

#[view]
struct SplitViewTest {
    taps:    u32,
    resizes: u32,
    button:  Weak<Button>,

    #[init]
    split: SplitView,
}

impl Setup for SplitViewTest {
    fn setup(mut self: Weak<Self>) {
        self.split.place().back();
        self.split.set_min_widths(100.0, 200.0, 100.0);
        self.split.set_left_width(150.0);
        self.split.set_right_width(150.0);
        self.split.set_divider_colors(GRAY, LIGHT_BLUE);
        self.split.resized.sub(move || self.resizes += 1);

        self.split.left.set_color(RED);
        self.split.center.set_color(GREEN);
        self.split.right.set_color(BLUE);

        // Pinned to the left panel's edge, so it always sits under the
        // divider's grab zone overlap.
        self.button = self.split.left.add_view();
        self.button.place().r(0).t(100).size(60, 40);
        self.button.on_tap(move || self.taps += 1);
    }
}

impl ViewTest for SplitViewTest {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let state = move || {
            from_main(move || {
                (
                    view.split.left_width(),
                    view.split.right_width(),
                    view.taps,
                    view.resizes,
                )
            })
        };

        assert_eq!(state(), (150.0, 150.0, 0, 0));
        check_colors(COLORS_START)?;

        // A drag follows the cursor.
        inject_touches(
            "
            150 300 b
            200 300 m
        ",
        );
        assert_eq!(state(), (200.0, 150.0, 0, 0));

        // A wild move keeps the capture and clamps at the center minimum:
        // 600 - 150 - 200.
        inject_touches("500 300 m");
        assert_eq!(state(), (250.0, 150.0, 0, 0));

        // Coming back never jumps, deltas count from the drag start.
        inject_touches(
            "
            220 300 m
            220 300 e
        ",
        );
        assert_eq!(state(), (220.0, 150.0, 0, 1));

        // A release over another touch enabled view still ends the drag on
        // the divider: no tap on that view, and later bare cursor moves
        // must not keep resizing.
        inject_touches(
            "
            220 300 b
            100 300 m
             60 120 e
        ",
        );
        assert_eq!(state(), (100.0, 150.0, 0, 2));
        inject_touches("400 300 m");
        assert_eq!(state(), (100.0, 150.0, 0, 2));

        // Inside its grab zone the divider beats the panel content under
        // it, even content registered after the divider.
        inject_touches(
            "
             98 120 b
             98 120 e
        ",
        );
        assert_eq!(state(), (100.0, 150.0, 0, 3));

        // Away from the zone the button taps normally.
        inject_touches(
            "
             70 120 b
             70 120 e
        ",
        );
        assert_eq!(state(), (100.0, 150.0, 1, 3));

        // The right divider drags the other way.
        inject_touches(
            "
            450 300 b
            350 300 m
            350 300 e
        ",
        );
        assert_eq!(state(), (100.0, 250.0, 1, 4));
        check_colors(COLORS_DRAGGED)?;

        // A hidden panel gives its room to the center and keeps its width
        // for when it comes back.
        from_main(move || view.split.set_left_hidden(true));
        let center = from_main(move || (view.split.center.x(), view.split.center.width()));
        assert_eq!(center, (0.0, 350.0));

        from_main(move || view.split.set_left_hidden(false));
        let center = from_main(move || (view.split.center.x(), view.split.center.width()));
        assert_eq!(center, (100.0, 250.0));
        check_colors(COLORS_RESTORED)?;

        Ok(())
    }
}

const COLORS_START: &str = r"
   4    4 - #ff0000
 112    4 - #ff0000
 220    4 - #00ff00
 592    4 - #0000e7
 352    8 - #00ff00
 472   48 - #0000e7
  92  100 - #ffffff
 132  100 - #ffffff
 112  108 - #ffffff
 148  116 - #ffffff
  96  120 - #ffffff
 128  124 - #ffffff
 108  136 - #ffffff
 148  136 - #ffffff
 300  148 - #00ff00
 592  148 - #0000e7
 452  172 - #0000e7
   4  224 - #ff0000
 132  284 - #ff0000
 300  300 - #00ff00
 592  300 - #0000e7
   4  364 - #ff0000
 448  376 - #00ff00
 280  428 - #00ff00
 148  432 - #ff0000
 576  444 - #0000e7
  56  448 - #ff0000
 372  520 - #00ff00
   4  528 - #ff0000
 472  588 - #0000e7
 152  592 - #00ff00
 592  592 - #0000e7
";

const COLORS_DRAGGED: &str = r"
   4    4 - #ff0000
 100    4 - #bcbcbc
 592    4 - #0000e7
 328    8 - #00ff00
 212   40 - #00ff00
  80  100 - #ffffff
  40  104 - #ffffff
  60  112 - #ffffff
  40  128 - #ffffff
  60  136 - #ffffff
  96  136 - #ffffff
 256  148 - #00ff00
 592  148 - #0000e7
 452  160 - #0000e7
 100  204 - #bcbcbc
   4  212 - #ff0000
 100  272 - #bcbcbc
 300  300 - #00ff00
 592  300 - #0000e7
 100  352 - #bcbcbc
   4  364 - #ff0000
 448  400 - #0000e7
 100  432 - #bcbcbc
 248  436 - #00ff00
 584  448 - #0000e7
   4  500 - #ff0000
  88  512 - #ff0000
 348  544 - #00ff00
   8  592 - #ff0000
 100  592 - #bcbcbc
 228  592 - #00ff00
 592  592 - #0000e7
";

const COLORS_RESTORED: &str = COLORS_DRAGGED;
