use std::ops::Deref;

use anyhow::Result;
use parking_lot::Mutex;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    gm::color::{BLUE, GREEN, PURPLE, RED},
    ui::{
        CellRegistry, Container, Setup, TableData, TableView, View, ViewData, ViewFrame, ViewSubviews,
        ViewTest, view,
    },
    ui_test::{check_colors, inject_scroll, inject_touches, set_record_probe_count},
};

static SELECTED: Mutex<String> = Mutex::new(String::new());

const SHORT: f32 = 30.0;
const TALL: f32 = 42.0;
const SPACING: f32 = 4.0;
const ROWS: usize = 1000;

/// Every third row is tall, like a commit row with ref chips.
fn is_tall(index: usize) -> bool {
    index % 3 == 2
}

fn height(index: usize) -> f32 {
    if is_tall(index) { TALL } else { SHORT }
}

const TOP: &str = r"
            8    4 - #00ff00
            36    4 - #00ff00
            156    4 - #00ff00
            192    4 - #00ff00
            228    4 - #00ff00
            320    4 - #00ff00
            368    4 - #00ff00
            396    4 - #00a600
            500    4 - #597c95
            556    4 - #597c95
            396    8 - #00a600
            592    8 - #597c95
            276   12 - #00ff00
            396   12 - #00a600
            64   16 - #00ff00
            96   16 - #00ff00
            396   16 - #00a600
            128   20 - #00ff00
            328   28 - #00ff00
            356   28 - #00ff00
            4   32 - #ff00ff
            40   32 - #ff00ff
            164   32 - #ff00ff
            220   32 - #ff00ff
            248   32 - #ff00ff
            452   32 - #597c95
            528   32 - #597c95
            304   36 - #0000e7
            192   40 - #0000e7
            396   44 - #0000e7
            592   44 - #597c95
            72   48 - #0000e7
            108   48 - #0000e7
            488   52 - #597c95
            12   56 - #0000e7
            144   56 - #0000e7
            420   56 - #597c95
            272   60 - #0000e7
            36   64 - #ff00ff
            172   64 - #ff00ff
            200   64 - #ff00ff
            228   64 - #ff00ff
            328   64 - #ff00ff
            380   64 - #ff00ff
            560   64 - #597c95
            524   72 - #597c95
            4   80 - #ff0000
            96   80 - #ff0000
            300   80 - #ff0000
            356   80 - #ff0000
            244   88 - #ff0000
            444   88 - #597c95
            60   92 - #ff0000
            188   92 - #ff0000
            272   96 - #ff0000
            332   96 - #ff0000
            136  100 - #ff0000
            220  100 - #ff0000
            396  100 - #ff0000
            488  100 - #597c95
            576  100 - #597c95
            528  108 - #597c95
            36  112 - #ff00ff
            88  112 - #ff00ff
            112  112 - #ff00ff
            304  112 - #ff00ff
            360  112 - #ff00ff
            4  116 - #0000e7
            172  120 - #0000e7
            248  120 - #0000e7
            384  124 - #0000e7
            432  128 - #597c95
            64  132 - #0000e7
            468  132 - #597c95
            108  136 - #0000e7
            204  136 - #0000e7
            280  136 - #0000e7
            552  136 - #597c95
            304  140 - #0000e7
            28  144 - #ff00ff
            152  144 - #ff00ff
            180  144 - #ff00ff
            232  144 - #ff00ff
            328  144 - #ff00ff
            364  144 - #ff00ff
            508  144 - #597c95
            396  148 - #00ff00
            84  152 - #00ff00
            128  152 - #00ff00
            592  156 - #597c95
            256  160 - #00ff00
            4  164 - #00ff00
            296  164 - #00ff00
            452  164 - #597c95
            536  168 - #597c95
            64  172 - #00ff00
            180  172 - #00ff00
            352  172 - #00ff00
            136  176 - #00ff00
            324  176 - #00ff00
            28  180 - #ff00ff
            100  180 - #ff00ff
            216  180 - #ff00ff
            276  180 - #ff00ff
            384  180 - #ff00ff
            420  180 - #597c95
            244  184 - #ff0000
            504  184 - #597c95
            192  196 - #ff0000
            120  200 - #ff0000
            584  200 - #597c95
            160  204 - #ff0000
            264  204 - #ff0000
            320  204 - #ff0000
            540  204 - #597c95
            4  208 - #ff0000
            76  208 - #ff0000
            232  208 - #ff0000
            360  208 - #ff0000
            396  208 - #ff0000
            436  212 - #597c95
            292  216 - #ff0000
            476  216 - #597c95
            44  224 - #ff00ff
            136  224 - #ff00ff
            200  224 - #ff00ff
            108  228 - #00ff00
            376  228 - #00ff00
            324  232 - #00ff00
            84  236 - #00ff00
            164  236 - #00ff00
            268  240 - #00ff00
            352  240 - #00ff00
            552  240 - #597c95
            232  244 - #00ff00
            592  244 - #597c95
            4  248 - #00ff00
            300  248 - #00ff00
            396  252 - #00ff00
            516  252 - #597c95
            40  256 - #00ff00
            328  256 - #00ff00
            448  256 - #597c95
            72  260 - #ff00ff
            108  260 - #ff00ff
            140  260 - #ff00ff
            188  260 - #ff00ff
            360  268 - #0000e7
            164  276 - #0000e7
            224  276 - #0000e7
            264  276 - #0000e7
            4  280 - #0000e7
            56  280 - #0000e7
            332  280 - #0000e7
            84  284 - #0000e7
            296  284 - #0000e7
            32  288 - #0000e7
            140  292 - #ff00ff
            188  292 - #ff00ff
            376  292 - #ff00ff
            108  296 - #ff0000
            480  296 - #597c95
            548  296 - #597c95
            432  300 - #597c95
            592  300 - #597c95
            4  308 - #ff0000
            216  308 - #ff0000
            252  308 - #ff0000
            312  308 - #ff0000
            68  312 - #ff0000
            160  312 - #ff0000
            40  316 - #ff0000
            284  316 - #ff0000
            368  320 - #ff0000
            116  324 - #ff0000
            336  324 - #ff0000
            396  328 - #ff0000
            556  332 - #597c95
            492  336 - #597c95
            20  340 - #ff00ff
            180  340 - #ff00ff
            244  340 - #ff00ff
            280  340 - #ff00ff
            432  340 - #597c95
            56  344 - #0000e7
            96  344 - #0000e7
            136  344 - #0000e7
            304  344 - #0000e7
            380  348 - #0000e7
            592  348 - #597c95
            328  352 - #0000e7
            208  356 - #0000e7
            528  356 - #597c95
            36  360 - #0000e7
            356  360 - #0000e7
            76  364 - #0000e7
            272  364 - #0000e7
            464  364 - #597c95
            164  368 - #0000e7
            240  368 - #0000e7
            396  368 - #0000e7
            12  376 - #00ff00
            132  376 - #00ff00
            320  376 - #00ff00
            52  380 - #00ff00
            372  380 - #00ff00
            500  380 - #597c95
            100  384 - #00ff00
            204  384 - #00ff00
            344  384 - #00ff00
            428  384 - #597c95
            296  392 - #00ff00
            544  392 - #597c95
            232  396 - #00ff00
            32  400 - #00ff00
            160  400 - #00ff00
            268  400 - #00ff00
            360  404 - #00ff00
            60  408 - #ff00ff
            108  408 - #ff00ff
            132  408 - #ff00ff
            188  408 - #ff00ff
            396  408 - #ff00ff
            4  412 - #ff0000
            580  412 - #597c95
            324  416 - #ff0000
            444  416 - #597c95
            512  416 - #597c95
            84  420 - #ff0000
            44  428 - #ff0000
            288  432 - #ff0000
            368  432 - #ff0000
            136  436 - #ff0000
            252  436 - #ff0000
            548  436 - #597c95
            216  440 - #ff0000
            396  444 - #ff0000
            4  452 - #ff00ff
            160  452 - #ff00ff
            344  452 - #ff00ff
            432  452 - #597c95
            500  452 - #597c95
            312  456 - #00ff00
            72  460 - #00ff00
            112  460 - #00ff00
            192  460 - #00ff00
            248  460 - #00ff00
            136  464 - #00ff00
            32  468 - #00ff00
            464  468 - #597c95
            272  472 - #00ff00
            368  472 - #00ff00
            532  476 - #597c95
            232  480 - #00ff00
            396  480 - #00ff00
            592  480 - #597c95
            64  484 - #00ff00
            4  488 - #ff00ff
            112  488 - #ff00ff
            140  488 - #ff00ff
            180  488 - #ff00ff
            340  488 - #ff00ff
            432  488 - #597c95
            208  492 - #0000e7
            308  492 - #0000e7
            88  500 - #0000e7
            48  504 - #0000e7
            276  504 - #0000e7
            372  504 - #0000e7
            472  504 - #597c95
            120  512 - #0000e7
            536  512 - #597c95
            244  516 - #0000e7
            328  516 - #0000e7
            180  520 - #ff00ff
            204  520 - #ff00ff
            300  520 - #ff00ff
            352  520 - #ff00ff
            396  520 - #ff00ff
            4  524 - #ff0000
            156  524 - #ff0000
            440  524 - #597c95
            32  528 - #ff0000
            500  528 - #597c95
            84  532 - #ff0000
            564  536 - #597c95
            228  540 - #ff0000
            272  540 - #ff0000
            368  540 - #ff0000
            56  544 - #ff0000
            112  544 - #ff0000
            332  544 - #ff0000
            304  548 - #ff0000
            180  552 - #ff0000
            144  556 - #ff0000
            248  560 - #ff0000
            4  564 - #ff0000
            216  564 - #ff0000
            436  564 - #597c95
            40  568 - #ff00ff
            88  568 - #ff00ff
            368  568 - #ff00ff
            396  568 - #ff00ff
            496  568 - #597c95
            280  572 - #0000e7
            324  572 - #0000e7
            64  580 - #0000e7
            128  584 - #0000e7
            160  588 - #0000e7
            352  588 - #0000e7
            528  588 - #597c95
            28  592 - #0000e7
            100  592 - #0000e7
            196  592 - #0000e7
            228  592 - #0000e7
            256  592 - #0000e7
            304  592 - #0000e7
            380  592 - #0000e7
            464  592 - #597c95
            592  592 - #597c95
