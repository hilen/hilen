use std::mem::take;

use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        Label, LogData, LogLine, LogView, Setup, TextSelection, UIManager, ViewData, ViewFrame, ViewTest,
        WHITE, view,
    },
    ui_test::{check_colors, inject_scroll, inject_touches, set_record_probe_count},
};
use parking_lot::Mutex;

use crate::{
    log_probe::{line, line_on_screen, status_style, test_style},
    text_points::{drag, point_after, point_of},
};

/// The names of the lines the log asked its data for.
static ASKED: Mutex<Vec<usize>> = Mutex::new(Vec::new());

const LINES: usize = 60;
const TRIMMED: usize = 20;
/// Rows that come into the 2 rows of slack past the screen are set up.
const MOST_NEW_ROWS: usize = 4;

/// The app drops the oldest lines of its log and tells the view with
/// `lines_removed`. Every line has a name it keeps. The lines on screen
/// stay where they are, a selection stays on its text, and no line is
/// measured again.
#[view]
struct LogViewTrim {
    /// The names of the lines the data holds.
    names: Vec<usize>,

    #[init]
    log:    LogView,
    status: Label,
}

impl Setup for LogViewTrim {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        self.names = (0..LINES).collect();

        self.log.set_style(test_style());
        self.log.set_data_source(self);
        self.log.set_frame((10.0, 10.0, 580.0, 300.0));

        status_style(self.status);
        self.status.set_text("60 lines, named 0 to 59");
        self.status.place().t(320).lr(10).h(270);
    }
}

impl LogData for LogViewTrim {
    fn number_of_lines(&self) -> usize {
        self.names.len()
    }

    fn line(&self, index: usize) -> LogLine {
        let name = self.names[index];
        ASKED.lock().push(name);
        let text = if name.is_multiple_of(4) {
            format!("line {name} of the log is long enough to wrap into a second row in a log this wide")
        } else {
            format!("line {name} of the log")
        };
        LogLine::new(text).with_prefix(format!("{name:03}"))
    }
}

impl LogViewTrim {
    fn top_of(self: Weak<Self>, part: &'static str) -> Option<f32> {
        from_main(move || line_on_screen(self.log, part).map(|label| label.absolute_frame().y()))
    }

    fn trim(mut self: Weak<Self>, count: usize) {
        from_main(move || {
            self.names.drain(..count);
            self.log.lines_removed(count);
        });
        wait_for_next_frame();
        wait_for_next_frame();
    }

    fn set_status(self: Weak<Self>, status: &'static str) {
        from_main(move || {
            self.status.set_text(status);
        });
        wait_for_next_frame();
    }
}

impl ViewTest for LogViewTrim {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(96);
        from_main(|| UIManager::set_drag_scrolling(false));
        wait_for_next_frame();
        wait_for_next_frame();

        // Up to the middle, then a selection over 3 lines. The wheel goes
        // to the view under the pointer, a tap puts it over the log.
        inject_touches("300 150 b\n300 150 e");
        inject_scroll(520);
        wait_for_next_frame();
        wait_for_next_frame();
        let (from, to) = from_main(move || {
            (
                point_of(line(view.log, "line 37 of"), "37 of"),
                point_after(line(view.log, "line 39 of"), "line 39"),
            )
        });
        inject_touches(drag(from, to));
        wait_for_next_frame();
        let selected = "37 of the log\n038 line 38 of the log\n039 line 39";
        let text = from_main(TextSelection::text);
        ensure!(text == selected, "the drag selected {text:?}");
        let before = view.top_of("line 37 of");
        ensure!(before.is_some(), "line 37 is not on screen");
        view.set_status("in the middle of the log\nlines 37 to 39 are selected");
        check_colors(SELECTED)?;
        take(&mut *ASKED.lock());

        // The oldest 20 lines go. The lines on screen stay.
        view.trim(TRIMMED);
        let asked = take(&mut *ASKED.lock());
        ensure!(
            asked.len() <= MOST_NEW_ROWS,
            "20 lines left the data and the log asked for {asked:?}"
        );
        let after = view.top_of("line 37 of");
        ensure!(
            after == before,
            "line 37 moved from {before:?} to {after:?} when old lines left"
        );
        let text = from_main(TextSelection::text);
        ensure!(
            text == selected,
            "the selection changed to {text:?} when old lines left"
        );
        // Only the scroll bar is new: the content is shorter now.
        view.set_status(
            "lines 0 to 19 left the data\nno line on screen moved\nthe selection is on the same text",
        );
        check_colors(TRIMMED_ABOVE)?;

