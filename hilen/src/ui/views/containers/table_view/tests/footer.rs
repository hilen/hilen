use std::ops::Deref;

use anyhow::Result;
use parking_lot::Mutex;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    gm::color::Color,
    ui::{CellRegistry, Container, Setup, TableData, TableView, View, ViewData, ViewTest, view},
    ui_test::{check_colors, inject_touches, set_record_probe_count},
};

static SELECTED: Mutex<String> = Mutex::new(String::new());

const BACKDROP: Color = Color::hex("#1b2230");
const PALETTE: [Color; 4] = [
    Color::hex("#4f8cff"),
    Color::hex("#3ecf8e"),
    Color::hex("#f5b942"),
    Color::hex("#f06b7a"),
];

// Colored rows over a backdrop no cell uses. Scrolled to the bottom the
// last row ends 30 points above the table bottom, so that strip shows
// the backdrop, a tap in it selects nothing and a tap just above it
// selects the last index.
#[view]
struct TableFooter {
    #[init]
    under: Container,
    table: TableView,
}

impl Setup for TableFooter {
    fn setup(mut self: Weak<Self>) {
        self.under.set_color(BACKDROP);
        self.under.place().tl(0).size(400, 600);

        self.table.place().tl(0).size(400, 600);
        self.table.set_data_source(self).register_cell::<Container>();
        self.table.set_cell_spacing(10);
        self.table.set_cell_margins(20, 20);
        self.table.set_footer_height(30);
        self.table.reload_data();
    }
}

impl TableData for TableFooter {
    fn cell_height(&self, _: usize) -> f32 {
        80.0
    }

