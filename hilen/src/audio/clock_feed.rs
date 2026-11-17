//! The timer behind `SoundClock::set_feed`. It lives only while a running
//! clock has a feed, so an app with no such clock does no timer work. It
//! never waits for a frame: a covered window draws none, and the sound has
//! to go on there.

#[cfg(all(test, not_wasm))]
use std::sync::atomic::AtomicUsize;
#[cfg(not_wasm)]
use std::{
    sync::mpsc::{Receiver, RecvTimeoutError, SyncSender, TrySendError, sync_channel},
    thread::Builder,
};
use std::{
    sync::{
        Arc, Weak,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

#[cfg(not_wasm)]
use log::error;
// The main thread of a browser must never park on a lock, see the dispatch
// queue of hreads.
#[cfg(not_wasm)]
use parking_lot::Mutex;
#[cfg(wasm)]
use spin::Mutex;

#[cfg(wasm)]
use crate::deps::hreads::{sleep, spawn};
use crate::{
    audio::clock::{SharedState as Clock, SoundClock},
    deps::hreads::on_main,
};

/// The time from one feed call to the next. A feed queues half a second
/// ahead, so a call can come 0.4 seconds late before a sound is late.
const FEED_PERIOD: Duration = Duration::from_millis(100);

/// Wakes the timer thread before its sleep is over.
#[cfg(not_wasm)]
type Timer = SyncSender<()>;
/// A browser has 1 thread, the timer is a task on it.
#[cfg(wasm)]
type Timer = ();

struct Feeds {
    /// The running clocks that have a feed.
    clocks: Vec<Weak<Clock>>,
    timer:  Option<Timer>,
}

static FEEDS: Mutex<Feeds> = Mutex::new(Feeds {
    clocks: Vec::new(),
    timer:  None,
});

/// A feed call waits in the queue of the main thread. A main thread that is
/// busy gets 1 call then, not one more for every period.
static CALL_WAITS: AtomicBool = AtomicBool::new(false);

/// A clock with a feed started, or a running clock got its feed. Its first
/// call comes at once.
pub(super) fn add(clock: &Arc<Clock>) {
    let clock = Arc::downgrade(clock);
    let mut feeds = FEEDS.lock();
    if !feeds.clocks.iter().any(|known| known.ptr_eq(&clock)) {
        feeds.clocks.push(clock);
    }
    feeds.wake_timer();
}

/// The clock stopped or is dropped.
pub(super) fn remove(clock: &Arc<Clock>) {
    let clock = Arc::downgrade(clock);
    let mut feeds = FEEDS.lock();
    feeds.clocks.retain(|known| !known.ptr_eq(&clock));
    // The task of a browser ends by itself after its sleep. Ending it here
    // would start a second task when a clock starts during that sleep.
    #[cfg(not_wasm)]
    if feeds.clocks.is_empty() {
        feeds.end_timer();
    }
}

/// A hot build that is stopped must leave no thread, see
/// `docs/hot-reload.md`.
#[cfg(hot)]
pub(super) fn stop() {
    let mut feeds = FEEDS.lock();
    feeds.clocks.clear();
    #[cfg(not_wasm)]
    feeds.end_timer();
}

/// The feed threads that have not ended yet.
#[cfg(all(test, not_wasm))]
static THREADS: AtomicUsize = AtomicUsize::new(0);

#[cfg(all(test, not_wasm))]
pub(super) fn timer_runs() -> bool {
    FEEDS.lock().timer.is_some() || THREADS.load(Ordering::Relaxed) > 0
}

#[cfg(not_wasm)]
impl Feeds {
    fn wake_timer(&mut self) {
        if let Some(timer) = &self.timer {
            match timer.try_send(()) {
                // Full means the thread is already asked to wake.
                Ok(()) | Err(TrySendError::Full(())) => return,
                Err(TrySendError::Disconnected(())) => {
                    error!("The feed thread of the sound clocks ended early, a new one starts");
                }
            }
        }
        let (timer, wakes) = sync_channel(1);
        match Builder::new().name("sound clock feed".into()).spawn(move || run(&wakes)) {
            Ok(thread) => {
                // The thread ends by itself with the last clock.
                drop(thread);
                self.timer = Some(timer);
            }
            Err(err) => {
                self.timer = None;
                error!("No thread for the feed of the sound clocks, no feed is called: {err}");
            }
        }
    }

    /// The thread sees the closed channel and ends.
    fn end_timer(&mut self) {
        self.timer = None;
    }
}

#[cfg(not_wasm)]
fn run(wakes: &Receiver<()>) {
    #[cfg(test)]
    THREADS.fetch_add(1, Ordering::Relaxed);
    loop {
        queue_call();
        match wakes.recv_timeout(FEED_PERIOD) {
            Ok(()) | Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    #[cfg(test)]
    THREADS.fetch_sub(1, Ordering::Relaxed);
}

#[cfg(wasm)]
impl Feeds {
    fn wake_timer(&mut self) {
        if self.timer.is_some() {
            // Not from inside `start`, the app is in the middle of its own
            // code there.
            spawn(async { queue_call() });
            return;
        }
        self.timer = Some(());
        spawn(async {
            while clocks_left() {
                queue_call();
                sleep(FEED_PERIOD.as_secs_f32()).await;
            }
        });
    }
}

/// False ends the task.
#[cfg(wasm)]
fn clocks_left() -> bool {
    let mut feeds = FEEDS.lock();
    if feeds.clocks.is_empty() {
        feeds.timer = None;
        return false;
    }
    true
}

fn queue_call() {
    if CALL_WAITS.swap(true, Ordering::Relaxed) {
        return;
    }
    on_main(|| {
        CALL_WAITS.store(false, Ordering::Relaxed);
        call_feeds();
    });
}

fn call_feeds() {
    let clocks: Vec<_> = FEEDS.lock().clocks.iter().filter_map(Weak::upgrade).collect();
    for clock in clocks {
        SoundClock::call_feed(clock);
    }
}
