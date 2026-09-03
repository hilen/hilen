use std::ops::Deref;

use anyhow::Result;
use parking_lot::Mutex;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    gm::color::Color,
    ui::{CellRegistry, Label, Setup, TableData, TableView, View, ViewData, ViewTest, view},
    ui_test::{check_colors, inject_touches, set_record_probe_count},
};

static SELECTED: Mutex<String> = Mutex::new(String::new());

const MIDDLE: &str = r"
               4    4 - #dfe8f0
              96    4 - #dfe8f0
             316    4 - #dfe8f0
             396    4 - #dfe8f0
             556    4 - #597c95
             476   20 - #597c95
             228   36 - #dfe8f0
             152   40 - #45484b
             196   40 - #010101
             236   44 - #aeb5bb
             248   44 - #383a3c
             160   48 - #010101
             152   52 - #45484b
             176   52 - #dfe8f0
              72   64 - #dfe8f0
               4   72 - #dfe8f0
             592   76 - #597c95
             332   92 - #c3d2e0
             396  100 - #c3d2e0
             488  100 - #597c95
             224  124 - #000000
              88  128 - #c3d2e0
             156  128 - #c3d2e0
             244  128 - #000000
             152  132 - #3d4146
             176  132 - #c3d2e0
             188  132 - #000000
             244  132 - #000000
             164  136 - #c3d2e0
             244  136 - #000000
              20  140 - #c3d2e0
             152  140 - #3d4146
             176  140 - #c3d2e0
             200  140 - #000101
             228  140 - #c3d2e0
             244  140 - #000000
             592  160 - #597c95
             392  168 - #c3d2e0
             316  184 - #dfe8f0
             484  192 - #597c95
               4  204 - #dfe8f0
              68  212 - #dfe8f0
             244  216 - #dfe8f0
             152  220 - #45484b
             160  220 - #dfe8f0
             176  224 - #dfe8f0
             240  224 - #dfe8f0
             164  228 - #dfe8f0
             204  228 - #000000
             232  228 - #000000
             152  232 - #45484b
             180  232 - #010101
             580  236 - #597c95
             432  260 - #597c95
               4  272 - #c3d2e0
             108  272 - #c3d2e0
             344  272 - #c3d2e0
             516  280 - #597c95
             392  304 - #c3d2e0
             152  308 - #3d4146
             160  308 - #c3d2e0
             224  308 - #c2d1df
             248  308 - #000000
             152  312 - #3d4146
             176  312 - #c3d2e0
              72  316 - #c3d2e0
             164  316 - #c3d2e0
             236  316 - #c3d2e0
             244  316 - #c3d2e0
             592  316 - #597c95
             152  320 - #3d4146
             176  320 - #c3d2e0
             200  320 - #000101
             228  320 - #c3d2e0
               8  332 - #c3d2e0
             316  352 - #c3d2e0
             396  360 - #dfe8f0
              92  376 - #dfe8f0
              32  388 - #dfe8f0
             492  392 - #597c95
             156  396 - #dfe8f0
             248  396 - #020202
             152  400 - #45484b
             196  400 - #010101
             248  400 - #020202
             592  400 - #597c95
             164  408 - #dfe8f0
             232  408 - #000000
             244  408 - #3f4244
             248  408 - #000000
             152  412 - #45484b
             176  412 - #dfe8f0
             248  412 - #020202
               4  444 - #dfe8f0
             344  448 - #dfe8f0
             432  456 - #597c95
              72  460 - #c3d2e0
             288  460 - #c3d2e0
             248  484 - #000000
             152  488 - #3d4146
             228  488 - #c3d2e0
             188  492 - #000000
             512  492 - #597c95
             164  496 - #c3d2e0
             204  496 - #010101
             244  496 - #c3d2e0
             152  500 - #3d4146
             176  500 - #c3d2e0
             192  504 - #000000
             224  504 - #000000
               4  508 - #c3d2e0
             592  508 - #597c95
              88  528 - #c3d2e0
             316  532 - #c3d2e0
             396  540 - #dfe8f0
             228  576 - #dfe8f0
             152  580 - #45484b
             176  584 - #dfe8f0
             244  584 - #dfe8f0
             152  588 - #45484b
             164  588 - #dfe8f0
             196  588 - #dfe8f0
             484  588 - #597c95
              56  592 - #dfe8f0
             156  592 - #dfe8f0
             188  592 - #dfe8f0
             352  592 - #dfe8f0
             584  592 - #597c95
            ";

