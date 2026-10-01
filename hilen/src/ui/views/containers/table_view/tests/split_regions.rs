use anyhow::Result;

use crate::{
    self as hilen,
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    gm::color::{Color, WHITE},
    ui::{
        CellRegistry, Label, Setup, TableData, TableView, TextAlignment, UIManager, View, ViewData,
        ViewFrame, ViewSubviews, ViewTest, view,
    },
    ui_test::{check_colors, checkpoint, inject_scroll_x, inject_touches},
};

const NUMBER_WIDTH: f32 = 40.0;
const HALF: f32 = 300.0;
const CODE_WIDTH: f32 = 700.0;
const ROW: f32 = 50.0;

/// Cuts the moving text of one side at the edges of that side.
#[view]
struct SideClip {}

impl Setup for SideClip {
    fn clips_to_bounds(&self) -> bool {
        true
    }

    fn setup(self: Weak<Self>) {}
}

/// One row of a side by side diff: 2 regions, each a pinned line number
/// and a text that moves sideways inside its own half.
#[view]
struct SplitCell {
    old_code: Weak<Label>,
    new_code: Weak<Label>,

    #[init]
    old_number: Label,
    old_clip:   SideClip,
    new_number: Label,
    new_clip:   SideClip,
}

impl Setup for SplitCell {
    fn setup(mut self: Weak<Self>) {
        for (number, x) in [(self.old_number, 0.0), (self.new_number, HALF)] {
            number.set_text_size(16).set_color(Color::hex("#39424e")).set_text_color(WHITE);
            number.place().l(x).t(0).b(0).w(NUMBER_WIDTH);
        }

        self.old_clip.place().l(NUMBER_WIDTH).t(0).b(0).w(HALF - NUMBER_WIDTH);
        self.new_clip.place().l(HALF + NUMBER_WIDTH).t(0).b(0).w(HALF - NUMBER_WIDTH);

        self.old_code = self.old_clip.add_view();
        self.new_code = self.new_clip.add_view();

        for code in [self.old_code, self.new_code] {
            code.set_text_size(16).set_alignment(TextAlignment::Left);
            code.set_moves_sideways(true);
            code.place().l(0).t(0).b(0).w(CODE_WIDTH);
        }
    }
}

impl SplitCell {
    fn set_row(self: Weak<Self>, index: usize) {
        self.old_number.set_text(index);
        self.new_number.set_text(index);
        self.old_code.set_text(format!("old {index}, at 100 200 300 400 500 600 end"));
        self.new_code.set_text(format!("new {index}, at 100 200 300 400 500 600 end"));
        self.old_code.set_color(Color::hex("#f6dcdc"));
        self.new_code.set_color(Color::hex("#dcf2dc"));
    }
}

// A cell with 2 side by side regions. Both texts move by the one sideways
// offset of the table, each line number stays at the left of its half, and
// each text is cut at the edges of its own half, so the old text never
// shows under the new side.
#[view]
struct TableSplitRegions {
    #[init]
    status: Label,
    table:  TableView,
}

impl Setup for TableSplitRegions {
    fn setup(mut self: Weak<Self>) {
        self.status.set_text_size(20).set_color(WHITE);
        self.status.place().t(0).l(0).size(600, 60);

        self.table.place().t(80).l(0).size(HALF * 2.0, 400);
        self.table.set_data_source(self).register_cell::<SplitCell>();
        // The text has 260 points of each half to show in, so the row
        // scrolls by 700 - 260 once the content is that much wider than
        // the table.
        self.table.set_content_width(HALF * 2.0 + CODE_WIDTH - (HALF - NUMBER_WIDTH));
        self.table.reload_data();
    }
}

impl TableData for TableSplitRegions {
    fn cell_height(&self, _: usize) -> f32 {
        ROW
    }

    fn number_of_cells(&self) -> usize {
        40
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let cell = registry.cell::<SplitCell>();
        cell.set_row(index);
        cell
    }

    fn cell_selected(&mut self, _: usize) {}
}

impl TableSplitRegions {
    fn say(self: Weak<Self>, text: &'static str) {
        from_main(move || {
            self.status.set_text(text);
        });
        wait_for_next_frame();
    }

    fn cell(self: Weak<Self>, index: usize) -> Weak<SplitCell> {
        self.table
            .visible_cells()
            .into_iter()
            .find(|(tag, _)| *tag == index)
            .unwrap_or_else(|| panic!("row {index} is not on screen"))
            .1
            .downcast::<SplitCell>()
            .expect("a cell of another type")
    }

    /// Left edges on screen of row `index`: old number, old text, new
    /// number, new text.
    fn row(self: Weak<Self>, index: usize) -> (f32, f32, f32, f32) {
        from_main(move || {
            let cell = self.cell(index);
            (
                cell.old_number.absolute_frame().origin.x,
                cell.old_code.absolute_frame().origin.x,
                cell.new_number.absolute_frame().origin.x,
                cell.new_code.absolute_frame().origin.x,
            )
        })
    }

    /// Whether the old and the new text of row 2 can be seen at `x`.
    fn seen_at(self: Weak<Self>, x: f32) -> (bool, bool) {
        from_main(move || {
            let cell = self.cell(2);
            let point = (x, 80.0 + ROW * 2.5).into();
            (
                cell.old_code.contains_visible(point),
                cell.new_code.contains_visible(point),
            )
        })
    }
}