";

const BOTTOM: &str = r"
            36    4 - #00ff00
            64    4 - #00ff00
            92    4 - #00ff00
            200    4 - #00ff00
            236    4 - #00ff00
            268    4 - #00ff00
            300    4 - #00ff00
            332    4 - #00ff00
            372    4 - #00ff00
            396    4 - #00ff00
            424    4 - #597c95
            464    4 - #597c95
            592    4 - #597c95
            160    8 - #00ff00
            128   12 - #00ff00
            4   16 - #00ff00
            528   16 - #597c95
            564   24 - #597c95
            284   28 - #00ff00
            396   28 - #00ff00
            56   32 - #ff00ff
            224   32 - #ff00ff
            340   32 - #ff00ff
            368   32 - #ff00ff
            28   36 - #0000e7
            192   36 - #0000e7
            312   36 - #0000e7
            260   40 - #0000e7
            488   40 - #597c95
            88   44 - #0000e7
            124   44 - #0000e7
            160   44 - #0000e7
            4   48 - #0000e7
            444   48 - #597c95
            532   52 - #597c95
            352   56 - #0000e7
            52   60 - #0000e7
            296   60 - #0000e7
            324   60 - #0000e7
            568   60 - #597c95
            24   64 - #ff00ff
            144   64 - #ff00ff
            396   64 - #ff00ff
            188   68 - #ff0000
            228   72 - #ff0000
            72   76 - #ff0000
            268   76 - #ff0000
            508   80 - #597c95
            108   84 - #ff0000
            316   84 - #ff0000
            344   88 - #ff0000
            372   88 - #ff0000
            144   92 - #ff0000
            40   96 - #ff0000
            244   96 - #ff0000
            448   96 - #597c95
            548   96 - #597c95
            180  100 - #ff0000
            80  104 - #ff0000
            4  108 - #ff0000
            220  112 - #ff00ff
            272  112 - #ff00ff
            316  112 - #ff00ff
            396  112 - #ff00ff
            484  112 - #597c95
            592  112 - #597c95
            356  116 - #0000e7
            104  120 - #0000e7
            156  124 - #0000e7
            56  128 - #0000e7
            296  128 - #0000e7
            128  132 - #0000e7
            532  132 - #597c95
            24  136 - #0000e7
            184  136 - #0000e7
            256  136 - #0000e7
            328  136 - #0000e7
            432  136 - #597c95
            380  140 - #0000e7
            80  144 - #ff00ff
            4  152 - #00ff00
            112  152 - #00ff00
            144  152 - #00ff00
            472  152 - #597c95
            220  156 - #00ff00
            356  156 - #00ff00
            176  160 - #00ff00
            580  160 - #597c95
            56  164 - #00ff00
            396  164 - #00ff00
            292  168 - #00ff00
            20  172 - #00ff00
            88  172 - #00ff00
            124  180 - #ff00ff
            160  180 - #ff00ff
            256  180 - #ff00ff
            328  180 - #ff00ff
            192  184 - #ff0000
            436  184 - #597c95
            496  188 - #597c95
            540  188 - #597c95
            72  196 - #ff0000
            224  196 - #ff0000
            284  196 - #ff0000
            368  196 - #ff0000
            396  200 - #ff0000
            12  204 - #ff0000
            340  204 - #ff0000
            48  208 - #ff0000
            96  208 - #ff0000
            132  212 - #ff0000
            312  212 - #ff0000
            592  212 - #597c95
            172  216 - #ff0000
            464  220 - #597c95
            248  224 - #ff00ff
            360  224 - #ff00ff
            208  228 - #00ff00
            284  228 - #00ff00
            396  228 - #00ff00
            32  232 - #00ff00
            432  232 - #597c95
            4  236 - #00ff00
            104  236 - #00ff00
            144  240 - #00ff00
            188  240 - #00ff00
            332  240 - #00ff00
            560  240 - #597c95
            76  244 - #00ff00
            224  248 - #00ff00
            304  248 - #00ff00
            520  248 - #597c95
            368  252 - #00ff00
            168  256 - #00ff00
            468  256 - #597c95
            8  260 - #ff00ff
            36  260 - #ff00ff
            120  260 - #ff00ff
            200  260 - #ff00ff
            244  260 - #ff00ff
            280  260 - #ff00ff
            392  260 - #ff00ff
            420  264 - #597c95
            60  268 - #0000e7
            320  268 - #0000e7
            348  268 - #0000e7
            588  272 - #597c95
            144  276 - #0000e7
            104  280 - #0000e7
            180  280 - #0000e7
            496  280 - #597c95
            548  280 - #597c95
            4  284 - #0000e7
            268  284 - #0000e7
            76  288 - #0000e7
            232  288 - #0000e7
            296  288 - #0000e7
            384  288 - #0000e7
            456  288 - #597c95
            204  292 - #ff00ff
            340  292 - #ff00ff
            156  300 - #ff0000
            36  304 - #ff0000
            112  308 - #ff0000
            252  308 - #ff0000
            420  308 - #597c95
            184  312 - #ff0000
            316  312 - #ff0000
            524  312 - #597c95
            368  320 - #ff0000
            484  320 - #597c95
            576  320 - #597c95
            4  324 - #ff0000
            72  328 - #ff0000
            284  328 - #ff0000
            140  332 - #ff0000
            216  332 - #ff0000
            176  340 - #ff00ff
            308  340 - #ff00ff
            336  340 - #ff00ff
            448  340 - #597c95
            396  344 - #0000e7
            36  348 - #0000e7
            100  348 - #0000e7
            364  348 - #0000e7
            152  356 - #0000e7
            248  356 - #0000e7
            540  360 - #597c95
            296  364 - #0000e7
            128  368 - #0000e7
            592  368 - #597c95
            4  372 - #ff00ff
            72  372 - #ff00ff
            104  372 - #ff00ff
            208  372 - #ff00ff
            272  372 - #ff00ff
            324  372 - #ff00ff
            376  372 - #ff00ff
            416  372 - #597c95
            32  376 - #00ff00
            172  376 - #00ff00
            488  376 - #597c95
            236  380 - #00ff00
            300  388 - #00ff00
            348  388 - #00ff00
            112  396 - #00ff00
            264  396 - #00ff00
            372  396 - #00ff00
            444  396 - #597c95
            40  400 - #00ff00
            148  404 - #00ff00
            396  404 - #00ff00
            76  408 - #ff00ff
            228  408 - #ff00ff
            332  408 - #ff00ff
            4  412 - #ff0000
            188  412 - #ff0000
            304  412 - #ff0000
            544  412 - #597c95
            588  412 - #597c95
            356  420 - #ff0000
            424  424 - #597c95
            484  424 - #597c95
            56  428 - #ff0000
            108  428 - #ff0000
            212  428 - #ff0000
            136  432 - #ff0000
            388  432 - #ff0000
            4  440 - #ff0000
            32  440 - #ff0000
            248  440 - #ff0000
            284  440 - #ff0000
            324  448 - #ff0000
            536  448 - #597c95
            72  452 - #ff00ff
            100  452 - #ff00ff
            156  452 - #ff00ff
            200  452 - #ff00ff
            364  452 - #ff00ff
            444  452 - #597c95
            224  456 - #00ff00
            592  456 - #597c95
            300  464 - #00ff00
            396  464 - #00ff00
            4  468 - #00ff00
            340  468 - #00ff00
            508  468 - #597c95
            116  472 - #00ff00
            64  476 - #00ff00
            144  476 - #00ff00
            240  476 - #00ff00
            276  476 - #00ff00
            472  480 - #597c95
            552  480 - #597c95
            40  484 - #00ff00
            172  484 - #00ff00
            320  488 - #ff00ff
            364  488 - #ff00ff
            208  492 - #0000e7
            296  492 - #0000e7
            436  492 - #597c95
            80  496 - #0000e7
            256  496 - #0000e7
            396  500 - #0000e7
            144  504 - #0000e7
            4  508 - #0000e7
            500  508 - #597c95
            588  512 - #597c95
            300  516 - #0000e7
            40  520 - #ff00ff
            68  520 - #ff00ff
            108  520 - #ff00ff
            180  520 - #ff00ff
            228  520 - #ff00ff
            264  520 - #ff00ff
            328  520 - #ff00ff
            356  520 - #ff00ff
            464  528 - #597c95
            428  532 - #597c95
            532  532 - #597c95
            88  540 - #ff0000
            204  540 - #ff0000
            392  540 - #ff0000
            568  540 - #597c95
            60  548 - #ff0000
            152  548 - #ff0000
            8  552 - #ff0000
            292  552 - #ff0000
            496  552 - #597c95
            332  556 - #ff0000
            240  560 - #ff0000
            112  564 - #ff0000
            28  568 - #ff00ff
            84  568 - #ff00ff
            136  568 - #ff00ff
            168  568 - #ff00ff
            192  568 - #ff00ff
            216  568 - #ff00ff
            268  568 - #ff00ff
            368  568 - #ff00ff
            592  568 - #597c95
            52  576 - #0000e7
            448  576 - #597c95
            4  580 - #0000e7
            396  580 - #000096
            396  584 - #000096
            248  588 - #0000e7
            312  588 - #0000e7
            396  588 - #000096
            28  592 - #0000e7
            76  592 - #0000e7
            120  592 - #0000e7
            152  592 - #0000e7
            180  592 - #0000e7
            220  592 - #0000e7
            276  592 - #0000e7
            352  592 - #0000e7
            396  592 - #000096
            496  592 - #597c95
            548  592 - #597c95