const TOP: &str = r"
               4    4 - #dfe8f0
             312    4 - #dfe8f0
             392    4 - #dfe8f0
             396    4 - #91979c
             500    4 - #597c95
              84    8 - #dfe8f0
             396    8 - #91979c
             396   12 - #91979c
             396   16 - #91979c
             396   20 - #91979c
             396   24 - #91979c
             396   28 - #91979c
             396   32 - #91979c
             160   36 - #010101
             396   36 - #91979c
             232   40 - #dfe8f0
             240   40 - #000000
             164   44 - #3b3e40
             184   44 - #dfe8f0
             160   48 - #010101
             232   48 - #dfe8f0
             240   48 - #000000
             184   52 - #dfe8f0
             200   52 - #000000
             592   60 - #597c95
               4   68 - #dfe8f0
             300   72 - #dfe8f0
             516   80 - #597c95
              64   92 - #c3d2e0
             432  120 - #597c95
             340  124 - #c3d2e0
             164  128 - #c3d2e0
             236  128 - #000000
             168  132 - #c3d2e0
             184  132 - #c3d2e0
             164  136 - #373b3f
             188  136 - #c3d2e0
             160  140 - #010101
             184  140 - #c3d2e0
             212  140 - #000000
             236  140 - #000000
             200  144 - #000000
             512  160 - #597c95
               4  176 - #c3d2e0
              88  176 - #c3d2e0
             592  176 - #597c95
             316  200 - #dfe8f0
             160  216 - #010101
             396  216 - #dfe8f0
             240  220 - #000000
             164  224 - #3b3e40
             184  224 - #dfe8f0
             472  224 - #597c95
             188  228 - #dfe8f0
             160  232 - #010101
             184  232 - #dfe8f0
             200  232 - #000000
             212  232 - #000000
             592  260 - #597c95
              64  268 - #dfe8f0
             328  280 - #c3d2e0
             396  288 - #c3d2e0
               4  292 - #c3d2e0
             484  296 - #597c95
             160  304 - #010101
             168  308 - #c3d2e0
             240  308 - #000000
             160  312 - #000000
             184  312 - #c3d2e0
             164  316 - #373b3f
             184  316 - #c3d2e0
             188  316 - #c3d2e0
             160  320 - #010101
             184  320 - #c3d2e0
             212  320 - #000000
             200  324 - #000000
             232  324 - #000000
             300  348 - #c3d2e0
              88  352 - #c3d2e0
             384  356 - #c3d2e0
             592  356 - #597c95
               4  360 - #dfe8f0
             456  364 - #597c95
             160  396 - #010101
             236  400 - #dfe8f0
             164  404 - #3b3e40
             188  408 - #dfe8f0
             232  408 - #3f4244
             516  408 - #597c95
             160  412 - #010101
             212  412 - #000000
             324  420 - #dfe8f0
             396  428 - #dfe8f0
              64  448 - #dfe8f0
             592  460 - #597c95
             292  468 - #c3d2e0
               4  472 - #c3d2e0
             232  484 - #000000
             164  488 - #c3d2e0
             168  492 - #c3d2e0
             184  492 - #c3d2e0
             164  496 - #373b3f
             188  496 - #c3d2e0
             240  496 - #000000
             372  496 - #c3d2e0
             476  496 - #597c95
             184  500 - #c3d2e0
             212  500 - #000000
             236  500 - #c3d2e0
             200  504 - #000000
              92  520 - #c3d2e0
             304  528 - #c3d2e0
               4  536 - #c3d2e0
             396  556 - #dfe8f0
             592  556 - #597c95
             160  576 - #010101
             164  584 - #3b3e40
             184  584 - #dfe8f0
             172  588 - #dfe8f0
             232  588 - #dfe8f0
             240  588 - #000000
              72  592 - #dfe8f0
             164  592 - #dfe8f0
             184  592 - #dfe8f0
             192  592 - #dfe8f0
             204  592 - #dfe8f0
             340  592 - #dfe8f0
             464  592 - #597c95
            ";

