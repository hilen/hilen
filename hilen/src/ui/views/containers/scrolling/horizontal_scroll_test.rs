use anyhow::Result;

use crate::{
    self as hilen,
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    gm::color::{Color, WHITE},
    ui::{
        Label, ModifiersState, ScrollView, Setup, UIManager, ViewData, ViewFrame, ViewSubviews, ViewTest,
        view,
    },
    ui_test::{check_colors, checkpoint, inject_modifiers, inject_scroll, inject_scroll_x, inject_touches},
};

const COLUMNS: usize = 6;
const ROWS: usize = 3;
const TILE: f32 = 200.0;

// A scroll view over a grid of tiles three times its width. The wheel
// `delta.x`, Shift plus the wheel and a finger drag all move it sideways,
// the offset clamps at both ends, and the bar on the bottom edge is there
// only while the content is wider than the view.
#[view]
struct HorizontalScroll {
    #[init]
    status: Label,
    scroll: ScrollView,
}

impl Setup for HorizontalScroll {
    fn setup(mut self: Weak<Self>) {
        self.status.set_text_size(20).set_color(WHITE);
        self.status.place().tl(0).size(600, 60);

        self.scroll.set_color(WHITE);
        self.scroll.set_content_size((400, 600));
        self.scroll.place().t(80).l(50).size(400, 300);

        for row in 0..ROWS {
            for column in 0..COLUMNS {
                let tile = self.scroll.add_view::<Label>();
                tile.set_text(format!("{row} {column}")).set_text_size(40);
                tile.set_color(if (row + column).is_multiple_of(2) {
                    Color::hex("#dfe8f0")
                } else {
                    Color::hex("#a9bdd0")
                });
                tile.set_frame((column * 200, row * 200, TILE, TILE));
            }
        }
    }
}

impl HorizontalScroll {
    fn say(self: Weak<Self>, text: &'static str) {
        from_main(move || {
            self.status.set_text(text);
        });
        wait_for_next_frame();
    }

    /// The sideways and the vertical offset.
    fn offsets(self: Weak<Self>) -> (f32, f32) {
        from_main(move || {
            (
                self.scroll.content_offset_x(),
                self.scroll.get_scroll_content_offset(),
            )
        })
    }

    /// The bottom bar: hidden, its x and its length.
    fn bar(self: Weak<Self>) -> (bool, f32, f32) {
        from_main(move || {
            let bar = &self.scroll.subviews()[2];
            (bar.is_hidden(), bar.frame().origin.x, bar.frame().size.width)
        })
    }
}

impl ViewTest for HorizontalScroll {
    fn before_start() {
        UIManager::set_drag_scrolling(true);
    }

    fn perform_test(mut view: Weak<Self>) -> Result<()> {
        inject_touches("250 230 m");

        view.say("content as wide as the view, no bottom bar");
        inject_scroll_x(-100);
        assert_eq!(view.offsets(), (0.0, 0.0));
        assert!(view.bar().0, "a bottom bar over content that fits");
        check_colors(CHECK_1)?;

        from_main(move || {
            view.scroll.set_content_width(TILE * 6.0);
        });
        view.say("content 3 times wider, bottom bar at the left");
        // The track stops 6 points short of the vertical bar: 400 - 4 - 6,
        // and the view shows a third of the content.
        assert_eq!(view.bar(), (false, 2.0, 130.0));
        check_colors(CHECK_2)?;

        inject_scroll_x(-100);
        view.say("wheel sideways by 100, the tiles moved left");
        assert_eq!(view.offsets(), (-100.0, 0.0));
        let first_tile = from_main(move || view.scroll.content.subviews()[0].absolute_frame().origin.x);
        assert!((first_tile - (50.0 - 100.0)).abs() < 0.001);
        check_colors(CHECK_3)?;

        inject_modifiers(ModifiersState::SHIFT);
        inject_scroll(-100);
        inject_modifiers(ModifiersState::empty());
        view.say("Shift plus wheel, 100 more to the left, not down");
        assert_eq!(view.offsets(), (-200.0, 0.0));
        check_colors(CHECK_4)?;

        inject_scroll(-100);
        view.say("plain wheel, down by 100, sideways untouched");
        assert_eq!(view.offsets(), (-200.0, -100.0));
        check_colors(CHECK_5)?;

        inject_scroll_x(-5000);
        view.say("far past the end, clamped at the right edge, bar at the right");
        assert_eq!(view.offsets(), (-800.0, -100.0));
        assert_eq!(view.bar(), (false, 262.0, 130.0));
        check_colors(CHECK_6)?;

        from_main(move || {
            view.scroll.set_content_width(400);
        });
        wait_for_next_frame();
        view.say("content fits again, back at the left, no bottom bar");
        assert!(view.offsets().0.abs() < f32::EPSILON);
        assert!(view.bar().0, "the bottom bar stayed over content that fits");
        check_colors(CHECK_7)?;

        // The drag goes last and pins no colors. Its fling stops between
        // 2 pixels, and text there does not render the same on every run.
        from_main(move || {
            view.scroll.set_content_width(TILE * 6.0);
        });
        wait_for_next_frame();
        inject_scroll_x(-800);
        assert_eq!(view.offsets(), (-800.0, -100.0));

        // A finger drag to the right by 100 pulls the content with it and
        // leaves the vertical offset alone, the drag keeps its axis.
        inject_touches(
            "
            250 230 b
            350 240 m
        ",
        );
        assert_eq!(view.offsets(), (-700.0, -100.0));
        inject_touches("350 240 e");
        while from_main(move || view.scroll.is_scrolling()) {
            wait_for_next_frame();
        }
        view.say("dragged right with a finger, then the fling settled");
        let (after_fling, down) = view.offsets();
        assert!(after_fling > -700.0, "the sideways drag left no fling");
        assert!((down + 100.0).abs() < 0.001);
        checkpoint("dragged right with a finger, then the fling settled")?;

        Ok(())
    }
}

