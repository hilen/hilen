#![cfg(target_arch = "wasm32")]

//! The browser logger. Every line goes to the console like before, and
//! under a test driver the lines at info and above also go over the
//! inspect socket, so the driver prints the app's progress live the way
//! the desktop lane does and a stuck run names the last thing the app
//! said. The browser lane cannot read the console, see `docs/ui-tests.md`.

use log::{Level, LevelFilter, Log, Metadata, Record};

struct WebLogger;

impl Log for WebLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        console_log::log(record);

        #[cfg(feature = "inspect")]
        if record.level() <= Level::Info && crate::inspect::web_transport::is_open() {
            crate::inspect::web_transport::push_log(record.level().as_str(), record.args().to_string());
        }
    }

    fn flush(&self) {}
}

static LOGGER: WebLogger = WebLogger;

pub(crate) fn init() {
    log::set_logger(&LOGGER).expect("Couldn't initialize logger");
    log::set_max_level(LevelFilter::Debug);
}
