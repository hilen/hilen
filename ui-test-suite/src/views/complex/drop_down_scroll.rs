use anyhow::Result;
use hilen::{
    dispatch::wait_for_next_frame,
    refs::Weak,
    ui::{Color, Setup, TextDropDown, ViewData, ViewFrame, ViewTest, view},
    ui_test::{check_colors, inject_scroll, inject_touches, set_record_probe_count},
};

/// A list with more rows than the window has room for. The open panel
/// stops short of the window edge and scrolls, a short list next to it
/// stays as tall as its rows and does not move.
#[view]
struct DropDownScroll {
    #[init]
    long:  TextDropDown,
    short: TextDropDown,
}

impl Setup for DropDownScroll {
    fn setup(self: Weak<Self>) {
        self.set_color(Color::rgb(0.92, 0.94, 0.97));

        self.long.set_values((1..=40).map(|row| format!("Row {row}")));
        self.long.place().t(40).l(40).size(200, 40);

        self.short.set_values(["One", "Two", "Three"]);
        self.short.place().t(40).l(320).size(200, 40);
    }
}

impl ViewTest for DropDownScroll {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);

        inject_touches("140 60 b\n140 60 e");
        check_colors(COLORS_1)?;

        // The pointer rests on the list, the wheel moves it.
        inject_touches("140 300 m");
        inject_scroll(-400);
        wait_for_next_frame();
        check_colors(COLORS_2)?;

        inject_touches("140 300 b\n140 300 e");
        let row: usize = view.long.value().trim_start_matches("Row ").parse()?;
        anyhow::ensure!(row > 12, "the list did not scroll, picked {}", view.long.value());
        anyhow::ensure!(view.height() > 0.0);

        inject_touches("420 60 b\n420 60 e");
        check_colors(COLORS_3)?;

        Ok(())
    }
}

const COLORS_1: &str = r"
      260    4 - #ebf0f7
      472    4 - #ebf0f7
      592    4 - #ebf0f7
       56   52 - #010101
      132   52 - #000000
      340   56 - #ffffff
      368   60 - #000000
       96   68 - #000000
      152   84 - #d0d4da
      180   84 - #d0d4da
      208   84 - #d0d4da
      232   96 - #99a4a6
       56  100 - #01daff
       68  100 - #01daff
      132  100 - #00daff
       56  104 - #01daff
       68  104 - #01daff
       92  104 - #00daff
      132  104 - #00daff
       56  108 - #00daff
       60  108 - #01daff
       64  108 - #00daff
      132  108 - #00daff
       56  112 - #01daff
      108  112 - #01daff
      132  112 - #00daff
      232  112 - #99a4a6
       56  116 - #01daff
       68  116 - #01daff
       96  116 - #00daff
      132  116 - #00daff
      232  124 - #99a4a6
      440  128 - #ebf0f7
       36  132 - #c9ced3
      128  136 - #ffffff
      536  140 - #ebf0f7
       84  144 - #ffffff
       56  148 - #010101
      232  156 - #a6a6a6
      344  160 - #ebf0f7
       36  168 - #c9ced3
       92  176 - #000000
      132  184 - #ffffff
      244  184 - #d0d4da
       56  188 - #010101
       92  212 - #000000
       56  216 - #000000
      232  216 - #a6a6a6
      136  220 - #000000
      420  232 - #ebf0f7
       56  244 - #010101
      244  244 - #d0d4da
       80  256 - #ffffff
      132  256 - #ffffff
       36  260 - #c9ced3
      240  264 - #b4b8bd
      592  280 - #ebf0f7
       56  284 - #010101
       84  288 - #ffffff
      240  288 - #b4b8bd
      132  292 - #ffffff
       36  304 - #c9ced3
      240  308 - #b4b8bd
       60  316 - #ffffff
       84  324 - #ffffff
      488  328 - #ebf0f7
       36  332 - #c9ced3
      128  332 - #000000
      240  332 - #b4b8bd
      364  344 - #ebf0f7
       56  352 - #010101
      240  352 - #b4b8bd
       80  360 - #ffffff
      132  364 - #ffffff
       96  368 - #000000
       36  376 - #c9ced3
      240  376 - #b4b8bd
       64  392 - #ffffff
      128  392 - #ffffff
      240  400 - #b4b8bd
       96  404 - #000000
       36  408 - #c9ced3
      240  424 - #b4b8bd
      592  424 - #ebf0f7
       56  432 - #000000
       80  432 - #ffffff
      148  432 - #ffffff
      452  436 - #ebf0f7
       36  444 - #c9ced3
      240  444 - #b4b8bd
       92  464 - #000000
      132  464 - #000000
      240  464 - #b4b8bd
       56  476 - #010101
      240  484 - #b4b8bd
       36  492 - #c9ced3
      352  492 - #ebf0f7
       92  500 - #000000
      132  504 - #000000
      240  508 - #b4b8bd
       56  512 - #010101
      148  528 - #000000
      240  528 - #b4b8bd
      500  536 - #ebf0f7
       64  540 - #000000
      132  544 - #000000
       36  548 - #c9ced3
       96  548 - #000000
      240  552 - #b4b8bd
       56  568 - #010101
      152  568 - #000000
       92  572 - #000000
      132  584 - #000000
       44  592 - #9ea2a6
       68  592 - #9ea2a6
       88  592 - #9ea2a6
      100  592 - #9ea2a6
      112  592 - #9ea2a6
      144  592 - #9ea2a6
      160  592 - #9ea2a6
      176  592 - #9ea2a6
      188  592 - #9ea2a6
      204  592 - #9ea2a6
      216  592 - #9ea2a6
      232  592 - #9ea2a6
      244  592 - #d0d4da
      408  592 - #ebf0f7
      592  592 - #ebf0f7