const BOTTOM: &str = r"
              52    4 - #c3d2e0
             244    4 - #000000
             336    4 - #c3d2e0
             464    4 - #597c95
             156    8 - #c3d2e0
             172    8 - #c3d2e0
             224    8 - #c3d2e0
             152   12 - #3d4146
             188   12 - #000000
             236   12 - #c3d2e0
             164   16 - #c3d2e0
             592   16 - #597c95
             152   20 - #3d4146
             176   20 - #c3d2e0
             200   20 - #000101
             244   20 - #c3d2e0
             224   24 - #000000
             396   28 - #c3d2e0
               4   56 - #c3d2e0
             524   60 - #597c95
              84   68 - #dfe8f0
             308   68 - #dfe8f0
             448   84 - #597c95
             372   88 - #dfe8f0
             248   96 - #020202
             152  100 - #45484b
             160  100 - #dfe8f0
             196  100 - #010101
             224  100 - #dfe8f0
             248  100 - #020202
             176  104 - #dfe8f0
             164  108 - #dfe8f0
             244  108 - #3f4244
             248  108 - #000000
             592  108 - #597c95
             152  112 - #45484b
             176  112 - #dfe8f0
             248  112 - #020202
               4  116 - #dfe8f0
             508  140 - #597c95
              52  148 - #dfe8f0
             108  148 - #dfe8f0
             424  156 - #597c95
             332  172 - #c3d2e0
             152  188 - #3d4146
             224  188 - #c3d2e0
             152  192 - #3d4146
             164  196 - #c3d2e0
             204  196 - #010101
             244  196 - #c3d2e0
             152  200 - #3d4146
             176  200 - #c3d2e0
               4  204 - #c3d2e0
             192  204 - #000000
             224  204 - #000000
             528  216 - #597c95
             396  236 - #c3d2e0
              80  240 - #dfe8f0
             316  248 - #dfe8f0
             592  260 - #597c95
               4  280 - #dfe8f0
             152  280 - #45484b
             160  280 - #dfe8f0
             196  280 - #010101
             224  280 - #dfe8f0
             176  284 - #dfe8f0
             248  284 - #000000
             488  284 - #597c95
             152  288 - #45484b
             164  288 - #dfe8f0
             172  288 - #dfe8f0
             152  292 - #45484b
             176  292 - #dfe8f0
             244  292 - #dfe8f0
              88  304 - #dfe8f0
             340  320 - #dfe8f0
             420  324 - #597c95
             592  336 - #597c95
              56  360 - #c3d2e0
             248  364 - #000000
             152  368 - #3d4146
             156  368 - #c3d2e0
             152  372 - #3d4146
             176  372 - #c3d2e0
             228  372 - #c3d2e0
             164  376 - #c3d2e0
             204  376 - #010101
             152  380 - #3d4146
             532  380 - #597c95
               4  384 - #c3d2e0
             192  384 - #000000
             224  384 - #000000
             396  400 - #c3d2e0
             320  420 - #dfe8f0
             468  420 - #597c95
              80  432 - #dfe8f0
               4  444 - #dfe8f0
             248  456 - #010101
             152  460 - #45484b
             160  460 - #dfe8f0
             196  460 - #010101
             224  460 - #dfe8f0
             396  460 - #dfe8f0
             176  464 - #dfe8f0
             164  468 - #dfe8f0
             152  472 - #45484b
             176  472 - #dfe8f0
             244  472 - #dfe8f0
             348  496 - #dfe8f0
             472  496 - #597c95
             592  500 - #597c95
               4  508 - #dfe8f0
              68  516 - #c3d2e0
             396  532 - #c3d2e0
             304  536 - #c3d2e0
             152  548 - #3d4146
             248  548 - #010101
             176  552 - #c3d2e0
             224  552 - #c3d2e0
             164  556 - #c3d2e0
             248  556 - #010101
             152  560 - #3d4146
             200  560 - #000101
               4  592 - #c3d2e0
              80  592 - #c3d2e0
             316  592 - #c3d2e0
             392  592 - #c3d2e0
             528  592 - #597c95
            ";

