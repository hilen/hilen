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
              64    4 - #00ff00
             128    4 - #00ff00
             228    4 - #0000e7
             316    4 - #0000e7
             592    4 - #597c95
             456   16 - #597c95
             272   52 - #0000e7
             384   56 - #0000e7
              72   60 - #00ff00
             148   76 - #00ff00
             220   88 - #0000e7
             312  100 - #ff00ff
               8  104 - #ff00ff
             516  120 - #597c95
             104  136 - #ffff00
             396  140 - #ff0000
             260  148 - #ff0000
             180  152 - #ffff00
              44  156 - #ffff00
               4  208 - #ff00ff
              80  208 - #ff00ff
             592  212 - #597c95
             252  216 - #0000e7
             348  216 - #0000e7
             152  220 - #00ff00
             484  224 - #597c95
             204  264 - #ff00ff
              40  268 - #00ff00
             308  296 - #0000e7
             404  300 - #597c95
             100  304 - #ff00ff
             488  308 - #597c95
               4  332 - #ffff00
             220  332 - #ff0000
             152  344 - #ffff00
              60  356 - #ffff00
             340  356 - #ff0000
             276  360 - #ff0000
             548  368 - #597c95
             396  396 - #ff0000
             100  404 - #ffff00
             184  404 - #ffff00
             304  420 - #ff00ff
               4  432 - #00ff00
             360  448 - #0000e7
             232  460 - #0000e7
             100  464 - #00ff00
             500  464 - #597c95
             160  484 - #00ff00
             396  500 - #0000e7
              64  512 - #00ff00
             280  516 - #ff00ff
             592  520 - #597c95
               4  528 - #ff00ff
             120  528 - #ff00ff
             344  536 - #ff0000
             216  568 - #ff0000
              64  572 - #ffff00
             396  588 - #ff0000
               4  592 - #ffff00
             120  592 - #ffff00
             312  592 - #ff0000
             508  592 - #597c95
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
             124    4 - #00ff00
             316    4 - #0000e7
             488    4 - #597c95
             396   16 - #0000e7
              64   20 - #00ff00
             220   36 - #0000e7
             156   56 - #00ff00
             348   60 - #0000e7
              64   84 - #ff00ff
             592   84 - #597c95
             292   92 - #ff0000
               4  100 - #ffff00
             396  100 - #ff0000
             496  104 - #597c95
             160  116 - #ffff00
              96  136 - #ffff00
             236  144 - #ff0000
             360  152 - #ff0000
               4  164 - #ffff00
             304  180 - #ff00ff
             100  196 - #00ff00
             180  196 - #00ff00
             396  204 - #0000e7
             512  204 - #597c95
              36  224 - #00ff00
             276  240 - #0000e7
             340  244 - #0000e7
             148  256 - #00ff00
             216  264 - #0000e7
               4  284 - #ff00ff
              96  296 - #ff00ff
             404  300 - #597c95
             304  304 - #ff0000
             496  328 - #597c95
             592  328 - #597c95
             204  332 - #ff00ff
              28  336 - #ffff00
             356  336 - #ff0000
              76  368 - #ffff00
             152  376 - #ffff00
             248  384 - #ff0000
               4  392 - #ff00ff
             336  392 - #ff00ff
             396  396 - #ff00ff
             512  424 - #597c95
             188  444 - #00ff00
             392  456 - #0000e7
             100  460 - #00ff00
             296  468 - #0000e7
               4  492 - #00ff00
             348  496 - #ff00ff
             468  500 - #597c95
             212  516 - #ff0000
             140  520 - #ffff00
             592  520 - #597c95
              68  536 - #ffff00
             276  536 - #ff0000
             384  544 - #ff0000
             460  588 - #597c95
               4  592 - #ffff00
             124  592 - #ffff00
             204  592 - #ff00ff
             312  592 - #ff0000
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
