use std::mem::take;

use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{Label, LogData, LogLine, LogView, Setup, ViewData, ViewFrame, ViewTest, WHITE, view},
    ui_test::{check_colors, inject_scroll, inject_touches, set_record_probe_count},
};
use parking_lot::Mutex;

use crate::log_probe::{line, lines_on_screen, status_style, test_style};

/// The lines the log asked its data for.
static ASKED: Mutex<Vec<usize>> = Mutex::new(Vec::new());

const LINES: usize = 20_000;
const LOG_HEIGHT: f32 = 380.0;
const WIDE: f32 = 580.0;
const NARROW: f32 = 400.0;
/// A new width or a scroll asks only for the rows on screen, each a few
/// times: for its exact height and for its cell. Far under the 20 000
/// lines of the log.
const MOST_ASKED: usize = 400;

fn text_of(index: usize) -> String {
    if index.is_multiple_of(5) {
        format!("line {index} is a longer line, it wraps into more rows when the log gets narrower than this")
    } else {
        format!("line {index} is short")
    }
}

/// A log of 20 000 lines gets a new width. The view asks its data only
/// for the rows on screen, not for all lines: a row out of view gets its
/// height from the count of its chars. The first row on screen stays at
/// its place over the new width. The status label shows how many lines
/// were asked.
#[view]
struct LogViewResize {
    #[init]
    log:    LogView,
    status: Label,
}

impl Setup for LogViewResize {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);

        self.log.set_style(test_style());
        self.log.set_data_source(self);
        self.log.set_frame((10.0, 10.0, WIDE, LOG_HEIGHT));

        status_style(self.status);
        self.status.set_text("20000 lines");
        self.status.place().t(LOG_HEIGHT + 20.0).lr(10).h(180);
    }
}

impl LogData for LogViewResize {
    fn number_of_lines(&self) -> usize {
        LINES
    }

    fn line(&self, index: usize) -> LogLine {
        ASKED.lock().push(index);
        LogLine::new(text_of(index)).with_prefix(format!("{index:05}"))
    }
}

impl LogViewResize {
    fn resize(self: Weak<Self>, width: f32) {
        from_main(move || {
            self.log.set_frame((10.0, 10.0, width, LOG_HEIGHT));
        });
        wait_for_next_frame();
        wait_for_next_frame();
        wait_for_next_frame();
    }

    /// How many lines were asked since the last call, written to the
    /// status label with `step`.
    fn asked(self: Weak<Self>, step: &'static str) -> usize {
        let asked = take(&mut *ASKED.lock()).len();
        from_main(move || {
            self.status
                .set_text(format!("{step}\nthe data was asked for {asked} lines of {LINES}"));
        });
        wait_for_next_frame();
        asked
    }

    /// The first line that starts inside the log, as `line N is`, and
    /// where it is.
    fn first_line(self: Weak<Self>) -> (String, f32) {
        from_main(move || {
            let top = self.log.absolute_frame().y();
            lines_on_screen(self.log, " is ")
                .into_iter()
                .map(|label| (label.text().to_string(), label.absolute_frame().y()))
                .filter(|(_, y)| *y >= top)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map_or_else(Default::default, |(text, y)| {
                    let name = text.split(" is").next().unwrap_or_default();
                    (format!("{name} is"), y)
                })
        })
    }
}

impl ViewTest for LogViewResize {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);
        wait_for_next_frame();
        wait_for_next_frame();
        ensure!(
            from_main(move || line(view.log, "line 19999 is").is_ok()),
            "the log does not show its end"
        );
        view.asked("20000 lines, the log shows its end");
        check_colors(AT_END)?;

        // A new width at the end of the log.
        view.resize(NARROW);
        let asked = view.asked("the log is narrower, still at its end");
        ensure!(asked <= MOST_ASKED, "a new width asked for {asked} lines");
        ensure!(
            from_main(move || line(view.log, "line 19999 is").is_ok() && view.log.is_following()),
            "the log left its end over a new width"
        );
        check_colors(NARROW_AT_END)?;

        // Up into a part of the log that was never on screen.
        inject_touches("200 200 b\n200 200 e");
        inject_scroll(60_000);
        wait_for_next_frame();
        wait_for_next_frame();
        let asked = view.asked("scrolled far up, into rows that were never on screen");
        ensure!(asked <= MOST_ASKED, "a scroll asked for {asked} lines");
        let (first, before) = view.first_line();
        ensure!(
            !first.is_empty(),
            "no line starts inside the log after the scroll"
        );
        check_colors(SCROLLED)?;

        // A new width in the middle: the first line stays where it is.
        view.resize(WIDE);
        let asked = view.asked("the log is wide again\nthe first line on screen stayed at its place");
        ensure!(
            asked <= MOST_ASKED,
            "a new width in the middle asked for {asked} lines"
        );
        let (first_now, after) = view.first_line();
        ensure!(
            first_now == first && (after - before).abs() < 0.5,
            "the first line was `{first}` at {before}, over the new width it is `{first_now}` at {after}"
        );
        check_colors(WIDE_IN_MIDDLE)?;

        Ok(())
    }
}

