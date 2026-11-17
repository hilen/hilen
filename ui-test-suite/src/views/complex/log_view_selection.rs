use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        Label, LogData, LogLine, LogView, Setup, TextSelection, UIManager, ViewData, ViewFrame, ViewTest,
        WHITE, view,
    },
    ui_test::{check_colors, inject_touches, set_record_probe_count},
};

use crate::{
    log_probe::{line, status_style, test_style},
    text_points::{click, drag, point_after, point_of, shown},
};

const LONG: &str = "this line is long and wraps into a second row of text, the selection goes over both of \
                    its rows";

fn lines() -> Vec<LogLine> {
    vec![
        LogLine::new("the server starts").with_prefix("10:00:01"),
        LogLine::new("listening on port 8080").with_prefix("10:00:02"),
        LogLine::new(LONG).with_prefix("10:00:03"),
        LogLine::new("a line with no prefix"),
        LogLine::new("\x1b[31mthe request failed\x1b[0m with a code").with_prefix("10:00:05"),
        LogLine::new("the server stops").with_prefix("10:00:06"),
    ]
}

/// A drag over the rows of a log selects their text. A copy holds what
/// is selected: of a row taken whole the prefix, a space and the text,
/// of a row with no prefix only the text, with 1 line break between 2
/// rows and no color code. The status label shows what a copy would
/// hold.
#[view]
struct LogViewSelection {
    lines: Vec<LogLine>,

    #[init]
    log:    LogView,
    status: Label,
}

impl Setup for LogViewSelection {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        self.lines = lines();

        self.log.set_style(test_style());
        self.log.set_data_source(self);
        self.log.set_frame((10.0, 10.0, 580.0, 300.0));

        status_style(self.status);
        self.status.set_text("nothing is selected");
        self.status.place().t(320).lr(10).h(270);
    }
}

impl LogData for LogViewSelection {
    fn number_of_lines(&self) -> usize {
        self.lines.len()
    }

    fn line(&self, index: usize) -> LogLine {
        self.lines[index].clone()
    }
}

impl LogViewSelection {
    /// The selected text, also written to the status label.
    fn show(self: Weak<Self>, step: &'static str) -> String {
        let text = from_main(move || {
            let text = TextSelection::text();
            self.status.set_text(format!("{step}\na copy holds:\n{}", shown(&text)));
            text
        });
        wait_for_next_frame();
        text
    }
}

impl ViewTest for LogViewSelection {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(96);
        from_main(|| UIManager::set_drag_scrolling(false));
        wait_for_next_frame();
        wait_for_next_frame();

        // From the middle of the text of row 1 over the long row into
        // the text of the row with no prefix.
        let (from, to) = from_main(move || {
            (
                point_of(line(view.log, "listening"), "port"),
                point_after(line(view.log, "no prefix"), "a line"),
            )
        });
        inject_touches(drag(from, to));
        let over_3_rows = format!("port 8080\n10:00:03 {LONG}\na line");
        let text = view.show("a drag from `port` of row 1 to the end of `a line` of row 3");
        ensure!(text == over_3_rows, "the drag over 3 rows selected {text:?}");
        check_colors(OVER_ROWS)?;

        // A drag that starts in a prefix takes the prefix too. The color
        // codes of the text are not in the copy.
        let (from, to) = from_main(move || {
            (
                point_of(line(view.log, "10:00:05"), "00:05"),
                point_after(line(view.log, "the server stops"), "the server"),
            )
        });
        inject_touches(drag(from, to));
        let from_prefix = "00:05 the request failed with a code\n10:00:06 the server";
        let text = view.show("a drag from inside the prefix of row 4 into row 5");
        ensure!(text == from_prefix, "the drag from a prefix selected {text:?}");
        check_colors(FROM_PREFIX)?;

        // Everything: 1 line of the copy per line of the log.
        from_main(TextSelection::select_all);
        let all = [
            "10:00:01 the server starts".to_string(),
            "10:00:02 listening on port 8080".to_string(),
            format!("10:00:03 {LONG}"),
            "a line with no prefix".to_string(),
            "10:00:05 the request failed with a code".to_string(),
            "10:00:06 the server stops".to_string(),
        ]
        .join("\n");
        let text = view.show("select all");
        ensure!(text == all, "select all took {text:?}");
        check_colors(ALL)?;

