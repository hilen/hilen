use std::{
    sync::{
        Once,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use log::error;
use parking_lot::Mutex;
use web_time::Instant;

/// Seconds a single test may run before the watchdog starts shouting.
/// `HILEN_TEST_STUCK_TIMEOUT` overrides it for slow devices.
const DEFAULT_NOTIFY_SECS: u64 = 30;

/// The run aborts after this many notify intervals. A stuck test holds the
/// main thread dispatch and cannot be skipped, so ending the run with a clear
/// report is the only honest hard action.
const ABORT_AFTER_INTERVALS: u64 = 4;

struct Running {
    name:     String,
    started:  Instant,
    /// Seconds already reported, so every threshold prints once.
    reported: u64,
}

static RUNNING: Mutex<Option<Running>> = Mutex::new(None);
static STARTED_TESTS: AtomicUsize = AtomicUsize::new(0);

/// Called before the first test of a run, so an in-app rerun reports its own
/// test count instead of accumulating over the process lifetime.
pub(crate) fn start_run() {
    STARTED_TESTS.store(0, Ordering::Relaxed);
}

/// Arms the watchdog for one test. Human mode holds tests on purpose, a held
/// test is not a stuck test, so it never arms there.
pub(crate) fn test_started(name: &str) {
    if super::human_mode() {
        return;
    }

    STARTED_TESTS.fetch_add(1, Ordering::Relaxed);

    *RUNNING.lock() = Some(Running {
        name:     name.to_string(),
        started:  Instant::now(),
        reported: 0,
    });

    spawn_watchdog();
}

pub(crate) fn test_finished() {
    *RUNNING.lock() = None;
}

fn spawn_watchdog() {
    static SPAWNED: Once = Once::new();

    SPAWNED.call_once(|| {
        std::thread::spawn(|| {
            loop {
                std::thread::sleep(Duration::from_secs(1));
                check();
            }
        });
    });
}

fn check() {
    let notify = notify_secs();

    let mut slot = RUNNING.lock();

    let Some(running) = slot.as_mut() else {
        return;
    };

    let elapsed = running.started.elapsed().as_secs();

    if elapsed < running.reported + notify {
        return;
    }

    running.reported = elapsed;
    let name = running.name.clone();

    drop(slot);

    if elapsed >= notify * ABORT_AFTER_INTERVALS {
        abort_run(&name, elapsed);
    }

    error!("HILEN_TEST_STUCK {name} {elapsed}s");
}

fn notify_secs() -> u64 {
    std::env::var("HILEN_TEST_STUCK_TIMEOUT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_NOTIFY_SECS)
}

/// No failure report here. Collecting one drives the main thread, and the
/// stuck test may be holding exactly that, so the report would hang too.
fn abort_run(name: &str, elapsed: u64) -> ! {
    let total = STARTED_TESTS.load(Ordering::Relaxed);

    // The failures collected so far die with the process, so they print here.
    let failures = super::take_failures();
    let failed = failures.len() + 1;

    for failure in &failures {
        println!("TEST FAILED: {}\n{}", failure.name, failure.detail);
    }

    error!("TEST FAILED: {name}\nstuck for {elapsed} seconds, run aborted");

    println!("HILEN_TEST_RESULT {total} tests, {failed} failed");

    std::process::exit(1)
}