";

const COLORS_2: &str = r"
      268    4 - #ebf0f7
      452    4 - #ebf0f7
      592    4 - #ebf0f7
       56   52 - #010101
      132   56 - #000000
      344   56 - #ffffff
       96   68 - #000000
      368   68 - #000000
       68   84 - #d0d4da
      172   84 - #d0d4da
      196   84 - #d0d4da
      220   84 - #d0d4da
       36   88 - #c9ced4
      132   96 - #000000
      240  100 - #b4b8bd
       84  104 - #ffffff
       56  108 - #010101
      488  116 - #ebf0f7
       36  128 - #c9ced3
      240  128 - #b4b8bd
      132  132 - #000000
       80  140 - #ffffff
       56  144 - #010101
      240  156 - #b4b8bd
      592  156 - #ebf0f7
      348  164 - #ebf0f7
       56  168 - #010101
      152  168 - #000000
       92  172 - #000000
      244  180 - #d0d4da
      132  184 - #000000
       36  188 - #c9ced3
       60  204 - #ffffff
      240  204 - #b4b8bd
      144  208 - #000000
      508  216 - #ebf0f7
       96  220 - #000000
       36  224 - #c9ced3
      240  228 - #b4b8bd
       64  240 - #ffffff
       80  248 - #ffffff
      412  248 - #ebf0f7
      144  252 - #010101
      240  252 - #b4b8bd
       56  256 - #010101
       96  256 - #000000
       36  260 - #c9ced3
      592  268 - #ebf0f7
      232  272 - #8ea2a6
       68  280 - #010101
      132  280 - #000000
      232  280 - #8ea2a6
      232  288 - #8ea2a6
       36  292 - #c9ced3
       96  292 - #000000
      232  300 - #8ea2a6
      148  308 - #000000
       60  316 - #ffffff
       80  320 - #ffffff
       36  328 - #c9ced3
       96  328 - #000000
      132  328 - #000000
      240  328 - #b4b8bd
      496  328 - #ebf0f7
       56  348 - #010101
      148  352 - #ffffff
      232  356 - #a6a6a6
      356  356 - #ebf0f7
       68  364 - #010101
       96  364 - #000000
      132  364 - #000000
       36  372 - #c9ced3
      240  372 - #b4b8bd
       56  388 - #010101
      240  388 - #b4b8bd
      136  392 - #ffffff
       80  396 - #ffffff
       36  404 - #c9ced3
      240  416 - #b4b8bd
      592  416 - #ebf0f7
      128  420 - #ffffff
       56  428 - #000000
      448  432 - #ebf0f7
       96  436 - #000000
       36  444 - #c9ced3
      240  448 - #b4b8bd
       60  456 - #ffffff
      128  456 - #ffffff
      148  460 - #ffffff
       36  468 - #c9ced3
       80  468 - #ffffff
      240  476 - #b4b8bd
      136  492 - #010101
      352  492 - #ebf0f7
      584  504 - #ebf0f7
       56  508 - #010101
       96  508 - #000000
      240  508 - #b4b8bd
      128  528 - #ffffff
      240  528 - #b4b8bd
      500  532 - #ebf0f7
       64  536 - #000000
      152  540 - #000000
       96  544 - #000000
       36  548 - #c9ced3
      240  548 - #b4b8bd
       60  564 - #ffffff
      128  564 - #ffffff
       92  568 - #000000
      240  568 - #b4b8bd
       36  592 - #c9ced3
       52  592 - #9ea2a6
       68  592 - #9ea2a6
       84  592 - #9ea2a6
       96  592 - #9ea2a6
      112  592 - #9ea2a6
      128  592 - #9ea2a6
      144  592 - #9ea2a6
      156  592 - #9ea2a6
      168  592 - #9ea2a6
      180  592 - #9ea2a6
      200  592 - #9ea2a6
      212  592 - #9ea2a6
      220  592 - #9ea2a6
      232  592 - #9ea2a6
      240  592 - #b4b8bd
      408  592 - #ebf0f7
      592  592 - #ebf0f7
