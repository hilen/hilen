use std::mem::take;

use anyhow::{Result, ensure};
use chrono::Utc;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{Label, LogData, LogLine, LogView, Setup, ViewData, ViewFrame, ViewTest, WHITE, view},
    ui_test::checkpoint,
};
use parking_lot::Mutex;

use crate::log_probe::{line, status_style, test_style};

/// The lines the log asked its data for, in the order of the calls.
static ASKED: Mutex<Vec<usize>> = Mutex::new(Vec::new());

const START: usize = 200;
const ADDED: usize = 10_000;
/// The rows on screen and the 2 rows of slack are asked a second time,
/// when their cells are set up.
const MOST_ON_SCREEN: usize = 40;
/// A debug build on a slow machine has to stay under this too. An
/// append that measured the old lines again would take many times
/// longer, the counts below catch that first.
const TIME_LIMIT_MS: i64 = 4000;

fn text_of(index: usize) -> String {
    if index.is_multiple_of(7) {
        format!(
            "line {index}: a longer line that wraps in a log of this width, so rows have different heights"
        )
    } else {
        format!("line {index}: a short line")
    }
}

/// 10 000 lines come to the end of a log that already has 200. The log
/// asks its data only for the new lines, the 200 before them are not
/// measured again, and the append stays under a time limit. The status
/// label shows the counts and the time.
#[view]
struct LogViewAppend {
    count: usize,

    #[init]
    log:    LogView,
    status: Label,
}

impl Setup for LogViewAppend {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        self.count = START;

        self.log.set_style(test_style());
        self.log.set_data_source(self);
        self.log.set_frame((10.0, 10.0, 580.0, 380.0));

        status_style(self.status);
        self.status.set_text("200 lines");
        self.status.place().t(400).lr(10).h(180);
    }
}

impl LogData for LogViewAppend {
    fn number_of_lines(&self) -> usize {
        self.count
    }

    fn line(&self, index: usize) -> LogLine {
        ASKED.lock().push(index);
        LogLine::new(text_of(index)).with_prefix(format!("{index:05}"))
    }
}

impl ViewTest for LogViewAppend {
    fn perform_test(mut view: Weak<Self>) -> Result<()> {
        wait_for_next_frame();
        wait_for_next_frame();
        ensure!(
            from_main(move || line(view.log, "line 199:").is_ok()),
            "the log does not show its end"
        );
        checkpoint("200 lines, the log shows line 199 at its end")?;
        take(&mut *ASKED.lock());

        let took = from_main(move || {
            view.count += ADDED;
            let start = Utc::now();
            view.log.lines_added();
            (Utc::now() - start).num_milliseconds()
        });
        wait_for_next_frame();
        wait_for_next_frame();

        let asked = take(&mut *ASKED.lock());
        let old = asked.iter().filter(|index| **index < START).count();
        let mut seen = vec![false; START + ADDED];
        for index in &asked {
            seen[*index] = true;
        }
        let missed = seen[START..].iter().filter(|seen| !**seen).count();
        let calls = asked.len();

        from_main(move || {
            view.status.set_text(format!(
                "{ADDED} lines were added in {took} ms\nthe data was asked {calls} times\n{old} of them for \
                 a line from before"
            ));
        });
        wait_for_next_frame();
        checkpoint("10000 lines added, the log shows line 10199 at its end")?;

        ensure!(old == 0, "{old} lines from before the append were asked again");
        ensure!(missed == 0, "{missed} new lines were never asked");
        ensure!(
            calls <= ADDED + MOST_ON_SCREEN,
            "the data was asked {calls} times for {ADDED} new lines"
        );
        ensure!(
            from_main(move || line(view.log, "line 10199:").is_ok()),
            "the log did not follow the new lines"
        );
        ensure!(
            took < TIME_LIMIT_MS,
            "{ADDED} lines took {took} ms, the limit is {TIME_LIMIT_MS} ms"
        );

        Ok(())
    }
}