";

// The backdrop has a color no cell uses, so the gaps expose it.
#[view]
struct TableVariableHeights {
    #[init]
    under: Container,
    table: TableView,
}

impl Setup for TableVariableHeights {
    fn setup(mut self: Weak<Self>) {
        self.under.set_color(PURPLE);
        self.under.place().tl(0).size(400, 600);

        self.table.place().tl(0).size(400, 600);
        self.table.set_data_source(self).register_cell::<Container>();
        self.table.set_cell_spacing(SPACING);
        self.table.set_variable_heights(true);
        self.table.reload_data();
    }
}

impl TableData for TableVariableHeights {
    fn cell_height(&self, index: usize) -> f32 {
        height(index)
    }

    fn number_of_cells(&self) -> usize {
        ROWS
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let cell = registry.cell::<Container>();
        cell.set_color(if is_tall(index) {
            RED
        } else if index.is_multiple_of(2) {
            GREEN
        } else {
            BLUE
        });
        cell
    }

    fn cell_selected(&mut self, index: usize) {
        *SELECTED.lock() += &format!("|{index}|");
    }
}

/// Where row `index` starts in the content, the sum of the rows above.
fn top(index: usize) -> f32 {
    (0..index).map(|i| height(i) + SPACING).sum()
}

impl ViewTest for TableVariableHeights {
    fn perform_test(mut view: Weak<Self>) -> Result<()> {
        set_record_probe_count(320);
        SELECTED.lock().clear();

        // Rows 0 to 2 are 30, 30 and 42 tall, each frame follows the
        // rows above it.
        from_main(move || {
            let cells: Vec<_> = view
                .table
                .scroll
                .content
                .subviews()
                .iter()
                .filter(|cell| !cell.is_hidden())
                .map(|cell| (cell.tag(), cell.y(), cell.height()))
                .collect();

            for (index, y, cell_height) in &cells {
                assert!((*y - top(*index)).abs() < 0.01, "row {index}");
                assert!((*cell_height - height(*index)).abs() < 0.01, "row {index}");
            }

            let total: f32 = top(ROWS) - SPACING;
            assert!((view.table.scroll.content_height() - total).abs() < 0.01);
        });

        check_colors(TOP)?;

        // A tap lands on the row under it, and a tap in a gap goes to
        // the closer row. Row 2 is the tall one at 68 to 110.
        inject_touches(
            "
                200   10   b
                200   10   e
                200   50   b
                200   50   e
                200   90   b
                200   90   e
                200  109   b
                200  109   e
                200  113   b
                200  113   e
            ",
        );

        assert_eq!(SELECTED.lock().deref(), "|0||1||2||2||3|");
        SELECTED.lock().clear();

        // Far down the list, the visible rows still sit where the sum
        // of the heights above puts them.
        inject_scroll(-20_000);

        from_main(move || {
            let offset = view.table.scroll.get_scroll_content_offset();
            let cells: Vec<_> = view
                .table
                .scroll
                .content
                .subviews()
                .iter()
                .filter(|cell| !cell.is_hidden())
                .map(|cell| (cell.tag(), cell.y()))
                .collect();

            assert!(!cells.is_empty());

            for (index, y) in &cells {
                assert!((*y - top(*index)).abs() < 0.01, "row {index}");
                assert!(*y + offset > -TALL - SPACING, "row {index} is above the viewport");
            }
        });

        // A tap at the bottom picks the row the offset says is there.
        let expected = from_main(move || {
            let offset = view.table.scroll.get_scroll_content_offset();
            let y = 590.0 - offset;
            (0..ROWS)
                .find(|index| top(*index) + height(*index) + SPACING / 2.0 >= y)
                .unwrap()
        });

        inject_touches("200 590 b\n200 590 e");
        assert_eq!(SELECTED.lock().deref(), &format!("|{expected}|"));
        SELECTED.lock().clear();

        // The last row lands flush with the table bottom.
        from_main(move || view.table.scroll_to_bottom());

        check_colors(BOTTOM)?;

        inject_touches("200 595 b\n200 595 e");
        assert_eq!(SELECTED.lock().deref(), &format!("|{}|", ROWS - 1));

        Ok(())
    }
}
