use std::ops::Deref;

use anyhow::Result;
use parking_lot::Mutex;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    gm::color::{Color, WHITE},
    ui::{
        Button, CellRegistry, Container, Label, Setup, TableData, TableView, View, ViewData, ViewFrame,
        ViewSubviews, ViewTest, view,
    },
    ui_test::{check_colors, inject_touches, inject_trackpad_scroll},
};

static SELECTED: Mutex<String> = Mutex::new(String::new());

const NATURAL: &str = r"
               4    4 - #6a4c93
             340   44 - #9f8bb9
             240   52 - #6a4c93
             268   56 - #836aa5
             520  124 - #000000
              68  128 - #1d3557
             168  128 - #50627d
             552  128 - #e63946
             136  132 - #8693a5
             520  132 - #4e1318
             548  144 - #010000
             588  148 - #e63946
             316  156 - #1d3557
              84  220 - #f1faee
             300  220 - #293137
             496  220 - #20262c
             404  224 - #f1faee
             156  276 - #597c95
              12  292 - #a8dadc
             500  324 - #75969a
             516  324 - #86adb0
             292  328 - #f1faee
              88  332 - #a8dadc
             320  432 - #6f8f93
             104  436 - #bfc7c0
             292  440 - #6b8a8d
             496  444 - #20262c
             196  504 - #81abb8
             300  544 - #d4ddd3
             300  548 - #d4ddd3
              84  552 - #a8dadc
             520  552 - #20262c
            ";

const STUCK: &str = r"
             588   12 - #e63946
             496   20 - #e63946
              68   28 - #1d3557
             168   28 - #50627d
             500   28 - #000000
             528   32 - #290a0d
             528   36 - #000000
             552   36 - #e63946
             576   36 - #010000
             528   44 - #300c0f
             332   56 - #1d3557
             520  144 - #20262c
              12  160 - #a8dadc
             196  160 - #81abb8
             384  180 - #f1faee
             116  256 - #20262c
             496  256 - #42484b
             276  264 - #a8dadc
              12  328 - #a8dadc
             128  364 - #86adb0
             312  368 - #21272d
              96  372 - #90babd
             508  372 - #749598
             192  432 - #f1faee
             324  476 - #a8dadc
             496  476 - #42484b
              80  480 - #f1faee
             496  484 - #42484b
             204  548 - #597c95
              96  588 - #90babd
             316  592 - #f1faee
             592  592 - #597c95
            ";

const BACK: &str = r"
               4    4 - #6a4c93
             268    4 - #836aa5
             276    4 - #c6bbd6
             400   52 - #1d3557
             544   60 - #e63946
             500   72 - #000000
             168   76 - #50627d
             176   80 - #607189
             516   80 - #96252e
             576   80 - #010000
              56   84 - #1d3557
             496   88 - #e63946
             528   92 - #300c0f
             348  140 - #a8dadc
             120  172 - #f1faee
             300  172 - #293137
             516  176 - #4a5053
             516  272 - #6f8f93
             104  276 - #87afb2
             284  280 - #f1faee
             500  284 - #75969a
             104  384 - #bfc7c0
             304  384 - #20262c
             500  388 - #a2aaa5
             204  408 - #597c95
             508  496 - #749598
              84  500 - #a8dadc
             472  500 - #a8dadc
             108  504 - #20262c
             520  504 - #20262c
             196  516 - #81abb8
             300  548 - #f1faee
            ";

const PLAIN_H: f32 = 100.0;
const STICKY_H: f32 = 60.0;
const SPACING: f32 = 10.0;
const ROW_H: f32 = 100.0;

// A grid of 3 columns under 2 header views, a plain one and a sticky one
// under it, like a strip of posters over a row of tabs. Scrolled down,
// the sticky header stays at the top as wide as the table, over the
// rows. A tap on it selects no cell under it and its button still takes
// taps. Scrolled back up it goes back to its place under the plain one.
#[view]
struct TableStickyHeader {
    plain:  Weak<Label>,
    sticky: Weak<Container>,
    button: Weak<Button>,
    taps:   usize,

