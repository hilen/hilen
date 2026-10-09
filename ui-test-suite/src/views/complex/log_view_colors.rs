use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        Color, DynamicColor, Label, LogData, LogLine, LogView, Setup, Theme, ThemeMode, ViewData, ViewFrame,
        ViewTest, view,
    },
    ui_test::{check_colors, set_record_probe_count},
};

use crate::log_probe::{line, status_style, test_style};

/// What each line of the fixture holds, with its ANSI codes.
const LINES: [(&str, &str); 9] = [
    ("\x1b[36mapi\x1b[0m", "plain text with no codes"),
    ("\x1b[36mapi\x1b[0m", "\x1b[31mred: the request failed\x1b[0m"),
    (
        "\x1b[35mworker\x1b[0m",
        "\x1b[32mgreen: done\x1b[0m and plain after the reset",
    ),
    ("\x1b[35mworker\x1b[0m", "\x1b[1;33mbold yellow warning\x1b[0m"),
    ("\x1b[36mapi\x1b[0m", "\x1b[2mdim: a debug line\x1b[0m"),
    (
        "\x1b[36mapi\x1b[0m",
        "\x1b[94mbright blue\x1b[0m \x1b[4munderlined\x1b[0m \x1b[9mstruck\x1b[0m",
    ),
    ("", "\x1b[38;5;208mcolor 208 of the table\x1b[0m, no prefix"),
    ("", "\x1b[38;2;200;0;120mthe color 200, 0, 120\x1b[0m"),
    (
        "\x1b[36mapi\x1b[0m",
        "\x1b[2K\x1b]0;a title\x07codes that are no colors are dropped",
    ),
];

/// A log whose lines hold ANSI codes. Every line says what it shows:
/// the 16 colors of the style, bold, dim, underlined and struck text,
/// the table of 256 colors and a free color. The prefixes are colored
/// too. No code is left in a text. The colors of the style follow the
/// theme.
#[view]
struct LogViewColors {
    #[init]
    log:    LogView,
    status: Label,
}

impl Setup for LogViewColors {
    fn setup(mut self: Weak<Self>) {
        self.set_color(DynamicColor::new(Color::hex("#ffffff"), Color::hex("#1c1c1e")));

        self.log.set_style(test_style());
        self.log.set_data_source(self);
        self.log.set_frame((10.0, 10.0, 580.0, 380.0));

        status_style(self.status);
        self.status
            .set_text_color(DynamicColor::new(Color::hex("#000000"), Color::hex("#ffffff")));
        self.status.set_text("light theme\nevery line says how it is drawn");
        self.status.place().t(400).lr(10).h(180);
    }
}

impl LogData for LogViewColors {
    fn number_of_lines(&self) -> usize {
        LINES.len()
    }

    fn line(&self, index: usize) -> LogLine {
        let (prefix, text) = LINES[index];
        LogLine::new(text).with_prefix(prefix)
    }
}

fn set_theme(view: Weak<LogViewColors>, mode: ThemeMode, status: &'static str) {
    from_main(move || {
        Theme::set_mode(mode);
        view.status.set_text(status);
    });
    wait_for_next_frame();
}

impl ViewTest for LogViewColors {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(96);
        wait_for_next_frame();
        wait_for_next_frame();

        // (the text as drawn, color runs, font runs)
        let drawn = [
            ("plain text with no codes", 0, 0),
            ("red: the request failed", 1, 0),
            ("green: done and plain after the reset", 1, 0),
            ("bold yellow warning", 1, 1),
            ("dim: a debug line", 1, 0),
            ("bright blue underlined struck", 1, 2),
            ("color 208 of the table, no prefix", 1, 0),
            ("the color 200, 0, 120", 1, 0),
            ("codes that are no colors are dropped", 0, 0),
        ];
        for (text, color_runs, font_runs) in drawn {
            let found = from_main(move || {
                let label = line(view.log, text);
                (
                    label.text().to_string(),
                    label.color_runs_len(),
                    label.font_runs_len(),
                )
            });
            ensure!(
                found == (text.to_string(), color_runs, font_runs),
                "the line `{text}` is drawn as {found:?}"
            );
        }
        let prefix = from_main(move || {
            let label = line(view.log, "worker");
            (label.text().to_string(), label.color_runs_len())
        });
        ensure!(
            prefix == ("worker".to_string(), 1),
            "the prefix is drawn as {prefix:?}"
        );
        check_colors(LIGHT)?;

        set_theme(
            view,
            ThemeMode::Dark,
            "dark theme\nthe colors of the style changed with it",
        );
        check_colors(DARK)?;

