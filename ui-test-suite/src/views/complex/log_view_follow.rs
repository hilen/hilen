use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{Label, LogData, LogLine, LogView, Setup, ViewData, ViewFrame, ViewTest, WHITE, view},
    ui_test::{check_colors, inject_scroll, inject_touches},
};

use crate::log_probe::{line, line_on_screen, status_style, test_style};

const LOG_WIDTH: f32 = 420.0;
const LOG_HEIGHT: f32 = 300.0;
const START: usize = 40;
const BATCH: usize = 5;

/// A log that gets new lines. While it shows its end it follows them.
/// After a scroll up it stays where the user left it, and a round button
/// with an arrow shows in its bottom right corner. A tap on the button
/// jumps to the end, the log follows again and the button is gone.
#[view]
struct LogViewFollow {
    count: usize,

    #[init]
    log:    LogView,
    status: Label,
}

impl Setup for LogViewFollow {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        self.count = START;

        self.log.set_style(test_style());
        self.log.set_data_source(self);
        self.log.set_frame((10.0, 10.0, LOG_WIDTH, LOG_HEIGHT));

        status_style(self.status);
        self.status.set_text("40 lines, the log shows its end\nno button");
        self.status.place().t(LOG_HEIGHT + 20.0).lr(10).h(260);
    }
}

impl LogData for LogViewFollow {
    fn number_of_lines(&self) -> usize {
        self.count
    }

    fn line(&self, index: usize) -> LogLine {
        LogLine::new(format!("line {index} of the log")).with_prefix(format!("{index:03}"))
    }
}

/// What a step looks at.
struct State {
    following: bool,
    /// The last line of the data is on screen and ends inside the log.
    end_shown: bool,
    /// Where `line 20` is on the screen, none when it is not shown.
    marker:    Option<f32>,
}

impl LogViewFollow {
    fn state(self: Weak<Self>) -> State {
        from_main(move || {
            let bottom = self.log.absolute_frame().max_y();
            let last = format!("line {} of", self.count - 1);
            State {
                following: self.log.is_following(),
                end_shown: line_on_screen(self.log, &last)
                    .is_some_and(|label| label.absolute_frame().max_y() <= bottom + 0.5),
                marker:    line_on_screen(self.log, "line 20 of").map(|label| label.absolute_frame().y()),
            }
        })
    }

    fn add(mut self: Weak<Self>, status: &'static str) {
        from_main(move || {
            self.count += BATCH;
            self.log.lines_added();
            self.status.set_text(status);
        });
        wait_for_next_frame();
        wait_for_next_frame();
    }
}

impl ViewTest for LogViewFollow {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        wait_for_next_frame();
        wait_for_next_frame();

        let start = view.state();
        ensure!(
            start.following && start.end_shown,
            "a new log does not show its end"
        );
        check_colors(AT_END)?;

        view.add("5 lines were added\nthe log followed them, line 44 is the last");
        let added = view.state();
        ensure!(
            added.following && added.end_shown,
            "the log did not follow the new lines"
        );
        check_colors(FOLLOWED)?;

        // The user scrolls up. The log stops following. The wheel goes to
        // the view under the pointer, a tap puts it over the log.
        inject_touches("200 150 b\n200 150 e");
        inject_scroll(350);
        wait_for_next_frame();
        from_main(move || {
            view.status
                .set_text("scrolled up\nthe log does not follow now\nthe button shows in the corner");
        });
        wait_for_next_frame();
        let up = view.state();
        ensure!(!up.following, "the log still follows after a scroll up");
        ensure!(!up.end_shown, "the scroll up left the end on screen");
        ensure!(
            up.marker.is_some(),
            "line 20 is not on screen after the scroll up"
        );
        check_colors(SCROLLED_UP)?;

