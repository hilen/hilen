use anyhow::Result;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    gm::color::{GRAY, LIGHT_BLUE},
    ui::{Button, Color, Label, Setup, VSplitView, ViewData, ViewFrame, ViewSubviews, ViewTest, view},
    ui_test::{check_colors, inject_touches},
};

/// A top and a bottom panel with a divider that is dragged up and down.
/// The status line in the top panel says how tall the bottom panel is on
/// screen, how many drags ended and how many taps the button took.
#[view]
struct VSplitViewTest {
    taps:    u32,
    resizes: u32,
    button:  Weak<Button>,
    status:  Weak<Label>,

    #[init]
    split: VSplitView,
}

impl Setup for VSplitViewTest {
    fn setup(mut self: Weak<Self>) {
        self.split.place().back();
        self.split.set_min_heights(200.0, 100.0);
        self.split.set_bottom_height(150.0);
        self.split.set_divider_colors(GRAY, LIGHT_BLUE);
        self.split.resized.sub(move || {
            self.resizes += 1;
            self.show_status();
        });

        self.split.top.set_color(Color::hex("#DCE6FB"));
        self.split.bottom.set_color(Color::hex("#FBE9D0"));

        let top = self.split.top.add_view::<Label>();
        top.set_text("top panel").set_text_size(20);
        top.place().tl(10).size(300, 30);

        self.status = self.split.top.add_view();
        self.status.set_text_size(20);
        self.status.place().t(50).l(10).size(500, 30);

        let bottom = self.split.bottom.add_view::<Label>();
        bottom.set_text("bottom panel").set_text_size(20);
        bottom.place().bl(10).size(300, 30);

        // Pinned to the top edge of the bottom panel, so it always sits
        // under the divider's grab zone overlap.
        self.button = self.split.bottom.add_view();
        self.button.set_text("button").set_text_size(20);
        self.button.place().t(0).r(60).size(120, 40);
        self.button.on_tap(move || {
            self.taps += 1;
            self.show_status();
        });

        self.split.bottom.size_changed().sub(move || self.show_status());
        self.show_status();
    }
}

impl VSplitViewTest {
    fn show_status(self: Weak<Self>) {
        self.status.set_text(format!(
            "bottom {}, drags {}, taps {}",
            self.split.bottom.height(),
            self.resizes,
            self.taps
        ));
    }
}

impl ViewTest for VSplitViewTest {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let state = move || {
            from_main(move || {
                (
                    view.split.bottom_height(),
                    view.split.top.height(),
                    view.taps,
                    view.resizes,
                )
            })
        };

        // A desktop lights the divider under a pointer and a phone has no
        // pointer. So the pointer rests on the top panel at every check,
        // and the look is the same on both.
        let rest = || inject_touches("300 120 m");

        assert_eq!(state(), (150.0, 450.0, 0, 0));
        check_colors(COLORS_START)?;

        // A drag up follows the cursor and makes the bottom panel taller.
        inject_touches(
            "
            300 450 b
            300 400 m
        ",
        );
        assert_eq!(state(), (200.0, 400.0, 0, 0));

        // A wild move keeps the capture and clamps at the top minimum:
        // 600 - 200.
        inject_touches("300 50 m");
        assert_eq!(state(), (400.0, 200.0, 0, 0));

        // Coming back never jumps, deltas count from the drag start.
        inject_touches(
            "
            300 420 m
            300 420 e
        ",
        );
        assert_eq!(state(), (180.0, 420.0, 0, 1));
        rest();
        check_colors(COLORS_DRAGGED)?;

        // A drag down stops at the bottom minimum. A release over another
        // touch enabled view still ends the drag on the divider: no tap
        // on that view, and later bare cursor moves must not keep
        // resizing.
        inject_touches(
            "
            300 420 b
            300 590 m
            480 520 e
        ",
        );
        assert_eq!(state(), (100.0, 500.0, 0, 2));
        inject_touches("300 300 m");
        assert_eq!(state(), (100.0, 500.0, 0, 2));

        // Inside its grab zone the divider beats the panel content under
        // it, even content registered after the divider.
        inject_touches(
            "
            480 502 b
            480 502 e
        ",
        );
        assert_eq!(state(), (100.0, 500.0, 0, 3));

        // Away from the zone the button taps normally.
        inject_touches(
            "
            480 525 b
            480 525 e
        ",
        );
        assert_eq!(state(), (100.0, 500.0, 1, 3));
        rest();
        check_colors(COLORS_SMALLEST)?;

        // A hidden panel gives its room to the top one and keeps its
        // height for when it comes back.
        from_main(move || view.split.set_bottom_hidden(true));
        assert_eq!(from_main(move || view.split.top.height()), 600.0);
        check_colors(COLORS_HIDDEN)?;

        from_main(move || view.split.set_bottom_hidden(false));
        assert_eq!(state(), (100.0, 500.0, 1, 3));

        // A height the view has no room for shows clamped at the top
        // minimum, and a drag starts from what is on screen.
        from_main(move || view.split.set_bottom_height(1000.0));
        assert_eq!(state(), (1000.0, 200.0, 1, 3));
        inject_touches(
            "
            300 200 b
            300 250 m
            300 250 e
        ",
        );
        assert_eq!(state(), (350.0, 250.0, 1, 4));
        rest();
        check_colors(COLORS_TALL)?;

