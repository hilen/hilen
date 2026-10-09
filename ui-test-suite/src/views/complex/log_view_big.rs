use std::fmt::Write;

use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{LogData, LogLine, LogView, Setup, ViewData, ViewTest, view},
    ui_test::{checkpoint, inject_scroll, inject_touches},
};

use crate::log_probe::line_on_screen;

const LINES: usize = 100_000;

/// The name, and the ANSI color of the name.
const SERVICES: [(&str, u8); 6] = [
    ("api", 36),
    ("worker", 35),
    ("db", 34),
    ("gateway", 33),
    ("auth", 32),
    ("scheduler", 96),
];

const WORDS: [&str; 8] = ["every", "word", "of", "this", "line", "has", "another", "color"];

/// A number that looks random and is the same on every run.
fn mixed(index: usize, salt: u64) -> usize {
    let mut value = (index as u64).wrapping_add(salt).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    usize::try_from(value % 1_000_000).unwrap_or_default()
}

fn prefix_of(index: usize) -> String {
    let second = index / 7;
    let (service, color) = SERVICES[mixed(index, 1) % SERVICES.len()];
    format!(
        "\x1b[2m{:02}:{:02}:{:02}.{:03}\x1b[0m \x1b[{color}m{service:<9}\x1b[0m",
        second / 3600 % 24,
        second / 60 % 60,
        second % 60,
        mixed(index, 2) % 1000
    )
}

fn text_of(index: usize) -> String {
    let id = mixed(index, 3);
    let ms = mixed(index, 4) % 900 + 1;
    let number = format!("\x1b[90m#{index}\x1b[0m");

    match mixed(index, 5) % 12 {
        0..=3 => format!(
            "{number} \x1b[32mINFO\x1b[0m  GET \x1b[4m/api/v1/users/{id}\x1b[0m \x1b[1;32m200\x1b[0m in \
             \x1b[36m{ms}ms\x1b[0m"
        ),
        4 => format!(
            "{number} \x1b[1;33mWARN\x1b[0m  slow query took \x1b[33m{ms}ms\x1b[0m: \x1b[2mSELECT * FROM \
             orders WHERE user_id = {id} AND status IN ('open', 'held') ORDER BY created_at DESC LIMIT \
             50\x1b[0m"
        ),
        5 => format!(
            "{number} \x1b[1;31mERROR\x1b[0m \x1b[31mconnection to 10.0.{}.{}:5432 refused\x1b[0m, retry \
             \x1b[1m{}\x1b[0m of 5. The pool has no free connection and the request waits. \x1b[2mat \
             src/net/pool.rs:{}, at src/net/client.rs:{}, at src/handlers/orders.rs:{}\x1b[0m",
            id % 255,
            ms % 255,
            id % 5 + 1,
            ms + 40,
            id % 400,
            ms
        ),
        6 => format!("{number} \x1b[2mDEBUG cache hit for the key user:{id}:profile, ttl 300s\x1b[0m"),
        7 => format!(
            "{number} {{\x1b[38;5;75m\"event\"\x1b[0m: \x1b[38;5;180m\"order.created\"\x1b[0m, \
             \x1b[38;5;75m\"order\"\x1b[0m: \x1b[38;5;141m{id}\x1b[0m, \x1b[38;5;75m\"total\"\x1b[0m: \
             \x1b[38;5;141m{ms}.50\x1b[0m, \x1b[38;5;75m\"paid\"\x1b[0m: \x1b[38;5;208mtrue\x1b[0m}}"
        ),
        8 => {
            let mut text = number;
            for (place, word) in WORDS.iter().enumerate() {
                let color = 16 + (id + place * 29) % 216;
                write!(text, " \x1b[38;5;{color}m{word}\x1b[0m").unwrap_or_default();
            }
            text
        }
        9 => {
            let mut text = format!("{number} deploy ");
            for step in 0..40 {
                let red = 255 - step * 6;
                let green = 60 + step * 4;
                let blue = (id + step * 5) % 256;
                write!(text, "\x1b[38;2;{red};{green};{blue}m#").unwrap_or_default();
            }
            write!(text, "\x1b[0m \x1b[1m{}%\x1b[0m", id % 101).unwrap_or_default();
            text
        }
        10 => format!(
            "{number} \x1b[9mthe old endpoint /v0/ping\x1b[0m is gone, use \x1b[4;94m/v1/health\x1b[0m"
        ),
        _ => format!("{number} plain text with no color code, {id} bytes were written"),
    }
}

/// A big log to look at and to play with: 100 000 lines with levels,
/// colors of all 3 kinds, bold, dim, underlined and struck text, long
/// lines that wrap and a few lines with no prefix. Open it with
/// `--present`. The lines are made from their index, the data keeps
/// nothing. The test jumps to the first line and back to the end.
#[view]
struct LogViewBig {
    #[init]
    log: LogView,
}

impl Setup for LogViewBig {
    fn setup(self: Weak<Self>) {
        self.log.set_data_source(self);
        self.log.place().back();
    }
}

impl LogData for LogViewBig {
    fn number_of_lines(&self) -> usize {
        LINES
    }

    fn line(&self, index: usize) -> LogLine {
        let line = LogLine::new(text_of(index));
        // A line with no prefix, like the next row of a stack trace.
        if mixed(index, 6).is_multiple_of(23) {
            line
        } else {
            line.with_prefix(prefix_of(index))
        }
    }
}

impl LogViewBig {
    fn shows(self: Weak<Self>, part: &'static str) -> bool {
        from_main(move || line_on_screen(self.log, part).is_some())
    }
}

impl ViewTest for LogViewBig {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        wait_for_next_frame();
        wait_for_next_frame();
        ensure!(
            view.shows("#99999") && from_main(move || view.log.is_following()),
            "a new log of {LINES} lines does not show its end"
        );
        checkpoint("100000 lines, the log shows the last one, #99999")?;

        // The wheel goes to the view under the pointer.
        inject_touches("300 300 b\n300 300 e");
        inject_scroll(1_000_000_000);
        wait_for_next_frame();
        wait_for_next_frame();
        ensure!(view.shows("#0 "), "the log does not show its first line");
        ensure!(
            !from_main(move || view.log.is_following()),
            "the log still follows at its first line"
        );
        checkpoint("scrolled to the top, the first line is #0")?;

        inject_scroll(-1_000_000_000);
        wait_for_next_frame();
        wait_for_next_frame();
        ensure!(
            view.shows("#99999") && from_main(move || view.log.is_following()),
            "the log did not come back to its end"
        );
        checkpoint("scrolled back to the end")?;

        Ok(())
    }
}