";

const COLORS_3: &str = r"
      244    4 - #ebf0f7
      592    4 - #ebf0f7
       56   52 - #010101
       68   52 - #010101
      132   52 - #000000
       56   56 - #010101
       60   56 - #ffffff
       64   56 - #ffffff
       92   56 - #000000
      132   56 - #000000
      340   56 - #ffffff
      344   56 - #ffffff
       56   60 - #000000
       64   60 - #000000
       80   60 - #ffffff
       84   60 - #ffffff
      132   60 - #000000
      340   60 - #ffffff
      344   60 - #ffffff
      348   60 - #ffffff
      368   60 - #000000
       56   64 - #010101
       80   64 - #ffffff
      132   64 - #000000
      148   64 - #000000
      336   64 - #010101
      368   64 - #000000
       56   68 - #010101
       68   68 - #010101
       96   68 - #000000
      132   68 - #000000
      352   68 - #ffffff
      360   68 - #ffffff
      364   68 - #ffffff
      368   68 - #000000
      320   84 - #d0d4da
      340   84 - #d0d4da
      356   84 - #d0d4da
      376   84 - #d0d4da
      396   84 - #d0d4da
      408   84 - #d0d4da
      420   84 - #d0d4da
      440   84 - #d0d4da
      464   84 - #d0d4da
      492   84 - #d0d4da
      508   84 - #d0d4da
      520   92 - #b4b8bd
      316   96 - #c9ced3
      340  104 - #ebfcff
      344  104 - #ebfcff
      384  104 - #01daff
      524  104 - #d0d4da
      316  108 - #c9ced3
      340  108 - #ebfcff
      344  108 - #ebfcff
      348  108 - #ebfcff
      368  108 - #00daff
      336  112 - #01daff
      368  112 - #00daff
      368  116 - #00daff
      520  116 - #b4b8bd
      316  120 - #c9ced3
      520  128 - #b4b8bd
      316  132 - #c9ced3
      524  136 - #d0d4da
      352  140 - #000000
      360  144 - #010101
      364  144 - #010101
      380  144 - #ffffff
      384  144 - #ffffff
      316  148 - #c9ced3
      380  148 - #ffffff
      384  148 - #ffffff
      520  148 - #b4b8bd
      356  152 - #000000
      316  164 - #c9ced3
      520  164 - #b4b8bd
      356  176 - #000000
      364  176 - #010101
      372  176 - #010101
      364  180 - #000000
      372  180 - #000000
      520  180 - #b4b8bd
      316  184 - #c9ced3
      364  184 - #000000
      372  184 - #010101
      388  184 - #ffffff
      364  188 - #000000
      372  188 - #010101
      384  188 - #010101
      524  192 - #d0d4da
        4  204 - #ebf0f7
      324  204 - #adb0b5
      332  204 - #adb0b5
      340  204 - #adb0b5
      348  204 - #adb0b5
      356  204 - #adb0b5
      368  204 - #adb0b5
      384  204 - #adb0b5
      404  204 - #adb0b5
      416  204 - #adb0b5
      436  204 - #adb0b5
      448  204 - #adb0b5
      460  204 - #adb0b5
      472  204 - #adb0b5
      484  204 - #adb0b5
      500  204 - #adb0b5
      516  204 - #adb0b5
      428  208 - #c9ced3
      172  216 - #ebf0f7
      476  308 - #ebf0f7
      592  316 - #ebf0f7
       76  332 - #ebf0f7
      376  340 - #ebf0f7
      244  352 - #ebf0f7
      152  392 - #ebf0f7
      488  412 - #ebf0f7
        4  452 - #ebf0f7
      592  472 - #ebf0f7
      404  480 - #ebf0f7
      136  488 - #ebf0f7
      300  508 - #ebf0f7
      500  516 - #ebf0f7
      104  584 - #ebf0f7
        4  592 - #ebf0f7
      208  592 - #ebf0f7
      432  592 - #ebf0f7
      592  592 - #ebf0f7
";
