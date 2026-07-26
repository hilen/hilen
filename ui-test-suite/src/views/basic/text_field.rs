use anyhow::Result;
use test_engine::{
    dispatch::from_main,
    refs::Weak,
    ui::{Anchor, Setup, ViewData, ViewTest, view},
    ui_test::{helpers::check_colors, inject_keys, inject_touches, set_record_probe_count},
};

const TYPED_TEXT_PROBES: &str = r"
               4    4 - #597c95
             100    4 - #597c95
             212    4 - #597c95
             312    4 - #597c95
             508    4 - #597c95
             588    4 - #597c95
             448   44 - #597c95
             372   60 - #597c95
             260   64 - #597c95
             532   72 - #597c95
             156   84 - #597c95
              60   88 - #597c95
             632  108 - #597c95
             316  116 - #597c95
             228  136 - #597c95
             432  140 - #597c95
             108  144 - #597c95
             532  160 - #597c95
               8  164 - #597c95
             148  204 - #597c95
             632  208 - #597c95
              72  220 - #597c95
             264  220 - #ffffff
             268  220 - #ffffff
             272  220 - #ffffff
             360  220 - #ffffff
             368  220 - #ffffff
             372  220 - #ffffff
             376  220 - #ffffff
             292  228 - #ffffff
             296  228 - #ffffff
             344  228 - #ffffff
             348  232 - #ffffff
             352  232 - #ffffff
             496  232 - #597c95
             292  236 - #ffffff
             296  236 - #ffffff
             344  236 - #ffffff
             568  240 - #597c95
             224  256 - #ffffff
             416  256 - #ffffff
               4  260 - #597c95
              88  304 - #ffffff
             164  304 - #ffffff
             324  304 - #ffffff
             440  304 - #ffffff
             488  304 - #ffffff
             536  304 - #ffffff
             616  304 - #ffffff
              32  308 - #ffffff
             576  316 - #ffffff
             264  320 - #ffffff
             384  320 - #ffffff
             212  324 - #ffffff
             304  348 - #ffffff
             472  352 - #ffffff
              76  356 - #ffffff
             128  356 - #ffffff
             524  356 - #ffffff
              24  360 - #ffffff
             576  360 - #ffffff
             424  364 - #ffffff
             192  372 - #ffffff
             376  372 - #ffffff
             252  376 - #ffffff
             616  380 - #ffffff
             312  396 - #ffffff
             328  400 - #000000
             312  404 - #ffffff
             480  404 - #ffffff
             576  404 - #ffffff
              92  408 - #ffffff
             412  412 - #ffffff
             532  412 - #ffffff
              28  424 - #ffffff
             368  424 - #ffffff
             616  424 - #ffffff
             156  432 - #ffffff
             240  432 - #ffffff
             112  448 - #ffffff
             292  448 - #ffffff
             200  452 - #ffffff
             512  452 - #ffffff
              68  456 - #ffffff
             420  456 - #ffffff
             464  456 - #ffffff
             560  456 - #ffffff
             340  464 - #ffffff
             144  476 - #ffffff
             236  484 - #ffffff
              24  488 - #ffffff
             104  492 - #ffffff
             184  496 - #ffffff
             284  496 - #ffffff
             400  496 - #ffffff
             444  496 - #ffffff
             520  496 - #ffffff
             596  496 - #ffffff
             340  544 - #597c95
               4  560 - #597c95
             168  572 - #597c95
             456  580 - #597c95
              92  592 - #597c95
             544  592 - #597c95
             236  604 - #597c95
             632  608 - #597c95
             352  616 - #597c95
               4  636 - #597c95
             424  640 - #597c95
              76  656 - #597c95
             292  660 - #597c95
             144  676 - #597c95
             488  676 - #597c95
             588  700 - #597c95
             232  704 - #597c95
             400  704 - #597c95
              40  712 - #597c95
             168  736 - #597c95
             316  740 - #597c95
             472  740 - #597c95
             104  768 - #597c95
             532  784 - #597c95
               4  792 - #597c95
             200  792 - #597c95
             272  792 - #597c95
             360  792 - #597c95
             432  792 - #597c95
             632  792 - #597c95