        set_theme(view, ThemeMode::System, "light theme again");
        check_colors(LIGHT_AGAIN)?;

        Ok(())
    }
}

/// Recorded with `--record-colors`.
const LIGHT: &str = r"
             224   24 - #62666b
              44   28 - #f6f8fa
             144   28 - #4d5257
             244   28 - #e1e4e6
             320   28 - #f6f8fa
             300   44 - #d12f3b
             168   48 - #d33843
             192   48 - #e59da3
             216   48 - #cf232f
             284   48 - #e28890
             300   48 - #d12f3b
              60   64 - #966ee4
             340   64 - #979b9e
              52   68 - #8251df
              80   68 - #c4b0ee
             116   68 - #12642a
             128   68 - #116329
             136   68 - #116329
             144   68 - #3e8052
             192   68 - #3e8052
             204   68 - #116329
             244   68 - #c7cacd
             272   68 - #62666b
             396   68 - #24292f
             444   68 - #24292f
              60   84 - #966ee4
             140   84 - #a97f29
              80   88 - #c4b0ee
             120   88 - #9a6801
             184   88 - #c7ae7a
             220   88 - #9a6700
             240   88 - #b08a3d
             284   88 - #9e6e0c
              52   92 - #8250df
              80   92 - #c4b0ee
             140   92 - #a97f29
             240   92 - #b08a3d
             268   92 - #9a6700
             232  108 - #e5e7e9
             128  112 - #d1d4d7
             216  112 - #787c80
             184  124 - #61acfd
              52  128 - #1c7c83
             156  128 - #268eff
             172  128 - #459dfe
             272  128 - #62666b
             116  132 - #218bff
             140  132 - #71b4fd
             156  132 - #a5cffc
             172  132 - #459dfe
             204  132 - #8ec3fc
             244  132 - #c7cacd
             300  132 - #909397
             328  132 - #24292f
             340  132 - #24292f
             356  132 - #24292f
             372  132 - #24292f
             128  136 - #b7d8fb
             184  136 - #b7d8fb
             280  136 - #b7babd
             240  148 - #fd9d31
             300  148 - #fe8e10
             128  152 - #fca84a
             144  152 - #fac88f
             172  152 - #ff8701
             212  152 - #f9d8b3
             232  152 - #ff8700
             240  152 - #fd9d31
             252  152 - #fbb464
             300  152 - #fe8e10
             396  152 - #7f8287
             364  156 - #44494e
             220  172 - #c80078
             248  172 - #eab6d7
             256  172 - #f1e0ed
             296  172 - #eab6d7
             304  172 - #f1e0ed
             120  176 - #cd1985
             184  176 - #e390c3
             196  192 - #bfc2c5
             300  192 - #31363c
              44  196 - #f6f8fa
             280  196 - #f6f8fa
             356  196 - #24292f
             412  200 - #5a5e63
             592  356 - #ffffff
             380  396 - #ffffff
              28  404 - #a9a9a9
              72  408 - #a0a0a0
              92  412 - #0d0d0d
             180  420 - #d3d3d3
              48  424 - #777777
              64  424 - #434343
             120  424 - #dddddd
             348  592 - #ffffff
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const DARK: &str = r"
             312   20 - #e6edf3
             224   24 - #a6acb2
             108   28 - #0d1117
             148   28 - #272c32
             264   28 - #d5dce2
             300   44 - #f0746c
             168   48 - #e67069
             192   48 - #743e3e
             284   48 - #8b4846
             300   48 - #f0746c
             216   52 - #fe7b72
              60   64 - #9d76d6
             272   64 - #a6acb2
             340   64 - #6f747a
              52   68 - #bb8cfe
              72   68 - #bc8cff
              80   68 - #58467a
             116   68 - #3fb850
             144   68 - #359845
             192   68 - #359845
             204   68 - #3fb950
             244   68 - #3e4248
             284   68 - #7e848a
             384   68 - #bbc2c8
             444   68 - #e6edf3
              60   84 - #9d76d6
             140   84 - #b28320
              52   88 - #bc8cff
              60   88 - #9d76d6
              80   88 - #58467a
             184   88 - #72571d
             220   88 - #d29922
             240   88 - #a2781f
             248   88 - #483a1a
             268   88 - #d29922
              52   92 - #bc8cff
              60   92 - #9d76d6
             104   92 - #3e331a
             240   92 - #a2781f
             248   92 - #483a1a
             284   92 - #c99321
             124  112 - #82888e
             216  112 - #8f959b
             184  124 - #588bb9
              52  128 - #39c4ce
             172  128 - #67a2d8
             116  132 - #79c0ff
             128  132 - #598dbb
             140  132 - #507ea8
             144  132 - #649ed2
             156  132 - #36546f
             172  132 - #67a2d8
             204  132 - #426688
             244  132 - #3e4248
             272  132 - #a6acb2
             328  132 - #e6edf3
             344  132 - #e6edf3
             364  132 - #e6edf3
             380  132 - #e6edf3
              52  136 - #1a474e
             280  136 - #4e5359
             128  148 - #b86407
             232  148 - #ff8700
             240  148 - #d07005
             300  148 - #f08001
             128  152 - #b86407
             144  152 - #74430d
             172  152 - #fe8700
             212  152 - #513211
             240  152 - #d07005
             252  152 - #9e5809
             300  152 - #f08001
             396  152 - #888e94
             364  156 - #c5cbd1
             220  172 - #c80078
             592  172 - #1c1c1e
             120  176 - #b5026e
             184  176 - #5b0a40
             196  192 - #464b51
              44  196 - #0d1117
             172  196 - #c3cad0
             280  196 - #0d1117
             348  196 - #0d1117
             412  200 - #afb5bb
             592  356 - #1c1c1e
              48  404 - #959596
              32  420 - #d8d8d8
              32  424 - #ffffff
              76  424 - #3a3a3c
             100  424 - #363638
             156  424 - #1c1c1e
             200  424 - #ffffff
             252  424 - #acacad
             256  424 - #717173
             380  592 - #1c1c1e
             592  592 - #1c1c1e