        view.add("5 more lines were added\nthe log stayed where it was");
        let stayed = view.state();
        ensure!(!stayed.following, "new lines made the log follow again");
        ensure!(!stayed.end_shown, "the log jumped to the new lines");
        ensure!(
            stayed.marker == up.marker,
            "line 20 moved from {:?} to {:?} when lines were added",
            up.marker,
            stayed.marker
        );
        check_colors(STAYED)?;

        // The button, in the bottom right corner of the log.
        let button = from_main(move || {
            let frame = *view.log.absolute_frame();
            (frame.max_x() - 28.0, frame.max_y() - 28.0)
        });
        inject_touches(format!(
            "{} {} b\n{} {} e",
            button.0, button.1, button.0, button.1
        ));
        from_main(move || {
            view.status
                .set_text("a tap on the button\nthe log shows its end again, line 49\nthe button is gone");
        });
        wait_for_next_frame();
        wait_for_next_frame();
        let jumped = view.state();
        ensure!(
            jumped.following && jumped.end_shown,
            "the button did not bring the end back"
        );
        check_colors(JUMPED)?;

        view.add("5 lines were added\nthe log follows again, line 54 is the last");
        let follows = view.state();
        ensure!(
            follows.following && follows.end_shown,
            "the log does not follow after the jump"
        );
        check_colors(FOLLOWS_AGAIN)?;

        // A scroll back down by hand also follows again.
        inject_scroll(200);
        wait_for_next_frame();
        ensure!(
            !view.state().following,
            "the second scroll up did not stop the log"
        );
        inject_scroll(-2000);
        wait_for_next_frame();
        wait_for_next_frame();
        ensure!(
            view.state().following,
            "a scroll down to the end did not make the log follow"
        );
        ensure!(
            from_main(move || line(view.log, "line 54 of").is_ok()),
            "the last line is not on screen"
        );

        Ok(())
    }
}