const CHECK_1: &str = r"
         592    4 - #ffffff
         292   24 - #d1d1d1
         392   24 - #454545
         116   28 - #ffffff
         328   28 - #5c5c5c
         392   28 - #454545
         140   32 - #929292
         192   32 - #303030
         236   32 - #ffffff
         284   32 - #858585
         332   32 - #000000
         364   32 - #e0e0e0
         432   32 - #ffffff
         452   32 - #2b2b2b
         480   32 - #161616
           4   56 - #ffffff
         368  168 - #000000
         128  180 - #74797d
         168  180 - #dfe8f0
         328  180 - #58636c
         336  180 - #a9bdd0
         368  188 - #000000
         248  276 - #dfe8f0
         148  280 - #a9bdd0
         448  280 - #dfe8f0
          52  296 - #a9bdd0
         132  368 - #000000
         332  368 - #000000
         364  372 - #dfe8f0
         164  376 - #a9bdd0
           4  592 - #597c95
         592  592 - #597c95
";

const CHECK_2: &str = r"
           4    4 - #ffffff
         208   28 - #000000
         468   28 - #c2c2c2
         112   32 - #ffffff
         164   32 - #131313
         268   32 - #c2c2c2
         332   32 - #797979
         356   32 - #d3d3d3
         392   32 - #ffffff
         436   32 - #ffffff
         484   32 - #a1a1a1
         592   56 - #ffffff
         368  168 - #000000
         132  180 - #dfe8f0
         160  180 - #1b1c1d
         168  180 - #dfe8f0
         324  180 - #8fa0b0
         328  180 - #58636c
         368  188 - #000000
         352  272 - #a9bdd0
          52  276 - #dfe8f0
         448  280 - #dfe8f0
         232  288 - #a9bdd0
         332  368 - #000000
         136  372 - #1c1f22
         364  372 - #dfe8f0
         148  376 - #6e7b87
         176  376 - #6e7b87
         336  376 - #252628
           4  592 - #597c95
         256  592 - #597c95
         592  592 - #597c95
";

const CHECK_3: &str = r"
           4    4 - #ffffff
         592    4 - #ffffff
         256   24 - #515151
         308   28 - #ffffff
         364   28 - #434343
         472   28 - #ffffff
         132   32 - #000000
         188   32 - #ffffff
         284   32 - #6a6a6a
         356   32 - #d2d2d2
         400   32 - #d3d3d3
         416   32 - #010101
         488   32 - #929292
         140   80 - #dfe8f0
         444  104 - #91979c
         268  168 - #000000
          60  180 - #1b1c1d
         224  180 - #8fa0b0
         236  180 - #a9bdd0
         428  180 - #74797d
         432  180 - #dfe8f0
         268  188 - #000000
         140  264 - #dfe8f0
         340  268 - #a9bdd0
         448  360 - #a9bdd0
         268  368 - #000000
         260  372 - #dfe8f0
          68  376 - #a9bdd0
         168  376 - #91979c
         236  376 - #252628
         352  592 - #597c95
         592  592 - #597c95