";

const LARGE_UNICODE_PROBES: &str = r"
               4    4 - #597c95
             272    4 - #597c95
             632    4 - #597c95
             140   28 - #597c95
             452   32 - #597c95
             548   56 - #597c95
             348   96 - #597c95
             228  104 - #597c95
             468  128 - #597c95
              76  148 - #597c95
             564  152 - #597c95
             224  212 - #ffffff
             244  212 - #ffffff
             300  212 - #ffffff
             320  212 - #ffffff
             340  212 - #ffffff
             396  212 - #ffffff
             416  212 - #ffffff
             264  220 - #ffffff
             268  220 - #ffffff
             272  220 - #ffffff
             360  220 - #ffffff
             368  220 - #ffffff
             372  220 - #ffffff
             376  220 - #ffffff
             292  228 - #ffffff
             296  228 - #ffffff
             344  228 - #ffffff
             240  232 - #ffffff
             316  232 - #ffffff
             348  232 - #ffffff
             352  232 - #ffffff
             396  232 - #ffffff
             272  236 - #ffffff
             292  236 - #ffffff
             296  236 - #ffffff
             344  236 - #ffffff
             224  240 - #ffffff
             376  240 - #ffffff
             416  240 - #ffffff
             252  244 - #ffffff
             268  252 - #ffffff
             304  252 - #ffffff
             236  256 - #ffffff
             288  256 - #ffffff
             320  256 - #ffffff
             340  256 - #ffffff
             368  256 - #ffffff
             396  256 - #ffffff
             412  256 - #ffffff
              24  304 - #bcbcbc
             452  304 - #bcbcbc
             612  304 - #bcbcbc
             552  308 - #bcbcbc
             368  312 - #bcbcbc
             200  316 - #000000
             204  316 - #000000
             104  320 - #010101
             116  320 - #010101
             120  320 - #010101
             200  320 - #000000
             204  320 - #000000
             276  320 - #010101
             280  320 - #010101
             304  320 - #010101
             308  320 - #010101
             312  320 - #010101
             200  324 - #000000
             204  324 - #000000
             500  340 - #000000
              76  344 - #010101
             232  344 - #000000
             464  344 - #000000
             144  348 - #010101
             408  348 - #010101
             372  352 - #010101
             608  364 - #bcbcbc
             312  368 - #010101
             168  376 - #000000
             244  376 - #bcbcbc
             108  380 - #010101
             512  380 - #010101
             352  384 - #010101
              24  388 - #bcbcbc
             208  388 - #000000
             280  388 - #010101
             556  388 - #010101
             400  392 - #010101
             444  396 - #010101
             168  416 - #000000
             368  416 - #010101
             296  420 - #010101
             496  420 - #010101
             616  420 - #bcbcbc
              72  428 - #000000
             208  428 - #000000
             132  432 - #000000
             396  436 - #010101
             252  440 - #bcbcbc
              24  444 - #bcbcbc
             344  444 - #bcbcbc
             452  448 - #bcbcbc
             552  448 - #bcbcbc
              60  472 - #bcbcbc
             284  476 - #bcbcbc
             384  484 - #bcbcbc
             164  492 - #bcbcbc
              24  496 - #bcbcbc
              96  496 - #bcbcbc
             232  496 - #bcbcbc
             336  496 - #bcbcbc
             436  496 - #bcbcbc
             512  496 - #bcbcbc
             592  496 - #bcbcbc
             544  592 - #597c95
             392  604 - #597c95
             276  620 - #597c95
               4  640 - #597c95
             632  640 - #597c95
             144  664 - #597c95
             488  676 - #597c95
             320  744 - #597c95
              96  752 - #597c95
             536  764 - #597c95
               4  792 - #597c95
             192  792 - #597c95
             436  792 - #597c95
             632  792 - #597c95
";

#[view]
struct TextField {
    #[init]
    field:      test_engine::ui::TextField,
    smol_field: test_engine::ui::TextField,
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