/// Recorded with `--record-colors`.
const AT_END: &str = r"
              80   20 - #62666b
             224   20 - #62666b
             184   24 - #24292f
             108   44 - #7b7f83
              44   48 - #c0c4ca
             224   48 - #a3a6aa
             164   64 - #bbbec1
             224   80 - #62666b
             100   88 - #dcdfe1
              40  104 - #f6f8fa
             164  128 - #bbbec1
              80  132 - #393e44
             224  132 - #393e44
              40  168 - #f6f8fa
             136  172 - #f6f8fa
              88  188 - #252a30
             204  192 - #a5a8ac
             424  208 - #a0a1a3
             244  212 - #c9cbce
              52  248 - #7f8790
             136  248 - #3e4348
             424  256 - #a0a1a3
             184  272 - #24292f
              88  280 - #787c80
             232  296 - #f6f8fa
             424  304 - #a0a1a3
              32  324 - #d5d5d5
              96  328 - #a0a0a0
             172  328 - #000000
               4  592 - #ffffff
             300  592 - #ffffff
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const FOLLOWED: &str = r"
             592    4 - #ffffff
              80   20 - #62666b
             232   24 - #f6f8fa
             164   40 - #252a30
              40   64 - #f6f8fa
             108   64 - #8d9195
             224   80 - #62666b
             136  100 - #7e8186
             184  108 - #24292f
              56  128 - #6f7882
             224  132 - #393e44
             100  152 - #dcdfe1
             164  172 - #bbbec1
             204  192 - #a5a8ac
              80  196 - #cdcfd2
             244  212 - #c9cbce
             424  220 - #a0a1a3
             164  236 - #bbbec1
              88  272 - #43484d
             192  276 - #4d5257
             232  296 - #f6f8fa
             424  304 - #a0a1a3
             124  324 - #a0a0a0
             140  324 - #000000
              84  328 - #a0a0a0
             140  328 - #000000
              32  340 - #2c2c2c
              32  344 - #000000
             228  344 - #858585
             268  344 - #b7b7b7
             124  592 - #ffffff
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const SCROLLED_UP: &str = r"
              80   12 - #62666b
             204   16 - #f6f8fa
             136   36 - #f6f8fa
              88   60 - #4e5258
             196   80 - #e1e4e6
             128   96 - #45494f
              44  100 - #8a919a
             424  112 - #a0a1a3
              56  116 - #727b85
             224  144 - #24292f
             164  164 - #bbbec1
              88  184 - #d9dcde
             424  192 - #a0a1a3
              88  224 - #d9dcde
             204  224 - #a3a6aa
              44  228 - #a8aeb4
             128  228 - #7d8185
             396  264 - #dee0e2
              56  268 - #727b84
             192  268 - #4d5257
             384  280 - #c8cacc
             420  284 - #cbcdcf
             108  288 - #24292f
             232  288 - #f6f8fa
             380  292 - #e7e9eb
             404  300 - #b6b8b9
              88  328 - #000000
              28  344 - #525252
             132  344 - #737373
             196  360 - #000000
             324  592 - #ffffff
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const STAYED: &str = r"
              80   12 - #62666b
             204   16 - #f6f8fa
             136   36 - #f6f8fa
              88   60 - #4e5258
             224   76 - #62666b
             128   96 - #45494f
              44  100 - #8a919a
             424  104 - #a0a1a3
              56  116 - #727b85
              96  140 - #4d5257
             224  144 - #24292f
             164  164 - #bbbec1
             424  176 - #a0a1a3
              88  184 - #d9dcde
             204  224 - #a3a6aa
              44  228 - #a8aeb4
             128  228 - #7d8185
             396  264 - #dee0e2
              56  268 - #727b84
             100  268 - #dcdfe1
             384  280 - #c8cacc
             420  284 - #cbcdcf
             232  288 - #f6f8fa
             380  292 - #e7e9eb
             404  300 - #b6b8b9
              88  308 - #d9dcde
             184  308 - #24292f
             160  328 - #000000
             128  340 - #111111
              28  344 - #525252
             296  592 - #ffffff
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const JUMPED: &str = r"
             592    4 - #ffffff
              80   20 - #62666b
             204   44 - #7b7f83
             108   64 - #8d9195
             164   64 - #bbbec1
              52  104 - #f6f8fa
             184  104 - #24292f
              96  108 - #4d5257
             224  132 - #393e44
              80  144 - #62666b
             128  148 - #9da0a4
             184  148 - #24292f
             192  192 - #4d5257
              80  196 - #cdcfd2
             244  212 - #c9cbce
             424  228 - #a0a1a3
             164  236 - #bbbec1
              52  248 - #7f8790
             128  252 - #9da0a4
              80  276 - #62666b
             196  276 - #e1e4e6
             156  296 - #f6f8fa
             232  296 - #f6f8fa
             424  304 - #a0a1a3
             124  328 - #9d9d9d
              32  340 - #2c2c2c
              80  340 - #111111
             232  340 - #000000
             188  344 - #c3c3c3
             124  360 - #000000
             324  592 - #ffffff
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const FOLLOWS_AGAIN: &str = r"
             592    4 - #ffffff
              80   20 - #62666b
             232   24 - #f6f8fa
             164   40 - #252a30
              52   48 - #c0c4ca
              88   64 - #d9dcde
             204   64 - #8d9195
             136  100 - #7e8186
              44  108 - #6e7781
             184  108 - #24292f
              88  124 - #82868a
             224  132 - #393e44
             108  148 - #f6f8fa
             164  172 - #bbbec1
             204  192 - #a5a8ac
              80  196 - #cdcfd2
             244  212 - #c9cbce
             164  236 - #bbbec1
             424  236 - #a0a1a3
              44  248 - #7f8790
              88  272 - #43484d
             192  276 - #4d5257
             232  296 - #f6f8fa
             424  304 - #a0a1a3
             124  324 - #a0a0a0
             140  324 - #000000
             140  328 - #000000
              32  340 - #2c2c2c
              32  344 - #000000
             200  344 - #565656
             288  592 - #ffffff
             592  592 - #ffffff
";
