use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{Label, LogData, LogLine, LogView, Setup, ViewData, ViewFrame, ViewTest, WHITE, view},
    ui_test::check_colors,
};

use crate::log_probe::{line, status_style, test_style};

const LOG_HEIGHT: f32 = 380.0;
const WIDE: f32 = 580.0;
const NARROW: f32 = 340.0;

const LONG: &str = "a long line that does not fit into the width of the log, so it wraps and goes on under \
                    its own start, beside the prefix column";

/// A log with short lines and 1 long line. The long line wraps, its row
/// is as tall as its text and the rows under it move down. Its prefix
/// stays in the prefix column. A narrower log wraps the line into more
/// rows, a wider log into fewer.
#[view]
struct LogViewWrap {
    lines: Vec<LogLine>,

    #[init]
    log:    LogView,
    status: Label,
}

impl Setup for LogViewWrap {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        self.lines = vec![
            LogLine::new("a short line").with_prefix("10:00:01"),
            LogLine::new(LONG).with_prefix("10:00:02"),
            LogLine::new("the line after the long one").with_prefix("10:00:03"),
            LogLine::new("a line with no prefix starts in the text column"),
            LogLine::new("the last line").with_prefix("10:00:05"),
        ];

        self.log.set_style(test_style());
        self.log.set_data_source(self);
        self.log.set_frame((10.0, 10.0, WIDE, LOG_HEIGHT));

        status_style(self.status);
        self.status.set_text("5 lines, the second one is long and wraps");
        self.status.place().t(LOG_HEIGHT + 20.0).lr(10).h(180);
    }
}

impl LogData for LogViewWrap {
    fn number_of_lines(&self) -> usize {
        self.lines.len()
    }

    fn line(&self, index: usize) -> LogLine {
        self.lines[index].clone()
    }
}

/// The frames a step of the test compares.
struct Rows {
    short:        f32,
    long:         f32,
    long_top:     f32,
    after_top:    f32,
    long_x:       f32,
    no_prefix_x:  f32,
    prefix_right: f32,
}

impl LogViewWrap {
    fn rows(self: Weak<Self>) -> Rows {
        from_main(move || {
            let long = line(self.log, "a long line");
            Rows {
                short:        line(self.log, "a short line").height(),
                long:         long.height(),
                long_top:     long.absolute_frame().y(),
                after_top:    line(self.log, "the line after").absolute_frame().y(),
                long_x:       long.absolute_frame().x(),
                no_prefix_x:  line(self.log, "no prefix").absolute_frame().x(),
                prefix_right: line(self.log, "10:00:02").absolute_frame().max_x(),
            }
        })
    }

    fn resize(self: Weak<Self>, width: f32, status: &'static str) {
        from_main(move || {
            self.log.set_frame((10.0, 10.0, width, LOG_HEIGHT));
            self.status.set_text(status);
        });
        // The log lays its rows out in the update of the next frame, and
        // the frame after it draws them.
        wait_for_next_frame();
        wait_for_next_frame();
    }
}

impl ViewTest for LogViewWrap {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        wait_for_next_frame();
        wait_for_next_frame();

        let wide = view.rows();
        ensure!(
            wide.long > wide.short * 1.5,
            "the long line is {} high, a short one {}, it did not wrap",
            wide.long,
            wide.short
        );
        ensure!(
            (wide.after_top - (wide.long_top + wide.long)).abs() < 0.5,
            "the row after the long line starts at {}, the long line ends at {}",
            wide.after_top,
            wide.long_top + wide.long
        );
        ensure!(
            (wide.long_x - wide.no_prefix_x).abs() < 0.5,
            "the text of a line with no prefix starts at {}, the text column at {}",
            wide.no_prefix_x,
            wide.long_x
        );
        ensure!(
            wide.prefix_right <= wide.long_x + 0.5,
            "the prefix ends at {}, the text starts at {}",
            wide.prefix_right,
            wide.long_x
        );
        check_colors(WIDE_LOG)?;

