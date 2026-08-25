use std::ops::Deref;

use anyhow::Result;
use parking_lot::Mutex;

use crate::{
    self as hilen,
    deps::refs::Weak,
    gm::color::{BLUE, Color, GREEN, PURPLE, RED, YELLOW},
    ui::{CellRegistry, Container, Setup, TableData, TableView, View, ViewData, ViewTest, view},
    ui_test::{check_colors, inject_scroll, inject_touches, set_record_probe_count},
};

static SELECTED: Mutex<String> = Mutex::new(String::new());

const PALETTE: [Color; 4] = [GREEN, BLUE, YELLOW, RED];

// The backdrop behind the table has a color no cell uses, so the
// gaps between cells expose it and the recorder pins probes there.
#[view]
struct TableSpacingTest {
    #[init]
    under: Container,
    table: TableView,
}

impl Setup for TableSpacingTest {
    fn setup(mut self: Weak<Self>) {
        self.under.set_color(PURPLE);
        self.under.place().tl(0).size(400, 600);

        self.table.place().tl(0).size(400, 600);
        self.table.set_data_source(self).register_cell::<Container>();
        self.table.set_cell_spacing(16);
        self.table.set_columns(2);
        self.table.reload_data();
    }
}

impl TableData for TableSpacingTest {
    fn cell_height(&self, _: usize) -> f32 {
        90.0
    }

    fn number_of_cells(&self) -> usize {
        12
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let cell = registry.cell::<Container>();
        cell.set_color(PALETTE[index % PALETTE.len()]);
        cell
    }

    fn cell_selected(&mut self, index: usize) {
        *SELECTED.lock() += &format!("|{index}|");
    }
}

// Cells are 192x90 on a 106 pitch, the 16px gaps show the backdrop
// between rows and columns.
fn check_unscrolled() -> Result<()> {
    check_colors(
        r"
               4    4 - #00ff00
             200    4 - #ff00ff
             396    4 - #000096
             592    4 - #597c95
             104    8 - #00ff00
             296   32 - #0000e7
             396   52 - #000096
              52   80 - #00ff00
             232   84 - #0000e7
             152   92 - #ff00ff
             324   96 - #ff00ff
             396   96 - #a600a6
             396  104 - #a600a6
             396  124 - #a60000
             536  132 - #597c95
             280  148 - #ff0000
             208  152 - #ff0000
             396  152 - #a60000
              36  168 - #ffff00
             124  176 - #ffff00
             396  196 - #a600a6
             396  200 - #a600a6
             396  204 - #a600a6
             396  208 - #a600a6
             196  224 - #ff00ff
             308  236 - #0000e7
               4  244 - #00ff00
              84  260 - #00ff00
             396  260 - #000096
             592  260 - #597c95
             396  284 - #000096
             220  288 - #0000e7
             396  304 - #a600a6
             396  308 - #a600a6
             396  312 - #a600a6
             396  316 - #a600a6
             280  320 - #ff0000
             164  328 - #ffff00
               8  332 - #ffff00
             504  332 - #597c95
             396  356 - #a60000
             396  380 - #a60000
             312  400 - #ff0000
             396  408 - #a600a6
             396  416 - #a600a6
              20  420 - #ff00ff
             112  424 - #00ff00
             228  424 - #0000e7
             564  424 - #597c95
             396  460 - #000096
             220  492 - #0000e7
              40  512 - #00ff00
             288  512 - #0000e7
             396  512 - #000096
             156  516 - #ff00ff
             508  520 - #597c95
             396  524 - #a600a6
             396  548 - #a60000
             396  576 - #a60000
               4  592 - #ffff00
             104  592 - #ffff00
             208  592 - #ff0000
             312  592 - #ff0000
             592  592 - #597c95
            ",
    )
}

// Content height is 6 rows of 90 plus 5 gaps of 16, no gap after the
// last row, so it lands flush with the table bottom after scrolling
// all the way down.
fn check_scrolled_to_bottom() -> Result<()> {
    check_colors(
        r"
               4    4 - #00ff00
              96    4 - #00ff00
             208    4 - #0000e7
             312    4 - #0000e7
             592    4 - #597c95
             396   24 - #000096
             220   76 - #ff00ff
             396   76 - #a600a6
             148   84 - #ff00ff
              48   88 - #ffff00
             292   88 - #ff0000
             396   88 - #a60000
             528  120 - #597c95
             396  132 - #a60000
             228  172 - #ff0000
               4  176 - #ff00ff
             396  176 - #a600a6
             396  184 - #a600a6
             396  188 - #a600a6
              96  192 - #00ff00
             312  200 - #0000e7
             396  216 - #000096
             396  240 - #000096
             592  240 - #597c95
             192  252 - #ff00ff
               8  264 - #00ff00
             276  280 - #0000e7
             396  284 - #a600a6
             396  288 - #a600a6
             396  292 - #a600a6
             396  296 - #a600a6
             112  304 - #ffff00
             220  316 - #ff0000
             396  316 - #a60000
             508  320 - #597c95
             396  340 - #a60000
               4  348 - #ffff00
             308  364 - #ff0000
              76  368 - #ffff00
             196  376 - #ff00ff
             396  392 - #a600a6
             396  396 - #a600a6
             396  400 - #a600a6
             572  416 - #597c95
             396  420 - #000096
             124  424 - #00ff00
              36  428 - #00ff00
             208  448 - #0000e7
             396  448 - #000096
             280  452 - #0000e7
             396  472 - #000096
             396  500 - #a600a6
             152  508 - #ff00ff
             396  508 - #a600a6
             512  512 - #597c95
              56  516 - #ffff00
             232  516 - #ff0000
             396  548 - #a60000
             296  568 - #ff0000
               4  592 - #ffff00
             104  592 - #ffff00
             208  592 - #ff0000
             396  592 - #a60000
             592  592 - #597c95
            ",
    )
}

impl ViewTest for TableSpacingTest {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);

        SELECTED.lock().clear();

        check_unscrolled()?;

        // Gaps are purely visual for touch: a tap in a gap selects
        // the nearest cell, each gap side goes to its closer cell.
        inject_touches(
            "
                 50   50   b
                 50   50   e
                300  150   b
                300  150   e
                196   50   b
                196   50   e
                204   50   b
                204   50   e
                100   94   b
                100   94   e
                100  102   b
                100  102   e
            ",
        );

        assert_eq!(SELECTED.lock().deref(), "|0||3||0||1||0||2|");
        SELECTED.lock().clear();

        inject_scroll(-1000);

        check_scrolled_to_bottom()?;

        inject_touches(
            "
                300  550   b
                300  550   e
            ",
        );

        assert_eq!(SELECTED.lock().deref(), "|11|");

        Ok(())
    }
}
