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
             268  228 - #d8d8d8
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
             340  232 - #000000
             344  232 - #ffffff
             348  232 - #ffffff
             352  232 - #ffffff
             280  236 - #000000
             288  236 - #000000
             300  236 - #ffffff
             308  236 - #000000
             324  236 - #000000
             340  236 - #000000
             344  236 - #ffffff
             348  236 - #ffffff
             352  236 - #ffffff
             368  236 - #000000
             268  240 - #d8d8d8
             280  240 - #000000
             288  240 - #000000
             296  240 - #ffffff
             308  240 - #000000
             324  240 - #000000
             368  240 - #000000
             292  244 - #000000
             296  244 - #000000
             300  244 - #000000
             308  244 - #000000
             312  244 - #000000
             316  244 - #000000
             328  244 - #000000
             332  244 - #000000
             348  244 - #000000
             472  248 - #597c95
              72  300 - #ffffff
             148  300 - #ffffff
             448  300 - #ffffff
             524  300 - #ffffff
             616  300 - #ffffff
              20  304 - #ffffff
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
             296  392 - #565656
             340  392 - #000000
             324  396 - #000000
             340  396 - #000000
             412  396 - #ffffff
             296  400 - #565656
             312  400 - #e6e6e6
             328  400 - #040404
             340  400 - #000000
             296  404 - #565656
             316  404 - #ffffff
             340  404 - #000000
              76  408 - #ffffff
             324  408 - #010101
             332  408 - #010101
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
             232  696 - #597c95
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
             332    4 - #597c95
             632    4 - #597c95
             168   76 - #597c95
             484   96 - #597c95
             280  108 - #597c95
              36  152 - #597c95
             624  152 - #597c95
             148  204 - #597c95
             480  204 - #597c95
             292  224 - #000000
             348  224 - #010101
             268  228 - #d8d8d8
             296  228 - #ffffff
             372  228 - #000000
             344  232 - #ffffff
             348  232 - #ffffff
             352  232 - #ffffff
             300  236 - #ffffff
             344  236 - #ffffff
             348  236 - #ffffff
             352  236 - #ffffff
             268  240 - #d8d8d8
             296  240 - #ffffff
             368  240 - #000000
             292  244 - #000000
             312  244 - #000000
             328  244 - #000000
             348  244 - #000000
             220  256 - #ffffff
             356  300 - #ffffff
             428  300 - #ffffff
             540  300 - #ffffff
             616  300 - #ffffff
              20  304 - #ffffff
             112  324 - #010101
             136  328 - #010101
             208  328 - #010101
             216  328 - #010101
             284  328 - #000000
             124  332 - #000000
             132  332 - #000000
             216  332 - #000000
             280  332 - #010101
             284  332 - #010101
             304  332 - #010101
             308  332 - #010101
             212  336 - #000000
             216  336 - #000000
             388  344 - #454545
             392  344 - #454545
             396  344 - #454545
              96  348 - #5f5f5f
             108  348 - #5f5f5f
             120  348 - #5f5f5f
             128  348 - #5f5f5f
             136  348 - #5f5f5f
             144  348 - #5f5f5f
             152  348 - #5f5f5f
             160  348 - #5f5f5f
             184  348 - #5f5f5f
             188  348 - #5f5f5f
             200  348 - #5f5f5f
             208  348 - #5f5f5f
             216  348 - #5f5f5f
             224  348 - #5f5f5f
             228  348 - #5f5f5f
             232  348 - #5f5f5f
             236  348 - #5f5f5f
             240  348 - #5f5f5f
             260  348 - #5f5f5f
             264  348 - #5f5f5f
             268  348 - #5f5f5f
             324  348 - #5f5f5f
             328  348 - #5f5f5f
             332  348 - #5f5f5f
             452  348 - #5f5f5f
             460  348 - #5f5f5f
             468  348 - #5f5f5f
             484  348 - #5f5f5f
             560  356 - #ffffff
             360  368 - #000000
             312  380 - #010101
             276  384 - #010101
              20  388 - #ffffff
             396  388 - #010101
             492  388 - #e7e7e7
             496  388 - #e7e7e7
             500  388 - #e7e7e7
             504  388 - #e7e7e7
             508  388 - #e7e7e7
             128  396 - #000000
             236  396 - #000000
             440  396 - #010101
             604  400 - #ffffff
             356  404 - #010101
             540  404 - #010101
             188  408 - #000000
             300  412 - #010101
             108  424 - #000000
              72  432 - #ffffff
             408  436 - #010101
             532  440 - #010101
             136  444 - #000000
             196  444 - #000000
             236  444 - #000000
             476  444 - #010101
             276  448 - #000000
              20  472 - #ffffff
             380  484 - #ffffff
             560  492 - #ffffff
              88  496 - #ffffff
             188  496 - #ffffff
             268  496 - #ffffff
             328  496 - #ffffff
             436  496 - #ffffff
             504  496 - #ffffff
             616  496 - #ffffff
               4  628 - #597c95
             316  644 - #597c95
             624  644 - #597c95
             156  648 - #597c95
             468  688 - #597c95
               4  792 - #597c95
             152  792 - #597c95
             304  792 - #597c95
             420  792 - #597c95
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