// Taps prove the scroll position exactly: after each programmatic offset
// the rows under the top and bottom edges are known, so selecting them
// pins where the content actually sits.
#[view]
struct TableSetContentOffset {
    #[init]
    table: TableView,
}

impl Setup for TableSetContentOffset {
    fn setup(mut self: Weak<Self>) {
        self.table.place().tl(0).size(400, 600);
        self.table.set_data_source(self).register_cell::<Label>();
        self.table.reload_data();
    }
}

impl TableData for TableSetContentOffset {
    fn cell_height(&self, _: usize) -> f32 {
        90.0
    }

    fn number_of_cells(&self) -> usize {
        100
    }

    // Rows show their index on alternating fills, so a human run reads
    // the landed scroll position straight off the screen.
    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let cell = registry.cell::<Label>();
        cell.set_text(format!("Row {index}"));
        cell.set_color(if index.is_multiple_of(2) {
            Color::hex("#dfe8f0")
        } else {
            Color::hex("#c3d2e0")
        });
        cell
    }

    fn cell_selected(&mut self, index: usize) {
        *SELECTED.lock() += &format!("|{index}|");
    }
}

impl ViewTest for TableSetContentOffset {
    fn perform_test(mut view: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);
        // -4500 puts row 50 flush with the top edge, and the bottom edge
        // at content 5095 lands in row 56.
        from_main(move || {
            view.table.set_content_offset(-4500);
            assert!((view.table.content_offset() + 4500.0).abs() < 0.001);
        });
        inject_touches(
            "
                200  5    b
                200  5    e
                200  595  b
                200  595  e
            ",
        );
        assert_eq!(SELECTED.lock().deref(), "|50||56|");
        SELECTED.lock().clear();

        // The fills pin the position in pixels: even rows light, odd rows
        // darker. Row 50 owns the top edge, row 51 sits under it, row 56
        // owns the bottom edge.
        check_colors(MIDDLE)?;

        // A positive offset clamps back to the top.
        from_main(move || {
            view.table.set_content_offset(100);
            assert!(view.table.content_offset().abs() < 0.001);
        });
        inject_touches(
            "
                200  5    b
                200  5    e
            ",
        );
        assert_eq!(SELECTED.lock().deref(), "|0|");
        SELECTED.lock().clear();

        // Back at the top: row 0 light, row 1 darker.
        check_colors(TOP)?;

        // Past the end clamps to the bottom: 100 rows of 90 against a 600
        // viewport is -8400, the last row flush with the bottom edge.
        from_main(move || {
            view.table.set_content_offset(-1_000_000);
            assert!((view.table.content_offset() + 8400.0).abs() < 0.001);
        });
        inject_touches(
            "
                200  595  b
                200  595  e
            ",
        );
        assert_eq!(SELECTED.lock().deref(), "|99|");
        SELECTED.lock().clear();

        // Clamped to the bottom: row 93 cut by the top edge, row 98
        // light, row 99 darker and flush with the bottom.
        check_colors(BOTTOM)?;

        Ok(())
    }
}