        // Lines of the selection go. It keeps what is left of it.
        view.trim(18);
        let text = from_main(TextSelection::text);
        ensure!(
            text == "038 line 38 of the log\n039 line 39",
            "the selection is {text:?} after its first line left"
        );
        view.set_status(
            "lines 20 to 37 left the data\nline 38 is the first line now\nthe selection starts there",
        );
        check_colors(TRIMMED_INTO)?;

        // At the end the log follows, also when old lines leave.
        from_main(TextSelection::clear);
        inject_scroll(-100_000);
        wait_for_next_frame();
        wait_for_next_frame();
        view.trim(10);
        ensure!(
            from_main(move || view.log.is_following() && line_on_screen(view.log, "line 59 of").is_some()),
            "the log left its end when old lines left"
        );
        view.set_status("at the end, lines 38 to 47 left the data\nthe log still shows line 59 at its end");
        check_colors(AT_END)?;

        Ok(())
    }
}

/// Recorded with `--record-colors`.
const SELECTED: &str = r"
             592    4 - #ffffff
             232   12 - #d9dcde
             120   16 - #34393f
             144   16 - #9ca0a3
             184   16 - #d9dcde
             272   16 - #f6f8fa
             328   16 - #d9dcde
              88   32 - #82868a
             164   36 - #bbbec1
             204   36 - #252a30
              44   40 - #7c848d
              80   56 - #62666b
             140   56 - #393e44
             192   60 - #4d5257
             224   76 - #62666b
              96   80 - #4d5257
             164   80 - #bbbec1
              40   96 - #f6f8fa
             204  100 - #a5a8ac
             268  100 - #f6f8fa
             380  100 - #5e6267
             484  104 - #cdcfd2
              88  120 - #63686c
             144  120 - #9ca0a3
             328  120 - #d9dcde
             192  136 - #4d5257
             224  136 - #62666b
             164  144 - #bbbec1
              56  160 - #bdc2c8
             584  160 - #a0a1a3
              52  164 - #818992
             584  172 - #a0a1a3
             184  180 - #24292f
             136  184 - #f6f8fa
             584  184 - #a0a1a3
              80  188 - #787c80
              88  188 - #787c80
             224  188 - #787c80
             584  196 - #a0a1a3
              80  200 - #62666b
             384  200 - #393d43
              40  204 - #f6f8fa
             164  204 - #bbbec1
             300  204 - #f6f8fa
             456  204 - #252a30
             500  204 - #a9acaf
             584  204 - #a0a1a3
             584  208 - #a1a2a3
             340  220 - #c7cacd
             284  228 - #5a5f64
             164  236 - #b2cefc
             200  236 - #b2cefc
              52  240 - #b5bbc1
              80  244 - #62666b
             136  244 - #b2cefc
             108  248 - #e4e6e8
             188  248 - #b2cefc
             232  248 - #b2cefc
              44  264 - #b2cefc
             556  264 - #dee0e2
             572  264 - #e8eaec
              88  268 - #9fb7e0
             156  268 - #b2cefc
             216  268 - #b2cefc
             244  268 - #93aad0
             120  272 - #b2cefc
             580  272 - #e5e6e8
             188  276 - #b2cefc
             544  276 - #d2d4d6
             544  280 - #c8cacc
             580  284 - #cbcdcf
              40  288 - #b2cefc
              88  288 - #9fb7e0
             232  288 - #f6f8fa
             544  288 - #c7c8ca
             100  292 - #a1bae3
             544  292 - #cfd1d3
             140  296 - #b2cefc
             580  296 - #e2e3e5
             556  300 - #bcbdbf
             564  300 - #b6b8b9
             568  300 - #bfc0c2
             184  308 - #24292f
             364  308 - #505459
             460  308 - #5a5e63
              56  328 - #a0a0a0
             136  328 - #030303
             152  340 - #000000
              28  344 - #a9a9a9
              80  344 - #6c6c6c
             152  344 - #000000
             424  448 - #ffffff
             252  496 - #ffffff
             592  504 - #ffffff
              92  592 - #ffffff
             408  592 - #ffffff
";