        view.resize(NARROW, "the log is narrower\nthe long line takes more rows");
        let narrow = view.rows();
        ensure!(
            narrow.long > wide.long,
            "the long line is {} high in the narrow log and {} in the wide one",
            narrow.long,
            wide.long
        );
        ensure!(
            (narrow.after_top - (narrow.long_top + narrow.long)).abs() < 0.5,
            "the row after the long line starts at {}, the long line ends at {}",
            narrow.after_top,
            narrow.long_top + narrow.long
        );
        check_colors(NARROW_LOG)?;

        view.resize(WIDE, "the log is wide again\nthe rows are as at the start");
        let again = view.rows();
        ensure!(
            (again.long - wide.long).abs() < 0.5 && (again.after_top - wide.after_top).abs() < 0.5,
            "the wide log came back with other rows: long {} at first {}",
            again.long,
            wide.long
        );
        check_colors(WIDE_AGAIN)?;

        Ok(())
    }
}

/// Recorded with `--record-colors`.
const WIDE_LOG: &str = r"
              60   24 - #6f7881
             244   44 - #808488
             376   44 - #24292f
             436   44 - #808488
             524   44 - #5f6368
             168   48 - #34393f
             224   48 - #e4e6e8
             340   48 - #dcdfe1
             484   48 - #e4e6e8
             308   68 - #252a30
             532   72 - #24292f
             376   88 - #d9dcde
             436   88 - #dfe1e4
             144   92 - #4d5257
              92  112 - #f6f8fa
             188  112 - #5f6368
             284  112 - #5a5f64
             176  132 - #909397
             212  132 - #93979a
             292  132 - #909397
             348  132 - #a6a9ad
             412  132 - #464a50
             484  132 - #909397
             572  132 - #4a4e54
              44  148 - #f6f8fa
             468  380 - #f6f8fa
              76  408 - #ffffff
             164  408 - #a0a0a0
             208  408 - #ffffff
             224  408 - #c3c3c3
             360  592 - #ffffff
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const NARROW_LOG: &str = r"
             592    4 - #ffffff
              60   24 - #6f7881
             224   48 - #e4e6e8
             124   68 - #464a50
             176   68 - #62666b
             252   68 - #a6a9ad
             136   88 - #d9dcde
             220   92 - #464a50
             168  108 - #e6e9eb
             140  132 - #5f6368
             244  132 - #dcdfe1
             300  132 - #a6a9ad
             184  136 - #b7babd
             168  176 - #bbbec1
              44  196 - #f6f8fa
             284  196 - #5a5f64
             148  216 - #dcdfe1
             196  216 - #3c4146
             128  220 - #63686c
             236  260 - #5a5f64
             272  260 - #f6f8fa
              44  296 - #f6f8fa
             196  296 - #acafb2
             564  300 - #ffffff
             164  304 - #24292f
             224  304 - #24292f
              32  420 - #2c2c2c
              80  424 - #aaaaaa
              84  424 - #111111
              92  424 - #020202
             148  424 - #000000
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const WIDE_AGAIN: &str = r"
              60   24 - #6f7881
             376   44 - #24292f
             436   44 - #808488
             168   48 - #34393f
             224   48 - #e4e6e8
             340   48 - #dcdfe1
             484   48 - #e4e6e8
             272   68 - #62666b
             476   68 - #5a5f64
             540   68 - #929599
             436   88 - #dfe1e4
             144   92 - #4d5257
             364  108 - #464a50
              44  112 - #f6f8fa
             188  112 - #5f6368
             332  112 - #5f6368
             176  132 - #909397
             292  132 - #909397
             484  132 - #909397
             556  132 - #252a30
             148  136 - #b7babd
             404  136 - #b7babd
              92  148 - #f6f8fa
             220  152 - #f6f8fa
             428  392 - #ffffff
             124  408 - #000000
             144  408 - #dcdcdc
             132  412 - #000000
              28  424 - #525252
             172  424 - #000000
             324  592 - #ffffff
             592  592 - #ffffff
";
