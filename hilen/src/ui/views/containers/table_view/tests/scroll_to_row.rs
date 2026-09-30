use anyhow::Result;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    gm::{
        LossyConvert,
        color::{Color, WHITE},
    },
    ui::{CellRegistry, Label, Setup, TableData, TableView, View, ViewData, ViewFrame, ViewTest, view},
    ui_test::{check_colors, checkpoint},
};

const ROW_30: &str = r"
            4    4 - #f08a24
            324    4 - #f08a24
            480   12 - #f08a24
            120   20 - #f08a24
            424   20 - #f08a24
            172   24 - #a86119
            120   76 - #c3d2e0
            176   84 - #666e75
            480  120 - #4f5255
            592  148 - #597c95
            116  188 - #0a0b0b
            424  200 - #8a9095
            4  212 - #c3d2e0
            292  228 - #91979c
            292  244 - #91979c
            176  248 - #515558
            424  308 - #010101
            480  352 - #b0bdca
            120  356 - #dfe8f0
            4  392 - #c3d2e0
            172  412 - #c3d2e0
            468  416 - #dfe8f0
            424  460 - #c3d2e0
            288  464 - #dfe8f0
            116  468 - #0b0c0c
            592  488 - #dfe8f0
            424  500 - #8a9095
            424  564 - #6a7279
            468  568 - #5b6269
            172  584 - #9da3a8
            120  588 - #dfe8f0
            304  592 - #597c95
            ";

const ROW_70: &str = r"
            344    4 - #f08a24
            480   12 - #f08a24
            120   20 - #f08a24
            424   20 - #f08a24
            172   24 - #a86119
            4   48 - #f08a24
            176   84 - #666e75
            476  112 - #dfe8f0
            424  180 - #a4b1bd
            4  188 - #c3d2e0
            120  188 - #c3d2e0
            336  224 - #597c95
            592  228 - #dfe8f0
            176  248 - #515558
            436  272 - #dfe8f0
            172  300 - #000000
            28  316 - #c3d2e0
            120  356 - #dfe8f0
            424  364 - #a4b1bd
            480  364 - #8b96a0
            116  412 - #0a0b0b
            592  424 - #597c95
            292  432 - #7f8992
            292  444 - #3a5161
            4  448 - #dfe8f0
            292  460 - #91979c
            468  476 - #000000
            292  480 - #91979c
            120  524 - #c3d2e0
            424  560 - #6a7279
            476  564 - #c3d2e0
            172  584 - #9da3a8
            ";

const CLAMPED: &str = r"
            300    4 - #597c95
            120   12 - #c3d2e0
            468   28 - #dfe8f0
            424   84 - #c3d2e0
            176  124 - #666e75
            564  124 - #dfe8f0
            176  128 - #666e75
            424  152 - #8a9095
            4  160 - #dfe8f0
            176  188 - #4f5255
            372  200 - #597c95
            116  240 - #0a0b0b
            424  244 - #6a7279
            592  256 - #c3d2e0
            176  292 - #515558
            4  332 - #c3d2e0
            424  336 - #8a9095
            176  344 - #000000
            480  344 - #dfe8f0
            308  400 - #f08a24
            592  428 - #f08a24
            424  452 - #f08a24
            480  452 - #f08a24
            424  456 - #824b13
            120  460 - #f08a24
            164  516 - #dfe8f0
            4  540 - #dfe8f0
            292  540 - #91979c
            292  548 - #3a5161
            424  572 - #a4b1bd
            480  576 - #c3d2e0
            148  580 - #000000
            ";

const ROWS: usize = 100;
const HEADER: f32 = 40.0;
const SPACING: f32 = 6.0;

/// One table of the fixture. The target row is orange, so a human run
/// sees at once whether it landed at the top.
#[view]
struct JumpTable {
    variable: bool,
    /// Heights after the reload step, every row a different size.
    reshaped: bool,
    target:   usize,

    #[init]
    table: TableView,
}

impl Setup for JumpTable {
    fn setup(mut self: Weak<Self>) {
        self.table.place().back();
        self.table.set_data_source(self).register_cell::<Label>();
        self.table.set_header_height(HEADER).set_cell_spacing(SPACING);
    }
}

impl JumpTable {
    fn build(mut self: Weak<Self>, variable: bool, title: &str) {
        self.variable = variable;
        self.table.set_variable_heights(variable);
        let header = self.table.add_header_view::<Label>();
        header.set_text(title).set_text_size(20);
        header.set_color(Color::hex("#39424e")).set_text_color(WHITE);
        header.place().lrt(0).h(HEADER);
    }

    /// The top row on screen and the row under the bottom edge.
    fn edges(self: Weak<Self>) -> (Option<usize>, Option<usize>) {
        let height = self.table.height();
        (
            self.table.index_at((10, 1).into()),
            self.table.index_at((10, height - 1.0).into()),
        )
    }
}

impl TableData for JumpTable {
    fn cell_height(&self, index: usize) -> f32 {
        if !self.variable {
            return 50.0;
        }
        let step = if self.reshaped { 7 } else { 3 };
        30.0 + (index % step).lossy_convert() * 14.0
    }

    fn number_of_cells(&self) -> usize {
        ROWS
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let cell = registry.cell::<Label>();
        cell.set_text(format!("Row {index}")).set_text_size(20);
        cell.set_color(if index == self.target {
            Color::hex("#f08a24")
        } else if index.is_multiple_of(2) {
            Color::hex("#dfe8f0")
        } else {
            Color::hex("#c3d2e0")
        });
        cell
    }
}

/// A fixed height table on the left and a variable height one on the
/// right, each with a header above its rows.
#[view]
struct TableScrollToRow {
    #[init]
    fixed:    JumpTable,
    variable: JumpTable,
}

impl Setup for TableScrollToRow {
    fn setup(self: Weak<Self>) {
        self.fixed.place().tl(0).size(295, 600);
        self.variable.place().tr(0).size(295, 600);
        self.fixed.build(false, "fixed");
        self.variable.build(true, "variable");

        // Before the first layout, the tables have no size yet.
        for mut table in [self.fixed, self.variable] {
            table.target = 30;
            table.table.reload_data();
            table.table.scroll_to_row(30);
        }
    }
}

impl ViewTest for TableScrollToRow {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(move || {
            assert_eq!(view.fixed.edges().0, Some(30));
            assert_eq!(view.variable.edges().0, Some(30));
        });
        checkpoint("row 30 orange at the top of both, asked before the first layout")?;
        check_colors(ROW_30)?;

        // New heights and a jump in the same frame, the jump must read the
        // reloaded geometry.
        from_main(move || {
            for mut table in [view.fixed, view.variable] {
                table.reshaped = true;
                table.target = 70;
                table.table.reload_data();
                table.table.scroll_to_row(70);
                assert_eq!(table.edges().0, Some(70));
            }
        });
        checkpoint("row 70 orange at the top, jumped right after a reload with new heights")?;
        check_colors(ROW_70)?;

        // Near the end the offset clamps, the last row sits on the bottom.
        from_main(move || {
            for mut table in [view.fixed, view.variable] {
                table.target = 97;
                table.table.reload_data();
                table.table.scroll_to_row(97);
                let (_, bottom) = table.edges();
                assert_eq!(bottom, Some(ROWS - 1));
                let max = table.table.content_height() - table.table.height();
                assert!((table.table.content_offset() + max).abs() < 0.001);
            }
        });
        checkpoint("row 97 orange near the bottom, row 99 on the bottom edge")?;
        check_colors(CLAMPED)?;

        Ok(())
    }
}