        // A click beside the text drops the selection.
        let beside = from_main(move || point_after(line(view.log, "the server stops"), "stops"));
        inject_touches(click((beside.0 + 60.0, beside.1)));
        let text = view.show("a click beside the text");
        ensure!(text.is_empty(), "a click left the selection {text:?}");
        check_colors(NONE)?;

        Ok(())
    }
}

/// Recorded with `--record-colors`.
const OVER_ROWS: &str = r"
              60   24 - #6f7881
             260   24 - #24292f
              92   28 - #f6f8fa
             140   28 - #5a5f64
             184   28 - #9ea1a5
             212   28 - #24292f
             332   40 - #b2cefc
             128   48 - #62666b
             184   48 - #d9dcde
             196   48 - #dcdfe1
             236   48 - #5f6368
             300   48 - #b2cefc
             384   56 - #cedffb
             436   56 - #cedffb
              64   60 - #b2cefc
             552   64 - #b2cefc
              32   68 - #b2cefc
             184   68 - #9fb7e0
             224   68 - #4e5a6b
             268   68 - #3b4450
             320   68 - #b2cefc
             472   68 - #24292f
             528   68 - #8398b8
              96   72 - #b2cefc
             364   72 - #3a424e
             156   88 - #7c8fae
             300   88 - #7c8fae
             396   88 - #b2cefc
             500   88 - #98b0d6
             220   92 - #3b4450
             332   92 - #4c5869
             424   92 - #687891
             476   92 - #3a434f
             500   92 - #98b0d6
             252   96 - #b2cefc
             352   96 - #b2cefc
             124  124 - #b2cefc
             168  124 - #b2cefc
             220  128 - #464a50
             240  132 - #4d5257
             148  136 - #889dbf
             204  136 - #b7babd
             308  136 - #b7babd
              92  148 - #f6f8fa
             272  148 - #da616a
              60  152 - #6f7881
             164  152 - #cf232f
             188  152 - #de747c
             192  152 - #d5424c
             204  152 - #df7880
             272  152 - #da616a
             280  152 - #df7880
             292  152 - #edc8cc
             328  152 - #24292f
             360  152 - #252a30
             396  152 - #f6f8fa
             188  156 - #de747c
              40  172 - #d2d6da
              60  172 - #6e7781
              88  172 - #d2d6da
             244  172 - #f6f8fa
             140  176 - #5a5f64
             212  176 - #24292f
             592  220 - #ffffff
             436  268 - #f6f8fa
             216  324 - #c5c5c5
             292  324 - #000000
              40  328 - #ffffff
              96  328 - #010101
             128  328 - #515151
             148  328 - #c9c9c9
             156  328 - #909090
             196  328 - #d2d2d2
             268  328 - #afafaf
             324  328 - #ffffff
              72  340 - #797979
              88  344 - #a9a9a9
             120  360 - #000000
             144  360 - #010101
             188  360 - #444444
             236  360 - #000000
             248  360 - #000000
             292  360 - #000000
             520  360 - #777777
             544  360 - #909090
              44  364 - #f8f8f8
             212  376 - #c4c4c4
              84  380 - #c5c5c5
             116  380 - #1c1c1c
             212  380 - #c4c4c4
             220  380 - #464646
             404  448 - #ffffff
             592  560 - #ffffff
             196  568 - #ffffff
               4  592 - #ffffff
             384  592 - #ffffff
";