/// Recorded with `--record-colors`.
const AT_END: &str = r"
              44   16 - #f6f8fa
             284   16 - #73777b
             108   20 - #31363c
             340   20 - #dcdfe1
             508   20 - #24292f
             232   32 - #4f5358
             412   36 - #5b5f64
             384   40 - #9ca0a3
             460   40 - #5b5f64
             176   44 - #f6f8fa
              72   76 - #cfd3d7
             184   76 - #b9bcbf
             120   84 - #34393f
             244   84 - #e1e4e6
             240  124 - #4d5257
             108  128 - #8d9195
              56  144 - #707983
             204  144 - #31363c
             348  144 - #a3a6aa
             540  144 - #a3a6aa
              96  164 - #9ca0a3
             384  164 - #9ca0a3
             292  168 - #f6f8fa
             412  168 - #5b5f64
             460  168 - #5b5f64
             120  188 - #34393f
             168  208 - #262b31
              72  212 - #7c848d
             240  224 - #4d5257
              44  248 - #f6f8fa
             184  264 - #d2d4d7
             388  268 - #41454b
             128  272 - #a5a8ac
             328  272 - #d9dcde
             540  272 - #a5a8ac
             232  288 - #d9dcde
              96  292 - #9ca0a3
             272  292 - #f6f8fa
             412  292 - #5b5f64
             460  292 - #5b5f64
              44  332 - #f6f8fa
             156  332 - #f6f8fa
             260  356 - #24292f
             100  360 - #787c80
             204  360 - #787c80
             584  372 - #a0a1a3
              56  376 - #707982
             584  380 - #a0a1a3
             584  384 - #a0a1a3
              76  408 - #909090
              28  412 - #010101
             144  420 - #070707
              28  424 - #525252
              32  424 - #000000
             144  424 - #000000
             256  424 - #000000
             300  424 - #010101
             176  428 - #5d5d5d
             448  444 - #ffffff
             592  524 - #ffffff
             376  532 - #ffffff
              96  592 - #ffffff
             280  592 - #ffffff
             472  592 - #ffffff
";