    #[init]
    table: TableView,
}

impl Setup for TableStickyHeader {
    fn setup(mut self: Weak<Self>) {
        self.table.place().tl(0).size(600, 600);
        self.table
            .set_data_source(self)
            .register_cell::<Label>()
            .set_columns(3)
            .set_cell_spacing(SPACING)
            .set_cell_margins(SPACING, SPACING)
            .set_header_height(PLAIN_H + STICKY_H + SPACING);

        self.plain = self.table.add_header_view::<Label>();
        self.plain
            .set_text("plain header")
            .set_text_size(24)
            .set_text_color(WHITE)
            .set_color(Color::hex("#6a4c93"));
        self.plain.place().t(0).lr(0).h(PLAIN_H);

        self.sticky = self.table.add_sticky_header_view::<Container>();
        self.sticky.set_color(Color::hex("#1d3557"));
        self.table.set_sticky_header_place(PLAIN_H, STICKY_H);

        let label = self.sticky.add_view::<Label>();
        label
            .set_text("sticky header")
            .set_text_size(24)
            .set_text_color(WHITE)
            .set_color(Color::hex("#1d3557"));
        label.place().l(10).center_y().size(220, 40);

        self.button = self.sticky.add_view::<Button>();
        self.button.set_text("tap").set_color(Color::hex("#e63946"));
        self.button.place().r(10).center_y().size(100, 40);
        self.button.on_tap(move || {
            self.taps += 1;
            let taps = self.taps;
            self.button.set_text(format!("taps {taps}"));
        });

        self.table.reload_data();
    }
}

impl TableData for TableStickyHeader {
    fn cell_height(&self, _: usize) -> f32 {
        ROW_H
    }

    fn number_of_cells(&self) -> usize {
        60
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let cell = registry.cell::<Label>();
        cell.set_text(format!("cell {index}"));
        cell.set_text_size(20);
        cell.set_text_color(Color::hex("#20262c"));
        cell.set_color(if index.is_multiple_of(2) {
            Color::hex("#f1faee")
        } else {
            Color::hex("#a8dadc")
        });
        cell
    }

    fn cell_selected(&mut self, index: usize) {
        *SELECTED.lock() += &format!("|{index}|");
    }
}

impl ViewTest for TableStickyHeader {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // Unscrolled, the sticky header sits under the plain one, as wide
        // as the table. A tap on it selects nothing, a tap on the first
        // row selects cell 0.
        from_main(move || {
            assert_eq!(view.sticky.frame().origin.y, PLAIN_H);
            assert_eq!(view.sticky.width(), 600.0);
        });
        inject_touches(
            "
                100  130  b
                100  130  e
                100  220  b
                100  220  e
            ",
        );
        assert_eq!(SELECTED.lock().deref(), "|0|");
        SELECTED.lock().clear();
        check_colors(NATURAL)?;

        // A real scroll of 400 points. The plain header is gone and the
        // sticky one stays at the top of the table, over row 2.
        inject_trackpad_scroll(-400);
        from_main(move || {
            assert_eq!(view.table.content_offset(), -400.0);
            assert_eq!(view.sticky.frame().origin.y, 400.0);
        });
        // The tap on the sticky header selects no cell under it, its
        // button counts the tap, and a tap under it selects cell 9, the
        // first cell of row 3, which starts 100 points under the table
        // top.
        inject_touches(
            "
                100  30   b
                100  30   e
                540  30   b
                540  30   e
                100  120  b
                100  120  e
            ",
        );
        assert_eq!(SELECTED.lock().deref(), "|9|");
        SELECTED.lock().clear();
        from_main(move || assert_eq!(view.taps, 1));
        check_colors(STUCK)?;

        // Scrolled back up to 50 points from the top, the sticky header is
        // at its place again, 50 points under the top of the table.
        inject_trackpad_scroll(350);
        from_main(move || {
            assert_eq!(view.table.content_offset(), -50.0);
            assert_eq!(view.sticky.frame().origin.y, PLAIN_H);
        });
        check_colors(BACK)?;

        Ok(())
    }
}
