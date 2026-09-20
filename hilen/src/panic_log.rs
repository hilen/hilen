//! A shipped desktop app has no console and Android swallows stderr, so the
//! log is the only place a panic can reach. Without this a crash at a user's
//! machine leaves a log file that just stops.

use std::{
    backtrace::Backtrace,
    panic::{set_hook, take_hook},
};

use log::error;

/// The earlier hook still runs after the log line. A test runner installs its
/// own before the engine starts, and the default one keeps the terminal
/// output of a dev run as it is.
pub(crate) fn install() {
    let earlier = take_hook();
    set_hook(Box::new(move |info| {
        error!("{info}\nBacktrace: {}", Backtrace::force_capture());
        earlier(info);
    }));
}