/// Recorded with `--record-colors`.
const TRIMMED_ABOVE: &str = r"
             232   12 - #d9dcde
             120   16 - #34393f
             196   16 - #dcdfe1
             284   16 - #5a5f64
             328   16 - #d9dcde
              88   32 - #82868a
             164   36 - #bbbec1
              44   40 - #7c848d
              80   56 - #62666b
             140   56 - #393e44
             224   56 - #62666b
             192   60 - #4d5257
             164   80 - #bbbec1
             584   84 - #a0a1a3
              40   96 - #f6f8fa
             260   96 - #24292f
              80  100 - #62666b
             108  100 - #a5a8ac
             204  100 - #a5a8ac
             300  100 - #f6f8fa
             340  100 - #a5a8ac
             380  100 - #5e6267
             484  104 - #cdcfd2
             584  104 - #a0a1a3
             144  120 - #9ca0a3
             584  124 - #a0a1a3
             184  140 - #24292f
             224  140 - #62666b
             100  144 - #dcdfe1
             584  144 - #a0a1a3
             584  152 - #a0a1a3
              56  160 - #bdc2c8
             584  160 - #a1a2a4
              52  164 - #818992
             164  164 - #bbbec1
              80  188 - #787c80
              88  188 - #787c80
             224  188 - #787c80
              80  200 - #62666b
             192  200 - #4d5257
              40  204 - #f6f8fa
             300  204 - #f6f8fa
             404  204 - #a9acaf
             456  204 - #252a30
             512  204 - #f6f8fa
             348  228 - #f6f8fa
             160  236 - #b2cefc
             212  236 - #b2cefc
             252  236 - #b2cefc
              52  240 - #b5bbc1
              80  244 - #62666b
             136  244 - #b2cefc
             108  248 - #e4e6e8
             232  248 - #b2cefc
              44  264 - #b2cefc
             216  264 - #b2cefc
             556  264 - #dee0e2
             572  264 - #e8eaec
              88  268 - #9fb7e0
             156  268 - #b2cefc
             188  268 - #b2cefc
             244  268 - #93aad0
             120  272 - #b2cefc
             580  272 - #e5e6e8
             544  276 - #d2d4d6
             544  280 - #c8cacc
             580  284 - #cbcdcf
              40  288 - #b2cefc
              88  288 - #9fb7e0
             232  288 - #f6f8fa
             544  288 - #c7c8ca
             100  292 - #a1bae3
             544  292 - #cfd1d3
             136  296 - #b2cefc
             580  296 - #e2e3e5
             556  300 - #bcbdbf
             564  300 - #b6b8b9
             568  300 - #bfc0c2
             184  308 - #24292f
             380  308 - #5e6267
             460  308 - #5a5e63
              28  324 - #a9a9a9
             176  340 - #cdcdcd
              80  344 - #000000
             124  344 - #c3c3c3
             136  344 - #000000
              32  360 - #000000
              88  360 - #101010
             144  360 - #000000
             148  360 - #000000
             444  452 - #ffffff
             292  460 - #ffffff
             592  504 - #ffffff
               4  524 - #ffffff
             156  592 - #ffffff
             408  592 - #ffffff
";

/// Recorded with `--record-colors`.
const TRIMMED_INTO: &str = r"
              48   16 - #b2cefc
             124   16 - #b2cefc
             148   16 - #b2cefc
             216   16 - #b2cefc
             584   16 - #a0a1a3
              80   20 - #4e5a6b
             188   20 - #b2cefc
             252   24 - #b2cefc
             100   28 - #a1bae3
             204   28 - #b2cefc
             164   32 - #b2cefc
             232   32 - #b2cefc
              40   44 - #b2cefc
              76   44 - #b2cefc
             136   44 - #b2cefc
              88   48 - #9fb7e0
             100   48 - #a1bae3
             204   48 - #e4e6e8
             584   52 - #a0a1a3
             244   68 - #c9cbce
             364   68 - #505459
             416   68 - #f6f8fa
             484   72 - #24292f
             328   88 - #d9dcde
             584   88 - #a0a1a3
             144   92 - #9ca0a3
             192   92 - #4d5257
             284   92 - #5a5f64
             100  112 - #dcdfe1
             584  124 - #a0a1a3
             184  128 - #24292f
              44  136 - #f6f8fa
              80  136 - #b7babd
             136  136 - #b7babd
             224  136 - #b7babd
             584  144 - #a0a1a3
             108  152 - #787c80
             164  152 - #bbbec1
             204  152 - #787c80
             584  160 - #a0a1a3
             384  168 - #393d43
              56  172 - #bdc2c8
             184  172 - #24292f
             312  176 - #34393f
             456  176 - #252a30
             504  176 - #f6f8fa
             232  192 - #d9dcde
             144  196 - #9ca0a3
             284  196 - #5a5f64
              88  216 - #d9dcde
             108  216 - #3c4146
             204  216 - #3c4146
             244  216 - #c9cbce
              44  220 - #f6f8fa
             128  236 - #9da0a4
             192  252 - #4d5257
              32  256 - #a7adb3
             224  256 - #62666b
             164  260 - #bbbec1
             552  264 - #e6e8ea
             560  264 - #dadcdd
             572  264 - #e8eaec
              88  280 - #d9dcde
             300  280 - #f6f8fa
             384  280 - #393d43
             416  280 - #f6f8fa
             456  280 - #24292f
             544  280 - #c8cacc
             580  280 - #cfd1d3
             544  288 - #c7c8ca
             580  288 - #ced0d1
             544  292 - #cfd1d3
             580  296 - #e2e3e5
              88  300 - #393e44
             144  300 - #9ca0a3
             184  300 - #d9dcde
             216  300 - #252a30
             252  300 - #f6f8fa
             348  300 - #393e44
             560  300 - #b5b7b8
             568  300 - #bfc0c2
             552  304 - #e1e3e5
              28  324 - #a9a9a9
              64  332 - #000000
             120  332 - #0d0d0d
              72  344 - #101010
             148  344 - #020202
              32  360 - #000000
              64  360 - #dddddd
              88  360 - #101010
             448  440 - #ffffff
             284  464 - #ffffff
             592  504 - #ffffff
               4  524 - #ffffff
             156  592 - #ffffff
             408  592 - #ffffff