/// Recorded with `--record-colors`.
const FROM_PREFIX: &str = r"
              60   24 - #6f7881
             212   24 - #24292f
             260   24 - #24292f
              92   28 - #f6f8fa
             140   28 - #5a5f64
             184   28 - #9ea1a5
             280   44 - #24292f
             328   44 - #f6f8fa
             136   48 - #d9dcde
             184   48 - #d9dcde
             196   48 - #dcdfe1
             236   48 - #5f6368
              44   64 - #f6f8fa
              92   64 - #f6f8fa
             184   68 - #d9dcde
             192   68 - #4d5257
             224   68 - #62666b
             280   68 - #f6f8fa
             364   68 - #44494e
             472   68 - #24292f
             500   68 - #e1e3e5
             528   68 - #b1b4b7
             252   72 - #24292f
             396   72 - #252a30
             156   88 - #a6a9ad
             184   88 - #24292f
             272   88 - #62666b
             300   88 - #a6a9ad
             136   92 - #24292f
             220   92 - #464a50
             332   92 - #5f6368
             424   92 - #888c90
             476   92 - #45494f
             500   92 - #cfd2d5
             220  128 - #464a50
             176  132 - #909397
             240  132 - #4d5257
             268  132 - #44494e
             148  136 - #b7babd
             156  136 - #b7babd
             204  136 - #b7babd
             308  136 - #b7babd
             124  144 - #b2cefc
              92  148 - #b2cefc
             272  148 - #c6556b
             336  148 - #404957
              60  152 - #6e7781
             164  152 - #cf232f
             188  152 - #c4647d
             192  152 - #cb3c4d
             204  152 - #c36780
             272  152 - #c6556b
             280  152 - #c36780
             292  152 - #b8a8ce
             320  152 - #4e5a6b
             352  152 - #b2cefc
             376  152 - #b2cefc
             396  152 - #b2cefc
             188  156 - #c4647d
             244  156 - #b2cefc
             416  156 - #b2cefc
             140  160 - #b2cefc
             112  164 - #b2cefc
              32  172 - #b2cefc
              52  172 - #b2cefc
              88  172 - #a0b7db
             140  176 - #495464
             184  176 - #7689a6
             216  176 - #b2cefc
              72  180 - #b2cefc
             128  180 - #b2cefc
             156  180 - #b2cefc
             456  296 - #f6f8fa
             148  324 - #000000
             252  324 - #898989
             316  324 - #c8c8c8
              40  328 - #ffffff
              96  328 - #010101
             220  328 - #c3c3c3
             292  328 - #ffffff
             116  332 - #0f0f0f
              72  340 - #797979
             592  340 - #ffffff
              96  344 - #b4b4b4
              56  356 - #000000
              32  360 - #000000
             152  360 - #434343
             196  360 - #000000
             264  360 - #9e9e9e
             296  360 - #ffffff
             324  360 - #000000
             464  468 - #ffffff
             204  508 - #ffffff
              84  592 - #ffffff
             324  592 - #ffffff
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const ALL: &str = r"
             104   16 - #b2cefc
              60   24 - #6e7781
             212   24 - #24292f
             260   24 - #24292f
             184   28 - #7689a6
             332   40 - #b2cefc
              44   48 - #b2cefc
             136   48 - #9fb7e0
             196   48 - #a1bae3
             224   48 - #b2cefc
             280   48 - #24292f
             436   56 - #cedffb
              92   64 - #b2cefc
             184   64 - #7284a0
             252   64 - #2d333c
             284   68 - #59677c
             364   68 - #3a424e
             396   68 - #2d333c
             472   68 - #24292f
             528   68 - #8398b8
              32   76 - #b2cefc
             272   84 - #4e5a6b
             156   88 - #7c8fae
             220   88 - #3b4450
             300   88 - #7c8fae
             332   92 - #4c5869
             424   92 - #687891
             500   92 - #98b0d6
             360   96 - #b2cefc
             124  108 - #b2cefc
             176  132 - #6d7e98
             244  132 - #a1bae3
             148  136 - #889dbf
             156  136 - #889dbf
             204  136 - #889dbf
             308  136 - #889dbf
             104  148 - #b2cefc
             272  148 - #c6556b
              60  152 - #6e7781
             164  152 - #cf232f
             188  152 - #c4647d
             204  152 - #c36780
             272  152 - #c6556b
             280  152 - #c36780
             292  152 - #b8a8ce
             360  152 - #252a30
              32  156 - #b2cefc
             188  156 - #c4647d
             412  160 - #b2cefc
              40  172 - #a0b7db
              88  172 - #a0b7db
             252  172 - #b2cefc
             140  176 - #495464
             184  176 - #7689a6
             592  212 - #ffffff
              80  324 - #c3c3c3
              88  340 - #a9a9a9
              48  344 - #ffffff
              88  344 - #a9a9a9
              96  344 - #b4b4b4
             168  360 - #dddddd
             212  360 - #010101
             256  360 - #dddddd
             348  360 - #000000
             408  360 - #d1d1d1
             476  360 - #010101
             496  360 - #c3c3c3
             540  360 - #dddddd
             580  360 - #000000
             544  364 - #ffffff
             288  376 - #aaaaaa
             540  376 - #c3c3c3
             140  380 - #cfcfcf
             288  380 - #aaaaaa
             316  380 - #000000
             368  380 - #acacac
             444  380 - #ffffff
             532  380 - #dddddd
             124  388 - #a5a5a5
             212  388 - #ffffff
             244  388 - #ffffff
              96  392 - #ffffff
             340  392 - #ffffff
              36  396 - #dfdfdf
              68  396 - #5f5f5f
             156  396 - #131313
             180  396 - #000000
             256  396 - #dcdcdc
             268  396 - #858585
             392  396 - #929292
             424  396 - #5d5d5d
             592  516 - #ffffff
             320  536 - #ffffff
               4  568 - #ffffff
             176  592 - #ffffff
             468  592 - #ffffff
