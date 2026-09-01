use std::{
    any::Any,
    panic::{AssertUnwindSafe, catch_unwind},
};

use anyhow::Result;
use log::error;
use parking_lot::Mutex;

use super::failure_report;
use crate::{
    dispatch::from_main,
    ui::{Theme, ThemeMode, UIManager},
};

/// One failed test. `detail` holds the returned error or the panic message
/// together with the failure report.
pub struct TestFailure {
    pub name:   String,
    pub detail: String,
}

static FAILURES: Mutex<Vec<TestFailure>> = Mutex::new(Vec::new());

/// Drop every recorded failure. The full suite calls this before a run so a
/// second run in the same process starts clean.
pub fn clear_failures() {
    FAILURES.lock().clear();
}

/// Take and clear the failures collected so far.
pub fn take_failures() -> Vec<TestFailure> {
    std::mem::take(&mut FAILURES.lock())
}

pub fn any_failed() -> bool {
    !FAILURES.lock().is_empty()
}

fn record(name: &str, detail: String) {
    error!("{name}: FAILED");
    FAILURES.lock().push(TestFailure {
        name: name.to_string(),
        detail,
    });
}

/// Record a failure from outside the runner, used by the panic hook for a main
/// thread panic that `catch_unwind` here cannot reach.
pub fn push_failure(name: &str, detail: String) {
    record(name, detail);
}

fn panic_message(panic: &(dyn Any + Send)) -> String {
    if let Some(s) = panic.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = panic.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic".to_string()
    }
}

/// Run one registered test. Catches a returned `Err` and a panic, records the
/// failure, and returns without propagating, so the run keeps going and every
/// failure is reported at the end.
pub fn run_test(name: &str, test: impl FnOnce() -> Result<()>) {
    #[cfg(not_wasm)]
    super::watchdog::test_started(name);

    // A headed window follows the OS theme, so on a dark desktop every
    // light color block failed while headless passed. The theme is part
    // of the environment a test must not depend on, and a test that
    // switched it must not leak into the next. Drag scrolling goes back
    // to the platform default the same way, a finger gesture test turns
    // it on in `before_start`.
    from_main(|| {
        Theme::set_mode(ThemeMode::System);
        Theme::set_system(Theme::Light);
        UIManager::set_drag_scrolling(UIManager::default_drag_scrolling());
        // Fallback fonts are global too, a test that registers one must
        // not leak it into the next.
        crate::ui::Font::reset_fallbacks();
        // Frame stepped time is a global opt in, so a stepped test that
        // returned early must not leave the clock stepped for the next.
        crate::gm::Clock::exit_stepped();
        // The dialog animation is global app state, a test that registers
        // one must not leak it into the next. The suite snapshot hands the
        // app's own animation back after the run.
        crate::BugReport::restore_animation(None);
    });

    match catch_unwind(AssertUnwindSafe(test)) {
        Ok(Ok(())) => {}
        Ok(Err(err)) => record(name, format!("{err:?}")),
        Err(panic) => {
            let report = failure_report().unwrap_or_else(|e| format!("failed to collect report: {e}"));
            record(name, format!("panic: {}\n{report}", panic_message(&*panic)));
        }
    }

    #[cfg(not_wasm)]
    super::watchdog::test_finished();
}