impl ViewTest for TableSplitRegions {
    fn before_start() {
        UIManager::set_drag_scrolling(true);
    }

    fn perform_test(mut view: Weak<Self>) -> Result<()> {
        inject_touches("300 300 m");

        view.say("both sides at their start");
        assert_eq!(view.row(2), (0.0, NUMBER_WIDTH, HALF, HALF + NUMBER_WIDTH));
        check_colors(CHECK_1)?;

        inject_scroll_x(-150);
        view.say("wheel sideways by 150, both texts moved, numbers stayed");
        assert_eq!(
            view.row(2),
            (0.0, NUMBER_WIDTH - 150.0, HALF, HALF + NUMBER_WIDTH - 150.0)
        );
        // The old text is 700 wide and reaches under the new side, where
        // its own half cuts it. The new text is the one seen there.
        assert_eq!(view.seen_at(200.0), (true, false));
        assert_eq!(view.seen_at(HALF + 100.0), (false, true));
        check_colors(CHECK_2)?;

        from_main(move || {
            view.table.set_content_offset_x(-10_000);
        });
        view.say("at the far end, each text ends at the right edge of its half");
        assert_eq!(
            view.row(2),
            (0.0, HALF - CODE_WIDTH, HALF, HALF * 2.0 - CODE_WIDTH)
        );
        check_colors(CHECK_3)?;

        from_main(move || {
            view.table.set_content_offset_x(-150);
        });
        wait_for_next_frame();

        // The drag goes last and pins no colors. Its fling stops between
        // 2 pixels, and text there does not render the same on every run.
        // A finger drag to the left by 100 moves both sides too.
        inject_touches(
            "
            400 300 b
            300 305 m
        ",
        );
        assert_eq!(
            view.row(2),
            (0.0, NUMBER_WIDTH - 250.0, HALF, HALF + NUMBER_WIDTH - 250.0)
        );
        inject_touches("300 305 e");
        while from_main(move || view.table.scroll.is_scrolling()) {
            wait_for_next_frame();
        }

        checkpoint("dragged left with a finger, both texts followed")?;

        Ok(())
    }
}

const CHECK_1: &str = r"
         156    4 - #ffffff
         456    4 - #ffffff
         324   24 - #adadad
         224   32 - #ffffff
         320   32 - #5f5f5f
          20  104 - #39424e
         128  104 - #f6dcdc
         540  104 - #a1b1a1
         588  104 - #000000
         228  156 - #000000
         320  156 - #ffffff
         460  160 - #6e796e
         540  204 - #a1b1a1
         152  208 - #f6dcdc
          76  252 - #1d1a1a
         368  256 - #474e47
           4  284 - #39424e
         552  300 - #778377
         292  304 - #f6dcdc
         428  308 - #020202
         120  352 - #3e3737
         312  400 - #39424e
         552  400 - #778377
          76  452 - #1d1a1a
           4  476 - #252b33
         164  476 - #a08f8f
         252  476 - #a08f8f
         312  476 - #252b33
         376  476 - #dcf2dc
         480  476 - #dcf2dc
         216  592 - #597c95
         592  592 - #597c95
";

const CHECK_2: &str = r"
         184    4 - #ffffff
         448    4 - #ffffff
          64   32 - #000000
         300   32 - #d2d2d2
         372   32 - #ffffff
         552   32 - #ffffff
         136  104 - #524949
         412  104 - #a1b1a1
         484  108 - #b4c6b4
           4  124 - #39424e
         320  156 - #ffffff
         176  200 - #1d1a1a
         100  204 - #000000
         388  204 - #dcf2dc
         480  204 - #dcf2dc
           4  232 - #39424e
         592  260 - #dcf2dc
         176  300 - #1d1a1a
          64  304 - #7b6e6e
         348  304 - #dcf2dc
         484  352 - #b4c6b4
         176  356 - #000000
          52  404 - #837575
         412  404 - #a1b1a1
         592  428 - #dcf2dc
         464  456 - #474e47
         168  476 - #a08f8f
         244  476 - #a08f8f
         300  476 - #252b33
         320  476 - #252b33
           4  592 - #597c95
         236  592 - #597c95
";

const CHECK_3: &str = r"
           4    4 - #ffffff
         212   24 - #c7c7c7
         296   24 - #000000
         344   24 - #7a7a7a
         516   24 - #050505
         212   28 - #c7c7c7
         124   32 - #969696
         168   32 - #000000
         212   32 - #c7c7c7
         256   32 - #6c6c6c
         328   32 - #2c2c2c
         340   32 - #acacac
         372   32 - #1e1e1e
         432   32 - #ffffff
         488   32 - #9f9f9f
         544   32 - #6e6e6e
         592   56 - #ffffff
         320  104 - #39424e
          20  152 - #ffffff
         320  156 - #ffffff
         592  244 - #dcf2dc
          20  304 - #39424e
         320  304 - #39424e
         336  384 - #39424e
          36  428 - #39424e
         300  476 - #252b33
         336  476 - #252b33
         372  476 - #8f9d8f
         448  476 - #8f9d8f
         592  476 - #dcf2dc
           4  548 - #597c95
         244  592 - #597c95
";