";

const CHECK_4: &str = r"
         592    4 - #ffffff
         152   24 - #292929
         388   24 - #d2d2d2
         468   24 - #000000
         100   28 - #ffffff
         256   28 - #ffffff
         116   32 - #868686
         128   32 - #d2d2d2
         204   32 - #000000
         288   32 - #434343
         296   32 - #6b6b6b
         356   32 - #393939
         440   32 - #ffffff
         500   32 - #eaeaea
           4   56 - #ffffff
         252  120 - #dfe8f0
         444  144 - #91979c
         168  168 - #000000
         128  180 - #58636c
         136  180 - #a9bdd0
         324  180 - #bdc5cb
         372  192 - #000000
          52  276 - #a9bdd0
         228  280 - #dfe8f0
         448  280 - #a9bdd0
         332  368 - #000000
         164  372 - #dfe8f0
         368  372 - #a9bdd0
         124  376 - #91979c
         244  376 - #91979c
           4  592 - #597c95
         592  592 - #597c95
";

const CHECK_5: &str = r"
           4    4 - #ffffff
         288   24 - #000000
         476   24 - #6b6b6b
         504   24 - #262626
         108   28 - #a9a9a9
         192   28 - #dcdcdc
         336   28 - #535353
         424   28 - #7a7a7a
         100   32 - #ffffff
         164   32 - #797979
         180   32 - #000000
         212   32 - #8c8c8c
         248   32 - #2c2c2c
         308   32 - #ffffff
         336   32 - #535353
         388   32 - #ffffff
         592   56 - #ffffff
         132   80 - #a9bdd0
         336   80 - #dfe8f0
         136   88 - #a9bdd0
         332   88 - #dfe8f0
         372   92 - #000000
         444  172 - #91979c
         228  184 - #dfe8f0
          52  228 - #dfe8f0
         332  268 - #000000
         136  280 - #252628
         336  288 - #1c1f22
         372  292 - #000000
         120  376 - #91979c
           4  592 - #597c95
         592  592 - #597c95
";

const CHECK_6: &str = r"
           4    4 - #ffffff
         428   24 - #5e5e5e
         236   28 - #c4c4c4
         524   28 - #9f9f9f
          76   32 - #ffffff
         120   32 - #797979
         168   32 - #464646
         216   32 - #969696
         288   32 - #c5c5c5
         312   32 - #e9e9e9
         332   32 - #d1d1d1
         380   32 - #f9f9f9
         452   32 - #b7b7b7
         548   32 - #1d1d1d
          24   56 - #ffffff
         164   80 - #dfe8f0
         332   80 - #a9bdd0
         368   92 - #000000
         248  172 - #dfe8f0
          52  176 - #dfe8f0
         444  184 - #91979c
         172  268 - #414950
         332  268 - #000000
         372  268 - #595d60
         172  276 - #414950
         164  284 - #17191c
         136  288 - #1c1f22
          52  376 - #a9bdd0
         248  376 - #a9bdd0
         436  376 - #91979c
         244  592 - #597c95
         592  592 - #597c95
";

const CHECK_7: &str = r"
           4    4 - #ffffff
         592    4 - #ffffff
         108   28 - #2e2e2e
         180   28 - #ffffff
         132   32 - #f9f9f9
         144   32 - #505050
         172   32 - #797979
         236   32 - #2c2c2c
         268   32 - #161616
         308   32 - #d2d2d2
         320   32 - #525252
         332   32 - #c5c5c5
         392   32 - #848484
         492   32 - #ffffff
         132   80 - #dfe8f0
         336   80 - #a9bdd0
         332   88 - #a9bdd0
         168   92 - #000000
          52  180 - #a9bdd0
         444  184 - #91979c
         444  264 - #91979c
         136  268 - #1c1f22
         332  268 - #000000
         368  268 - #000000
         160  280 - #151719
         336  288 - #252628
         368  288 - #000000
          52  376 - #a9bdd0
         252  376 - #dfe8f0
         448  376 - #dfe8f0
           4  592 - #597c95
         592  592 - #597c95
";
