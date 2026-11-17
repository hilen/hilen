//! The feed tests run with no window and no frame loop. The test thread
//! stands in for the main thread and runs the queued calls by hand, the way
//! the event loop does for a covered window.

use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    thread::sleep,
    time::{Duration, Instant},
};

use serial_test::serial;

use crate::{
    audio::{
        SoundClock,
        clock_feed::timer_runs,
        clock_test::{Output, click},
    },
    deps::hreads::{invoke_dispatched, set_current_thread_as_main},
};

/// A wait that only a broken timer uses up.
const TOO_LONG: Duration = Duration::from_secs(5);
/// More than 3 periods of the timer.
const SOME_PERIODS: Duration = Duration::from_millis(350);

/// Runs what is queued for the main thread until `done`. False when that
/// took too long.
fn run_main_until(mut done: impl FnMut() -> bool) -> bool {
    let start = Instant::now();
    loop {
        invoke_dispatched();
        if done() {
            return true;
        }
        if start.elapsed() > TOO_LONG {
            return false;
        }
        sleep(Duration::from_millis(2));
    }
}

fn run_main_for(time: Duration) {
    let start = Instant::now();
    while start.elapsed() < time {
        invoke_dispatched();
        sleep(Duration::from_millis(2));
    }
}

/// Gives `clock` a feed that counts its calls.
fn count_calls(clock: &mut SoundClock) -> Arc<AtomicUsize> {
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = calls.clone();
    clock.set_feed(move |_| {
        counted.fetch_add(1, Ordering::Relaxed);
    });
    calls
}

fn count(calls: &AtomicUsize) -> usize {
    calls.load(Ordering::Relaxed)
}

#[test]
#[serial]
fn the_feed_is_called_on_a_timer_with_no_frame_drawn() {
    set_current_thread_as_main();
    let mut clock = SoundClock::silent(120.0);
    let calls = Arc::new(AtomicUsize::new(0));
    let beat = Arc::new(AtomicU64::new(0));
    let (counted, seen) = (calls.clone(), beat.clone());
    clock.set_feed(move |clock| {
        counted.fetch_add(1, Ordering::Relaxed);
        seen.store(clock.beats().to_bits(), Ordering::Relaxed);
    });

    clock.start();
    // The app is in the middle of its own code in `start`, the first call
    // waits for the main thread.
    assert_eq!(count(&calls), 0);

    let start = Instant::now();
    assert!(run_main_until(|| count(&calls) >= 4));
    // The call at the start and 3 periods of 0.1 seconds.
    assert!(start.elapsed() >= Duration::from_millis(250));
    // The feed read the time of the clock, which went on.
    assert!(f64::from_bits(beat.load(Ordering::Relaxed)) > 0.4);

    drop(clock);
    assert!(run_main_until(|| !timer_runs()));
}

#[test]
#[serial]
fn a_feed_queues_sounds_on_the_clock_it_gets() {
    set_current_thread_as_main();
    let (mut clock, sound) = SoundClock::connected(120.0);
    let mut output = Output::new(sound);
    let mut queued = false;
    clock.set_feed(move |clock| {
        if queued {
            return;
        }
        queued = true;
        assert!(clock.is_running());
        clock.play_at_seconds(&click(), 0.01, 1.0);
        clock.set_bpm(90.0);
    });

    clock.start();
    // The clock of the app and the clock the feed got are 1 clock.
    assert!(run_main_until(|| (clock.bpm() - 90.0).abs() < 1e-6));
    output.render(4, 480, 128);
    // 0.01 seconds is sample 480.
    assert_eq!(output.starts(), [480]);

    drop(clock);
    assert!(run_main_until(|| !timer_runs()));
}

#[test]
#[serial]
fn a_stopped_clock_gets_no_calls_until_it_starts_again() {
    set_current_thread_as_main();
    let mut clock = SoundClock::silent(120.0);
    let calls = count_calls(&mut clock);
    clock.start();
    assert!(run_main_until(|| count(&calls) >= 1));

    clock.stop();
    let at_stop = count(&calls);
    assert!(run_main_until(|| !timer_runs()));
    run_main_for(SOME_PERIODS);
    assert_eq!(count(&calls), at_stop);

    clock.start();
    assert!(run_main_until(|| count(&calls) > at_stop));

    drop(clock);
    assert!(run_main_until(|| !timer_runs()));
}

#[test]
#[serial]
fn a_dropped_clock_gets_no_calls_and_its_feed_is_dropped() {
    set_current_thread_as_main();
    let mut clock = SoundClock::silent(120.0);
    let calls = count_calls(&mut clock);
    clock.start();
    assert!(run_main_until(|| count(&calls) >= 1));

    drop(clock);
    let at_drop = count(&calls);
    assert!(run_main_until(|| !timer_runs()));
    run_main_for(SOME_PERIODS);
    assert_eq!(count(&calls), at_drop);
    // The feed held the other half of the counter.
    assert_eq!(Arc::strong_count(&calls), 1);
}

#[test]
#[serial]
fn the_timer_lives_only_while_a_running_clock_has_a_feed() {
    set_current_thread_as_main();
    assert!(!timer_runs());

    // A feed on a stopped clock.
    let mut first = SoundClock::silent(120.0);
    let first_calls = count_calls(&mut first);
    assert!(!timer_runs());

    // A running clock with no feed.
    let mut second = SoundClock::silent(120.0);
    second.start();
    assert!(!timer_runs());
    run_main_for(SOME_PERIODS);
    assert_eq!(count(&first_calls), 0);

    // The feed comes while the clock runs.
    let second_calls = count_calls(&mut second);
    assert!(timer_runs());
    first.start();
    assert!(run_main_until(
        || count(&first_calls) >= 1 && count(&second_calls) >= 1
    ));

    // 1 of 2 clocks stops, the other one is still fed.
    second.stop();
    let (first_before, second_before) = (count(&first_calls), count(&second_calls));
    assert!(run_main_until(|| count(&first_calls) >= first_before + 2));
    assert_eq!(count(&second_calls), second_before);
    assert!(timer_runs());

    first.stop();
    assert!(run_main_until(|| !timer_runs()));
}
