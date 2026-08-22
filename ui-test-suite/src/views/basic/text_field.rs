use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Anchor, Setup, ViewData, ViewTest, view},
    ui_test::{helpers::check_colors, inject_keys, inject_touches, set_record_probe_count},
};

const TYPED_TEXT_PROBES: &str = r"
               4    4 - #597c95
             232    4 - #597c95
             572    4 - #597c95
             324    8 - #597c95
             120   20 - #597c95
             404   56 - #597c95
             500   68 - #597c95
              32   96 - #597c95
             296  104 - #597c95
             632  108 - #597c95
             120  136 - #597c95
             212  152 - #597c95
             432  152 - #597c95
             528  164 - #597c95
               4  192 - #597c95
             624  204 - #597c95
             292  224 - #000000
             296  224 - #000000
             300  224 - #000000
             348  224 - #010101
             268  228 - #dadada
             280  228 - #000000
             288  228 - #000000
             296  228 - #ffffff
             308  228 - #000000
             324  228 - #000000
             372  228 - #000000
             280  232 - #000000
             288  232 - #000000
             308  232 - #000000
             324  232 - #000000
             340  232 - #020202
             344  232 - #ffffff
             348  232 - #ffffff
             352  232 - #ffffff
             280  236 - #000000
             288  236 - #000000
             300  236 - #ffffff
             308  236 - #000000
             324  236 - #000000
             340  236 - #020202
             344  236 - #ffffff
             348  236 - #ffffff
             352  236 - #ffffff
             356  236 - #0c0c0c
             368  236 - #000000
             268  240 - #dadada
             280  240 - #000000
             288  240 - #000000
             296  240 - #ffffff
             308  240 - #000000
             324  240 - #000000
             368  240 - #000000
             268  244 - #dadada
             292  244 - #000000
             296  244 - #000000
             300  244 - #000000
             308  244 - #000000
             312  244 - #000000
             316  244 - #000000
             328  244 - #000000
             332  244 - #000000
             348  244 - #010101
             472  248 - #597c95
              72  300 - #ffffff
             148  300 - #ffffff
             448  300 - #ffffff
             524  300 - #ffffff
             616  300 - #ffffff
             196  320 - #ffffff
             320  320 - #ffffff
             248  324 - #ffffff
             396  324 - #ffffff
             480  352 - #ffffff
              20  356 - #ffffff
             112  356 - #ffffff
             560  360 - #ffffff
             616  360 - #ffffff
             176  368 - #ffffff
             240  380 - #ffffff
             296  392 - #585858
             340  392 - #030303
             324  396 - #010101
             332  396 - #010101
             340  396 - #020202
             412  396 - #ffffff
             296  400 - #585858
             312  400 - #f0f0f0
             340  400 - #030303
             296  404 - #585858
             316  404 - #ffffff
             340  404 - #030303
              76  408 - #ffffff
             324  408 - #000000
             332  408 - #000000
             616  416 - #ffffff
             144  428 - #ffffff
             544  428 - #ffffff
              20  432 - #ffffff
             224  432 - #ffffff
             472  436 - #ffffff
             380  444 - #ffffff
              96  484 - #ffffff
             340  484 - #ffffff
             172  492 - #ffffff
              20  496 - #ffffff
             268  496 - #ffffff
             412  496 - #ffffff
             472  496 - #ffffff
             532  496 - #ffffff
             612  496 - #ffffff
             472  576 - #597c95
              88  584 - #597c95
             228  600 - #597c95
             340  604 - #597c95
             632  612 - #597c95
               4  640 - #597c95
             144  672 - #597c95
             492  676 - #597c95
             396  680 - #597c95
             596  700 - #597c95
             316  732 - #597c95
             100  764 - #597c95
             532  772 - #597c95
               4  792 - #597c95
             196  792 - #597c95
             432  792 - #597c95
             632  792 - #597c95
";