";

/// Recorded with `--record-colors`.
const AT_END: &str = r"
             144   20 - #9ca0a3
             120   24 - #34393f
             192   24 - #4d5257
             216   24 - #24292f
             252   24 - #f6f8fa
             284   24 - #5a5f64
             328   24 - #d9dcde
             164   40 - #24292f
             584   40 - #a0a1a3
              88   44 - #d9dcde
             108   44 - #7b7f83
             204   44 - #7b7f83
             224   48 - #a2a5a9
              56   64 - #90979f
             108   64 - #8d9195
             164   64 - #bbbec1
             584   76 - #a0a1a3
             128   80 - #72767b
             192   80 - #4d5257
             224   80 - #62666b
             100   88 - #dcdfe1
              44  100 - #a8aeb5
             128  100 - #7e8186
              80  108 - #62666b
             164  108 - #bbbec1
             224  108 - #62666b
             384  108 - #393d43
             460  108 - #5a5e63
             512  108 - #f6f8fa
             584  108 - #a0a1a3
             184  124 - #82868a
             328  124 - #82868a
             144  128 - #9ca0a3
             252  128 - #f6f8fa
             292  132 - #393e44
             584  140 - #a0a1a3
             192  144 - #4d5257
             164  148 - #bbbec1
             224  148 - #62666b
              96  152 - #4d5257
             184  168 - #252a30
              44  172 - #f6f8fa
             136  172 - #96999d
             584  172 - #a0a1a3
              96  192 - #4d5257
             196  192 - #e1e4e6
             224  192 - #62666b
             260  208 - #d6d8db
             584  208 - #a0a1a3
              44  212 - #f6f8fa
              80  212 - #62666b
             108  212 - #63686c
             164  212 - #bbbec1
             192  212 - #4d5257
             376  212 - #f6f8fa
             416  212 - #f6f8fa
             456  212 - #24292f
             512  212 - #f6f8fa
             340  228 - #c7cacd
             252  232 - #f6f8fa
             120  236 - #34393f
             144  236 - #9ca0a3
             196  236 - #dcdfe1
             300  236 - #f6f8fa
             584  240 - #a0a1a3
              44  248 - #7f8790
             224  252 - #62666b
              88  256 - #d9dcde
             164  256 - #bbbec1
             184  272 - #24292f
             584  272 - #a0a1a3
             100  276 - #dcdfe1
             244  276 - #c9cbce
              80  280 - #787c80
             224  280 - #787c80
              56  296 - #707982
             108  296 - #b7babd
             164  296 - #bbbec1
             204  296 - #b7babd
             584  304 - #a0a1a3
             112  328 - #a0a0a0
             148  328 - #525252
              32  340 - #2c2c2c
             172  340 - #626262
              32  344 - #000000
              80  344 - #848484
             204  344 - #c3c3c3
             412  412 - #ffffff
             556  448 - #ffffff
             284  456 - #ffffff
              40  472 - #ffffff
             160  512 - #ffffff
             452  548 - #ffffff
               4  592 - #ffffff
             316  592 - #ffffff
             592  592 - #ffffff
";