/// Recorded with `--record-colors`.
const NARROW_AT_END: &str = r"
             592    4 - #ffffff
             252   16 - #f6f8fa
             120   20 - #34393f
              72   32 - #a8aeb5
             184   32 - #7e8186
              56   60 - #707982
             240   60 - #4d5257
             204   64 - #393e44
             120   84 - #34393f
             252  100 - #31363c
             320  100 - #62666b
             340  104 - #dcdfe1
             280  124 - #9ea1a5
             100  128 - #cdcfd2
             184  128 - #cccfd1
             100  140 - #d6d8db
             136  144 - #d9dcde
             184  144 - #63686c
             240  144 - #9ca0a3
             308  144 - #93979a
             356  144 - #93979a
             244  168 - #e1e4e6
             156  204 - #f6f8fa
              56  208 - #6f7881
             240  208 - #4d5257
             108  212 - #63686c
             204  212 - #63686c
             592  228 - #ffffff
              72  244 - #6e7781
             184  244 - #24292f
             320  244 - #62666b
             268  252 - #464a50
             340  252 - #dcdfe1
             136  272 - #9ea1a5
             280  272 - #9ea1a5
             100  288 - #c1c4c7
             196  288 - #c1c4c7
             376  288 - #c1c4c7
             236  292 - #888c90
             308  292 - #93979a
             312  292 - #f6f8fa
             356  292 - #93979a
             244  316 - #e1e4e6
              44  332 - #f6f8fa
             108  336 - #31363c
             252  336 - #f6f8fa
             176  352 - #f6f8fa
             128  356 - #666a6f
             404  372 - #a0a1a3
              56  376 - #707983
             260  376 - #252a30
             404  384 - #a0a1a3
             228  404 - #4e4e4e
             160  408 - #101010
             192  408 - #b7b7b7
             584  408 - #ffffff
             144  420 - #070707
              28  424 - #525252
              32  424 - #000000
             144  424 - #000000
             176  424 - #000000
             248  424 - #0b0b0b
             376  592 - #ffffff
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const SCROLLED: &str = r"
             592    4 - #ffffff
              44   20 - #b5bbc1
              72   20 - #b5bbc1
             108   28 - #31363c
             128   28 - #e4e6e8
             240   28 - #4d5257
             176   44 - #f6f8fa
              44   84 - #6e7781
             268   88 - #464a50
             236   92 - #888c90
             340   92 - #dcdfe1
             108  112 - #a6a9ad
             212  112 - #f6f8fa
             156  132 - #f6f8fa
             280  132 - #8d9195
             376  132 - #d9dcde
              44  148 - #b3b9bf
             244  156 - #e1e4e6
             176  176 - #f6f8fa
              56  196 - #6f7882
             260  196 - #252a30
             108  200 - #4e5258
             204  216 - #31363c
             244  220 - #e1e4e6
             592  224 - #ffffff
              44  232 - #707982
             336  240 - #4d5257
             136  260 - #9ea1a5
             236  260 - #24292f
             280  260 - #9ea1a5
             100  264 - #cdcfd2
             184  264 - #cdcfd2
             376  276 - #d6d8db
             312  280 - #f6f8fa
             156  296 - #b9bcbf
             260  300 - #252a30
              44  316 - #7f8790
             404  336 - #a0a1a3
             120  344 - #34393f
             176  344 - #f6f8fa
             244  344 - #e1e4e6
             380  344 - #dadcdd
             392  344 - #e8eaec
             404  348 - #a0a1a3
             364  360 - #c8cacc
             400  360 - #cfd1d3
             400  368 - #ced0d1
             364  372 - #cfd1d3
             400  376 - #e2e3e5
              44  380 - #8b929b
             384  380 - #b6b8b9
             268  384 - #464a50
             320  384 - #62666b
             372  384 - #e1e3e5
             188  404 - #131313
             108  408 - #000000
             584  408 - #ffffff
             144  420 - #070707
              28  424 - #525252
             144  424 - #000000
             248  424 - #0b0b0b
             272  424 - #717171
             388  592 - #ffffff
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const WIDE_IN_MIDDLE: &str = r"
              72   20 - #b5bbc1
             184   20 - #92969a
             128   28 - #e4e6e8
             240   28 - #4d5257
              44   64 - #f6f8fa
             244   72 - #e1e4e6
             292   92 - #f6f8fa
             336   92 - #4d5257
             508   92 - #24292f
              96  112 - #9ca0a3
             172  112 - #464a50
             376  112 - #a5a8ac
             428  112 - #73777b
             232  116 - #cdcfd2
             472  116 - #cdcfd2
             204  152 - #31363c
              44  168 - #7f8790
             156  168 - #3e4348
             244  176 - #e1e4e6
             100  200 - #787c80
             204  200 - #787c80
             284  216 - #73777b
             376  216 - #d9dcde
             508  216 - #24292f
             232  236 - #d9dcde
             336  240 - #9ca0a3
             412  240 - #5b5f64
             456  240 - #f6f8fa
              44  252 - #b5bbc1
             156  252 - #92969a
             128  260 - #e4e6e8
             204  280 - #31363c
             100  296 - #f6f8fa
             252  300 - #f6f8fa
              44  316 - #6e7781
             264  320 - #e6e9eb
             176  336 - #e6e9eb
             584  336 - #a0a1a3
             484  340 - #2c3137
             560  344 - #dadcdd
             108  348 - #b7babd
             584  348 - #a0a1a3
             544  360 - #c8cacc
             580  360 - #cfd1d3
             316  364 - #464a50
             408  364 - #f6f8fa
             580  368 - #ced0d1
             544  372 - #cfd1d3
              44  380 - #c1c6cb
             560  380 - #b5b7b8
             568  380 - #bfc0c2
             244  388 - #e1e4e6
             124  408 - #000000
             132  412 - #000000
              28  424 - #525252
             232  424 - #000000
              32  440 - #000000
             144  440 - #000000
             192  440 - #909090
             484  500 - #ffffff
             220  580 - #ffffff
              64  592 - #ffffff
             380  592 - #ffffff
             592  592 - #ffffff
";