";

/// Recorded with `--record-colors`.
const LIGHT_AGAIN: &str = r"
             224   24 - #62666b
              44   28 - #f6f8fa
             148   28 - #dcdfe1
             244   28 - #e1e4e6
             264   28 - #34393f
             300   44 - #d12f3b
             168   48 - #d33843
             192   48 - #e59da3
             216   48 - #cf232f
             284   48 - #e28890
             300   48 - #d12f3b
              60   64 - #966ee4
             272   64 - #62666b
             340   64 - #979b9e
             376   64 - #24292f
              52   68 - #8251df
              80   68 - #c4b0ee
             116   68 - #12642a
             128   68 - #116329
             136   68 - #116329
             144   68 - #3e8052
             192   68 - #3e8052
             204   68 - #116329
             244   68 - #c7cacd
             284   68 - #888c90
             444   68 - #24292f
              60   84 - #966ee4
             140   84 - #a97f29
              80   88 - #c4b0ee
             120   88 - #9a6801
             168   88 - #9a6700
             184   88 - #c7ae7a
             220   88 - #9a6700
             240   88 - #b08a3d
              52   92 - #8250df
              80   92 - #c4b0ee
             140   92 - #a97f29
             240   92 - #b08a3d
             280   96 - #9a6801
             232  108 - #e5e7e9
             128  112 - #d1d4d7
             216  112 - #787c80
             184  124 - #61acfd
              52  128 - #1c7c83
             156  128 - #268eff
             172  128 - #459dfe
             272  128 - #62666b
             116  132 - #218bff
             144  132 - #4ba0fe
             156  132 - #a5cffc
             172  132 - #459dfe
             196  132 - #cae1fb
             204  132 - #8ec3fc
             272  132 - #62666b
             328  132 - #24292f
             348  132 - #24292f
             360  132 - #24292f
             376  132 - #24292f
              52  136 - #b5d3d7
             128  136 - #b7d8fb
             184  136 - #b7d8fb
             272  136 - #b7babd
             232  148 - #ff8700
             240  148 - #fd9d31
             300  148 - #fe8e10
             128  152 - #fca84a
             144  152 - #fac88f
             172  152 - #ff8701
             212  152 - #f9d8b3
             240  152 - #fd9d31
             252  152 - #fbb464
             300  152 - #fe8e10
             396  152 - #7f8287
             364  156 - #44494e
             220  172 - #c80078
             248  172 - #eab6d7
             256  172 - #f1e0ed
             296  172 - #eab6d7
             304  172 - #f1e0ed
             120  176 - #cd1985
             184  176 - #e390c3
             196  192 - #bfc2c5
              44  196 - #f6f8fa
             172  196 - #464a50
             280  196 - #f6f8fa
             356  196 - #24292f
             444  196 - #f6f8fa
             412  200 - #5a5e63
             588  368 - #f6f8fa
             384  400 - #ffffff
              28  404 - #a9a9a9
              72  408 - #a0a0a0
             124  408 - #434343
              92  412 - #0d0d0d
             320  592 - #ffffff
             592  592 - #ffffff
";