const LARGE_UNICODE_PROBES: &str = r"
               4    4 - #597c95
             312    4 - #597c95
             632    4 - #597c95
             160   60 - #597c95
             472   60 - #597c95
              36  152 - #597c95
             592  152 - #597c95
             220  212 - #ffffff
             252  212 - #ffffff
             284  212 - #ffffff
             324  212 - #ffffff
             356  212 - #ffffff
             396  212 - #ffffff
             292  224 - #000000
             348  224 - #010101
             268  228 - #dcdcdc
             296  228 - #ffffff
             372  228 - #000000
             416  228 - #ffffff
             240  232 - #ffffff
             344  232 - #ffffff
             348  232 - #ffffff
             352  232 - #ffffff
             220  236 - #ffffff
             300  236 - #ffffff
             344  236 - #ffffff
             348  236 - #ffffff
             352  236 - #ffffff
             268  240 - #dcdcdc
             296  240 - #ffffff
             368  240 - #000000
             396  240 - #ffffff
             268  244 - #dcdcdc
             292  244 - #000000
             312  244 - #000000
             328  244 - #000000
             348  244 - #000000
             224  256 - #ffffff
             248  256 - #ffffff
             284  256 - #ffffff
             360  256 - #ffffff
             380  256 - #ffffff
             416  256 - #ffffff
             540  300 - #bcbcbc
             616  300 - #bcbcbc
              20  304 - #bcbcbc
             112  324 - #010101
             136  328 - #000000
             208  328 - #000000
             216  328 - #000000
             284  328 - #000000
             124  332 - #000000
             132  332 - #000000
             280  332 - #000000
             308  332 - #000000
             212  336 - #000000
             388  344 - #292929
             392  344 - #292929
             396  344 - #292929
              96  348 - #3c3c3c
             108  348 - #3c3c3c
             120  348 - #3c3c3c
             128  348 - #3c3c3c
             136  348 - #3c3c3c
             144  348 - #3c3c3c
             152  348 - #3c3c3c
             160  348 - #3c3c3c
             184  348 - #3c3c3c
             188  348 - #3c3c3c
             200  348 - #3c3c3c
             208  348 - #3c3c3c
             216  348 - #3c3c3c
             224  348 - #3c3c3c
             232  348 - #3c3c3c
             240  348 - #3c3c3c
             260  348 - #3c3c3c
             264  348 - #3c3c3c
             324  348 - #3c3c3c
             328  348 - #3c3c3c
             452  348 - #3c3c3c
             460  348 - #3c3c3c
             468  348 - #3c3c3c
             484  348 - #3c3c3c
             568  352 - #bcbcbc
             316  384 - #010101
              20  388 - #bcbcbc
             276  388 - #010101
             492  388 - #a1a1a1
             496  388 - #a1a1a1
             500  388 - #a1a1a1
             504  388 - #a1a1a1
             508  388 - #a1a1a1
             396  392 - #010101
             128  396 - #000000
             236  396 - #000000
             184  400 - #000000
             352  400 - #010101
             604  400 - #bcbcbc
             544  408 - #000000
             436  412 - #010101
             300  420 - #000000
              72  432 - #bcbcbc
             220  440 - #000000
             476  440 - #000000
             136  444 - #010101
             524  444 - #000000
             272  448 - #000000
             388  448 - #010101
             580  448 - #bcbcbc
              20  472 - #bcbcbc
              88  496 - #bcbcbc
             180  496 - #bcbcbc
             284  496 - #bcbcbc
             348  496 - #bcbcbc
             424  496 - #bcbcbc
             488  496 - #bcbcbc
             556  496 - #bcbcbc
             616  496 - #bcbcbc
               4  632 - #597c95
             332  640 - #597c95
             624  644 - #597c95
             168  668 - #597c95
             480  680 - #597c95
               4  792 - #597c95
             200  792 - #597c95
             332  792 - #597c95
             512  792 - #597c95
             632  792 - #597c95
";

#[view]
struct TextField {
    #[init]
    field:      hilen::ui::TextField,
    smol_field: hilen::ui::TextField,
}

impl Setup for TextField {
    fn setup(self: Weak<Self>) {
        self.field.place().size(600, 200).center();
        self.smol_field
            .place()
            .size(200, 50)
            .center_x()
            .anchor(Anchor::Bot, self.field, 40);
    }
}

fn check_typed_text() -> Result<()> {
    inject_touches(
        r"
            389  576  b
            389  576  e
            399  292  b
            399  292  e
            427  147  b
            427  147  e
            391  237  b
            391  235  e
    ",
    );

    inject_keys("HELLOY");

    inject_touches(
        r"
            452  442  b
    ",
    );

    inject_keys("text");

    inject_touches(
        r"
            10  10  b
    ",
    );

    check_colors(TYPED_TEXT_PROBES)
}

fn check_large_unicode_text(view: Weak<TextField>) -> Result<()> {
    from_main(move || {
        view.field.set_text_size(140);
        view.field.clear();
    });

    inject_touches(
        r"
            452  442  b
    ",
    );

    inject_keys("ŽĖЎФЪ");

    check_colors(LARGE_UNICODE_PROBES)
}

impl ViewTest for TextField {
    // 640 is the width of the smallest supported screen, and the 600 wide field
    // needs almost all of it.
    fn canvas() -> (u32, u32) {
        (640, 800)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);

        check_typed_text()?;
        check_large_unicode_text(view)?;

        Ok(())
    }
}