";

/// Recorded with `--record-colors`.
const NONE: &str = r"
              60   24 - #6f7881
             212   24 - #24292f
             252   24 - #f6f8fa
              92   28 - #f6f8fa
             140   28 - #5a5f64
             184   28 - #9ea1a5
             280   44 - #24292f
             308   44 - #f6f8fa
             136   48 - #d9dcde
             172   48 - #464a50
             184   48 - #d9dcde
             192   48 - #4d5257
             236   48 - #5f6368
              44   64 - #f6f8fa
              72   64 - #f6f8fa
             100   68 - #f6f8fa
             176   68 - #62666b
             184   68 - #d9dcde
             192   68 - #4d5257
             224   68 - #62666b
             268   68 - #464a50
             284   68 - #73777b
             320   68 - #f6f8fa
             364   68 - #44494e
             472   68 - #24292f
             500   68 - #e1e3e5
             528   68 - #b1b4b7
             148   72 - #24292f
             252   72 - #24292f
             396   72 - #252a30
             220   84 - #464a50
             156   88 - #a6a9ad
             184   88 - #24292f
             272   88 - #62666b
             300   88 - #a6a9ad
             136   92 - #24292f
             220   92 - #464a50
             332   92 - #5f6368
             424   92 - #888c90
             476   92 - #45494f
             500   92 - #cfd2d5
             156  112 - #f6f8fa
             220  128 - #464a50
             176  132 - #909397
             220  132 - #464a50
             240  132 - #4d5257
             244  132 - #dcdfe1
             268  132 - #44494e
             280  132 - #9ea1a5
             292  132 - #909397
             148  136 - #b7babd
             156  136 - #b7babd
             204  136 - #b7babd
             268  136 - #44494e
             308  136 - #b7babd
              92  148 - #f6f8fa
             272  148 - #da616a
             336  148 - #4d5257
              60  152 - #6f7881
             164  152 - #cf232f
             188  152 - #de747c
             192  152 - #d5424c
             204  152 - #df7880
             272  152 - #da616a
             280  152 - #df7880
             292  152 - #edc8cc
             320  152 - #62666b
             360  152 - #252a30
             396  152 - #f6f8fa
             188  156 - #de747c
              40  172 - #d2d6da
              60  172 - #6e7781
              76  172 - #7c848d
              88  172 - #d2d6da
             252  172 - #f6f8fa
             140  176 - #5a5f64
             184  176 - #9ea1a5
             212  176 - #24292f
             592  192 - #ffffff
             460  276 - #f6f8fa
              60  324 - #2a2a2a
             100  324 - #a0a0a0
             120  324 - #5f5f5f
              48  328 - #dddddd
             124  328 - #010101
             592  328 - #ffffff
              72  340 - #797979
              96  340 - #b4b4b4
              88  344 - #a9a9a9
             368  380 - #ffffff
             560  460 - #ffffff
             144  500 - #ffffff
             440  528 - #ffffff
               4  592 - #ffffff
             284  592 - #ffffff
             592  592 - #ffffff
";