    fn number_of_cells(&self) -> usize {
        100
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

const SCROLLED_TO_BOTTOM: &str = r"
              32    4 - #3ecf8e
              76    4 - #3ecf8e
             204    4 - #3ecf8e
             244    4 - #3ecf8e
             324    4 - #3ecf8e
             376    4 - #3ecf8e
             476    4 - #597c95
             568    4 - #597c95
             124   16 - #3ecf8e
             172   24 - #3ecf8e
              96   28 - #3ecf8e
             284   28 - #3ecf8e
             356   28 - #3ecf8e
             396   32 - #1b2230
             232   36 - #1b2230
             320   40 - #f5b942
              20   44 - #f5b942
              64   44 - #f5b942
             592   44 - #597c95
             144   52 - #f5b942
             448   56 - #597c95
             208   60 - #f5b942
             536   60 - #597c95
             100   68 - #f5b942
             296   68 - #f5b942
             256   76 - #f5b942
             336   80 - #f5b942
             388   80 - #1b2230
              52   84 - #f5b942
             160   88 - #f5b942
             592   92 - #597c95
               4   96 - #1b2230
             196   96 - #f5b942
             488  100 - #597c95
             128  108 - #f5b942
             440  108 - #597c95
              92  116 - #f5b942
             244  116 - #f5b942
             288  116 - #f5b942
             356  116 - #f5b942
             164  128 - #1b2230
             396  128 - #1b2230
              44  132 - #f06b7a
             204  132 - #f06b7a
               8  136 - #1b2230
             544  136 - #597c95
              76  144 - #f06b7a
             248  148 - #f06b7a
             328  148 - #f06b7a
             368  148 - #f06b7a
             132  152 - #f06b7a
             288  152 - #f06b7a
             456  156 - #597c95
             172  160 - #f06b7a
               4  172 - #1b2230
              96  172 - #f06b7a
             220  176 - #f06b7a
             264  176 - #f06b7a
             304  180 - #f06b7a
             356  180 - #f06b7a
             592  180 - #597c95
              56  184 - #f06b7a
             396  188 - #1b2230
             180  192 - #f06b7a
             508  192 - #597c95
             136  200 - #f06b7a
              96  220 - #4f8cff
             212  220 - #4f8cff
             244  220 - #4f8cff
             304  220 - #4f8cff
             348  220 - #4f8cff
              20  224 - #4f8cff
             276  224 - #4f8cff
              60  228 - #4f8cff
             124  228 - #4f8cff
             176  228 - #4f8cff
             396  228 - #1b2230
             580  232 - #597c95
             148  244 - #4f8cff
             452  244 - #597c95
              36  248 - #4f8cff
             204  248 - #4f8cff
             248  252 - #4f8cff
             280  252 - #4f8cff
             324  252 - #4f8cff
             176  256 - #4f8cff
             532  256 - #597c95
             104  260 - #4f8cff
             376  260 - #4f8cff
              64  268 - #4f8cff
             132  268 - #4f8cff
              20  272 - #4f8cff
             312  276 - #4f8cff
             240  280 - #4f8cff
             164  284 - #4f8cff
             204  284 - #4f8cff
             336  288 - #4f8cff
             592  288 - #597c95
             368  292 - #4f8cff
              56  296 - #4f8cff
              88  296 - #4f8cff
             124  296 - #4f8cff
             264  296 - #4f8cff
             292  296 - #4f8cff
             448  296 - #597c95
             400  300 - #597c95
             544  304 - #597c95
             496  312 - #597c95
              32  316 - #3ecf8e
             192  316 - #3ecf8e
             156  320 - #3ecf8e
             356  320 - #3ecf8e
             260  324 - #3ecf8e
             316  324 - #3ecf8e
             116  332 - #3ecf8e
             224  332 - #3ecf8e
             584  340 - #597c95
              84  344 - #3ecf8e
              40  348 - #3ecf8e
             384  348 - #1b2230
             144  352 - #3ecf8e
             180  352 - #3ecf8e
               4  356 - #1b2230
             336  356 - #3ecf8e
             448  356 - #597c95
             212  360 - #3ecf8e
             292  360 - #3ecf8e
             536  364 - #597c95
             116  368 - #3ecf8e
             240  368 - #3ecf8e
              80  376 - #3ecf8e
              48  380 - #3ecf8e
             160  384 - #3ecf8e
             196  384 - #3ecf8e
             356  388 - #3ecf8e
             396  396 - #1b2230
             592  396 - #597c95
             108  400 - #f5b942
             228  400 - #f5b942
             272  400 - #f5b942
             316  400 - #f5b942
               4  404 - #1b2230
             488  412 - #597c95
             160  420 - #f5b942
             196  420 - #f5b942
             372  424 - #f5b942
              56  432 - #f5b942
              92  436 - #f5b942
             228  436 - #f5b942
              16  440 - #1b2230
             336  440 - #f5b942
             288  444 - #f5b942
             548  452 - #597c95
             124  456 - #f5b942
             396  456 - #1b2230
             452  456 - #597c95
             500  460 - #597c95
              40  468 - #f5b942
             180  472 - #f5b942
             224  472 - #f5b942
               4  476 - #1b2230
             312  480 - #1b2230
             108  484 - #1b2230
             264  484 - #1b2230
              72  488 - #1b2230
             392  488 - #1b2230
             152  492 - #f06b7a
             360  492 - #f06b7a
             484  504 - #597c95
             220  508 - #f06b7a
             292  508 - #f06b7a
             592  508 - #597c95
             124  512 - #f06b7a
              32  520 - #f06b7a
             324  520 - #f06b7a
             396  520 - #1b2230
             260  524 - #f06b7a
             356  524 - #f06b7a
             176  532 - #f06b7a
             536  540 - #597c95
              76  544 - #f06b7a
             284  556 - #f06b7a
             320  556 - #f06b7a
             360  556 - #f06b7a
             396  556 - #1b2230
             448  556 - #597c95
             228  560 - #f06b7a
              20  568 - #f06b7a
             128  568 - #f06b7a
             292  588 - #1b2230
              64  592 - #1b2230
             100  592 - #1b2230
             180  592 - #1b2230
             216  592 - #1b2230
             260  592 - #1b2230
             324  592 - #1b2230
             360  592 - #1b2230
             396  592 - #12161f
             500  592 - #597c95
             588  592 - #597c95
";

const NO_FOOTER: &str = r"
               4    4 - #1b2230
              76    4 - #3ecf8e
             200    4 - #3ecf8e
             240    4 - #3ecf8e
             320    4 - #3ecf8e
             376    4 - #3ecf8e
             468    4 - #597c95
             560    4 - #597c95
             164    8 - #3ecf8e
             280   16 - #3ecf8e
              40   20 - #3ecf8e
             120   20 - #3ecf8e
               4   40 - #1b2230
             220   40 - #3ecf8e
             592   40 - #597c95
             344   44 - #3ecf8e
             148   48 - #3ecf8e
             264   48 - #3ecf8e
              64   52 - #3ecf8e
             444   52 - #597c95
             300   56 - #3ecf8e
             392   56 - #1b2230
             524   56 - #597c95
             184   64 - #1b2230
             116   68 - #1b2230
              20   72 - #f5b942
             244   76 - #f5b942
             324   88 - #f5b942
             592   88 - #597c95
             284   96 - #f5b942
             476   96 - #597c95
              72  100 - #f5b942
             360  100 - #f5b942
              36  104 - #f5b942
             148  104 - #f5b942
             396  104 - #1b2230
             108  108 - #f5b942
             224  108 - #f5b942
               4  120 - #1b2230
             536  120 - #597c95
             192  124 - #f5b942
             256  128 - #f5b942
             312  128 - #f5b942
             440  128 - #597c95
              84  136 - #f5b942
             588  140 - #597c95
             124  144 - #f5b942
             396  144 - #1b2230
              48  148 - #f5b942
             164  152 - #1b2230
             284  152 - #1b2230
             212  156 - #1b2230
             348  156 - #1b2230
               4  172 - #1b2230
             388  176 - #1b2230
             448  176 - #597c95
             244  180 - #f06b7a
             512  180 - #597c95
             136  184 - #f06b7a
             300  184 - #f06b7a
              48  188 - #f06b7a
              88  192 - #f06b7a
             176  192 - #f06b7a
             212  192 - #f06b7a
             360  192 - #f06b7a
             592  192 - #597c95
             396  208 - #1b2230
             268  212 - #f06b7a
              20  220 - #f06b7a
             108  220 - #f06b7a
             300  220 - #f06b7a
             188  224 - #f06b7a
             224  224 - #f06b7a
             336  224 - #f06b7a
             140  228 - #f06b7a
              60  232 - #f06b7a
             464  240 - #597c95
             528  244 - #597c95
               4  248 - #1b2230
              96  252 - #4f8cff
             160  252 - #4f8cff
             268  252 - #4f8cff
             308  252 - #4f8cff
             128  256 - #4f8cff
             236  256 - #4f8cff
             376  256 - #4f8cff
             192  260 - #4f8cff
             340  260 - #4f8cff
              64  268 - #4f8cff
              28  272 - #4f8cff
             172  276 - #4f8cff
             584  276 - #597c95
             144  280 - #4f8cff
             232  288 - #4f8cff
             368  288 - #4f8cff
             448  288 - #597c95
             108  292 - #4f8cff
             284  292 - #4f8cff
             324  292 - #4f8cff
               4  296 - #1b2230
             188  296 - #4f8cff
              40  300 - #4f8cff
             256  300 - #4f8cff
             400  300 - #597c95
              72  304 - #4f8cff
             212  308 - #4f8cff
             496  308 - #597c95
             160  316 - #4f8cff
             548  316 - #597c95
             360  320 - #4f8cff
              20  324 - #4f8cff
             232  324 - #4f8cff
             332  324 - #4f8cff
              92  328 - #4f8cff
             124  328 - #4f8cff
             196  328 - #4f8cff
             264  328 - #4f8cff
             304  328 - #4f8cff
              48  348 - #3ecf8e
             152  348 - #3ecf8e
             396  348 - #1b2230
             456  348 - #597c95
              84  356 - #3ecf8e
             212  356 - #3ecf8e
             340  360 - #3ecf8e
             516  360 - #597c95
               4  364 - #1b2230
             180  364 - #3ecf8e
             292  368 - #3ecf8e
             568  368 - #597c95
             240  376 - #3ecf8e
             144  380 - #3ecf8e
              44  384 - #3ecf8e
             108  384 - #3ecf8e
              76  388 - #3ecf8e
             360  392 - #3ecf8e
             396  392 - #1b2230
              12  396 - #1b2230
             480  400 - #597c95
             196  404 - #3ecf8e
             328  408 - #3ecf8e
              96  416 - #3ecf8e
             236  416 - #3ecf8e
             280  416 - #3ecf8e
             560  420 - #597c95
              52  424 - #1b2230
             156  424 - #1b2230
               4  432 - #1b2230
             396  432 - #1b2230
             124  436 - #f5b942
             208  436 - #f5b942
             360  444 - #f5b942
             444  448 - #597c95
             232  456 - #f5b942
             312  456 - #f5b942
             520  456 - #597c95
             100  460 - #f5b942
             188  460 - #f5b942
             592  460 - #597c95
              56  464 - #f5b942
             144  468 - #f5b942
             272  468 - #f5b942
              16  472 - #1b2230
             396  480 - #1b2230
              80  484 - #f5b942
             344  484 - #f5b942
             240  496 - #f5b942
             124  500 - #f5b942
             164  500 - #f5b942
              48  504 - #f5b942
             292  504 - #f5b942
             468  504 - #597c95
             200  508 - #f5b942
               8  512 - #1b2230
              88  516 - #1b2230
             396  516 - #1b2230
             364  520 - #f06b7a
             324  528 - #f06b7a
             528  528 - #597c95
             144  532 - #f06b7a
             248  536 - #f06b7a
             216  540 - #f06b7a
             592  540 - #597c95
             180  544 - #f06b7a
              48  548 - #f06b7a
             288  548 - #f06b7a
               4  552 - #1b2230
             108  552 - #f06b7a
             396  552 - #1b2230
             360  560 - #f06b7a
             324  568 - #f06b7a
             208  572 - #f06b7a
             452  576 - #597c95
              84  584 - #f06b7a
              20  592 - #f06b7a
             148  592 - #f06b7a
             268  592 - #f06b7a
             380  592 - #1b2230
             528  592 - #597c95
             584  592 - #597c95
";

impl ViewTest for TableFooter {
    fn perform_test(mut view: Weak<Self>) -> Result<()> {
        set_record_probe_count(200);
        SELECTED.lock().clear();

        from_main(move || {
            view.table.scroll_to_bottom();
        });

        check_colors(SCROLLED_TO_BOTTOM)?;

        // 100 rows of 80 with 10 spacing is 8990 of rows, plus the
        // 30 footer against a 600 viewport puts row 99 at 490 to 570.
        inject_touches(
            "
                200  595  b
                200  595  e
                200  580  b
                200  580  e
                200  560  b
                200  560  e
                200  5    b
                200  5    e
            ",
        );
        assert_eq!(SELECTED.lock().deref(), "|99||93|");
        SELECTED.lock().clear();

        from_main(move || {
            view.table.set_footer_height(0);
            view.table.scroll_to_bottom();
        });

        check_colors(NO_FOOTER)?;

        inject_touches(
            "
                200  595  b
                200  595  e
            ",
        );
        assert_eq!(SELECTED.lock().deref(), "|99|");
        SELECTED.lock().clear();

        Ok(())
    }
}