        Ok(())
    }
}

const COLORS_START: &str = r"
 412    4 - #dce6fb
 592    4 - #dce6fb
  48   24 - #dce6fb
  84   28 - #000000
 200   64 - #dce6fb
 244   64 - #000000
  32   68 - #dce6fb
  88   68 - #000000
 144   68 - #dce6fb
 172   68 - #dce6fb
 448  212 - #dce6fb
 196  260 - #dce6fb
   4  300 - #dce6fb
  84  452 - #fbe9d0
 232  452 - #fbe9d0
 424  452 - #ffffff
 536  452 - #ffffff
 464  468 - #000000
 456  472 - #ffffff
 492  472 - #ffffff
 420  488 - #ffffff
 528  488 - #ffffff
  28  568 - #000000
  32  576 - #fbe9d0
  44  576 - #fbe9d0
  60  576 - #fbe9d0
  68  576 - #fbe9d0
 100  576 - #fbe9d0
 136  576 - #000000
 304  592 - #fbe9d0
 472  592 - #fbe9d0
 592  592 - #fbe9d0
";

const COLORS_DRAGGED: &str = r"
 412    4 - #dce6fb
 592    4 - #dce6fb
  48   24 - #dce6fb
  84   28 - #000000
 172   64 - #dce6fb
 244   64 - #000000
  32   68 - #dce6fb
  88   68 - #010101
 144   68 - #dce6fb
 444  200 - #dce6fb
 144  256 - #dce6fb
  60  420 - #bcbcbc
 140  420 - #bcbcbc
 320  420 - #bcbcbc
 592  420 - #bcbcbc
 220  424 - #fbe9d0
 420  424 - #ffffff
 536  424 - #ffffff
 456  440 - #ffffff
 460  440 - #010101
 492  440 - #ffffff
 424  456 - #ffffff
 524  456 - #ffffff
  28  568 - #000000
  32  576 - #fbe9d0
  44  576 - #fbe9d0
  68  576 - #fbe9d0
 100  576 - #fbe9d0
 136  576 - #000000
 460  576 - #fbe9d0
 328  592 - #fbe9d0
 592  592 - #fbe9d0
";

const COLORS_SMALLEST: &str = r"
 440    4 - #dce6fb
  84   24 - #000000
  48   28 - #dce6fb
  32   64 - #dce6fb
 108   64 - #000000
 172   64 - #dce6fb
  88   68 - #000000
 144   68 - #dce6fb
 232   72 - #101012
 592  160 - #dce6fb
 380  264 - #dce6fb
 148  304 - #dce6fb
   4  500 - #bcbcbc
  84  500 - #bcbcbc
 180  500 - #bcbcbc
 352  500 - #bcbcbc
 288  504 - #fbe9d0
 420  504 - #ffffff
 536  504 - #ffffff
 456  520 - #ffffff
 492  520 - #ffffff
 424  536 - #ffffff
 524  536 - #ffffff
  28  568 - #000000
  32  576 - #fbe9d0
  44  576 - #fbe9d0
  60  576 - #fbe9d0
 100  576 - #fbe9d0
 136  576 - #000000
 236  592 - #fbe9d0
 352  592 - #fbe9d0
 592  592 - #fbe9d0
";

const COLORS_HIDDEN: &str = r"
 468    4 - #dce6fb
  28   24 - #050506
  64   24 - #dce6fb
  76   24 - #dce6fb
  48   28 - #dce6fb
  84   28 - #000000
 100   28 - #dce6fb
  28   60 - #000000
 216   60 - #000000
 108   64 - #000000
 144   64 - #dce6fb
 172   64 - #dce6fb
 224   64 - #dce6fb
 244   64 - #000000
  28   68 - #000000
  88   68 - #000000
 112   68 - #dce6fb
 124   68 - #dce6fb
 160   68 - #dce6fb
 216   68 - #000000
 228   68 - #000000
 236   68 - #dce6fb
 232   72 - #101012
 592  208 - #dce6fb
 384  216 - #dce6fb
  48  332 - #dce6fb
 508  400 - #dce6fb
 300  408 - #dce6fb
 400  576 - #dce6fb
   4  592 - #dce6fb
 208  592 - #dce6fb
 592  592 - #dce6fb
";

const COLORS_TALL: &str = r"
 340    4 - #dce6fb
 592    4 - #dce6fb
  48   24 - #dce6fb
  76   24 - #dce6fb
 100   28 - #dce6fb
 200   64 - #16171a
 244   64 - #000000
  28   68 - #000000
 112   68 - #dce6fb
 160   68 - #dce6fb
 464  100 - #dce6fb
 280  208 - #dce6fb
   4  248 - #dce6fb
 420  252 - #ffffff
 536  252 - #ffffff
 456  272 - #ffffff
 460  272 - #010101
 492  272 - #ffffff
 420  288 - #ffffff
 528  288 - #ffffff
 192  328 - #fbe9d0
  48  404 - #fbe9d0
 496  452 - #fbe9d0
 348  528 - #fbe9d0
  28  568 - #000000
  32  576 - #fbe9d0
  44  576 - #fbe9d0
  60  576 - #fbe9d0
  68  576 - #fbe9d0
 100  576 - #fbe9d0
 136  576 - #000000
 592  592 - #fbe9d0
";
