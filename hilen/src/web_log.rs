#![cfg(target_arch = "wasm32")]

//! The browser logger. Every line goes to the console like before, and
//! under a test driver the lines at info and above also go over the
//! inspect socket, so the driver prints the app's progress live the way
//! the desktop lane does and a stuck run names the last thing the app
//! said. The browser lane cannot read the console, see `docs/ui-tests.md`.

use std::sync::OnceLock;

use log::{Level, LevelFilter, Log, Metadata, Record};

use crate::app::is_debug_target;

struct WebLogger;

static APP_TARGETS: OnceLock<&'static [&'static str]> = OnceLock::new();

impl Log for WebLogger {
    // The same filter as the native logger. On an old TV the debug lines of
    // naga alone were about 13700 at start, and the console keeps them all.
    fn enabled(&self, metadata: &Metadata) -> bool {
        let app = APP_TARGETS.get().copied().unwrap_or_default();
        metadata.level() <= Level::Warn
            || (metadata.level() <= Level::Debug && is_debug_target(metadata.target(), app))
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        console_log::log(record);

        // The bug report ring keeps the same lines the console shows, like
        // the native dispatch does.
        crate::bug_report::BugReport::push_log_line(record.args().to_string());

        #[cfg(feature = "inspect")]
        if record.level() <= Level::Info && crate::inspect::web_transport::is_open() {
            crate::inspect::web_transport::push_log(record.level().as_str(), record.args().to_string());
        }
    }

    fn flush(&self) {}
}

static LOGGER: WebLogger = WebLogger;

pub(crate) fn init(app_targets: &'static [&'static str]) {
    log::set_logger(&LOGGER).expect("Couldn't initialize logger");
    log::set_max_level(LevelFilter::Debug);
    if APP_TARGETS.set(app_targets).is_err() {
        log::warn!("The log targets of the app were set twice, the first ones stay");
    }
}
