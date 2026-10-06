//! The engine's own memory of the desktop window. The default
//! `App::window_placement` reads it and the default
//! `App::window_placement_changed` feeds it, so an app that writes neither
//! hook opens where it was closed.

use std::{
    env::var_os,
    fs::{create_dir_all, read_to_string, rename, write},
    io::ErrorKind,
    mem::replace,
    path::{Path, PathBuf},
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    thread::{Builder, sleep},
    time::Duration,
};

use anyhow::Result;
use log::{debug, info, warn};
use parking_lot::Mutex;

use crate::{filesystem::Paths, window::WindowPlacement};

/// The name of the file in the data folder of the app.
pub(crate) const FILE: &str = "window_placement.json";

/// How long a changed placement waits for the next one before it is
/// written. A drag of the window edge fires many times a second, all of
/// them inside one wait end as 1 write.
const WRITE_DELAY: Duration = Duration::from_millis(500);

/// Set once at start, and only for a real launch with a window. While it
/// is empty nothing is read and nothing is written.
static STORE: OnceLock<Arc<PlacementStore>> = OnceLock::new();

/// On while a UI test run inside a real app owns the window.
static PAUSED: AtomicBool = AtomicBool::new(false);

/// Whether this launch keeps its window. A run with no window has
/// nothing to keep, and a test window has to open at the size the test
/// asks for, never at one a run before it left behind.
fn remembered(headless: bool, test_storage: bool, test_env: bool) -> bool {
    !headless && !test_storage && !test_env
}

/// The spellings that turn a real app into a test run, see
/// `AppRunner::spawn_test_autorun`.
fn test_env() -> bool {
    var_os("HILEN_RUN_TESTS").is_some() || var_os("HILEN_PRESENT").is_some()
}

/// Turns the memory on for this launch, when it is one that keeps its
/// window.
pub(crate) fn open(headless: bool) {
    if !remembered(headless, Paths::test_storage(), test_env()) {
        debug!("window placement is not kept in a headless or test run");
        return;
    }
    let file = Paths::storage().join(FILE);
    info!("window placement file {}", file.display());
    if STORE.set(PlacementStore::new(file, WRITE_DELAY)).is_err() {
        warn!("window placement memory was opened twice");
    }
}

/// The placement the last run left, `None` on the first start.
pub(crate) fn saved() -> Option<WindowPlacement> {
    STORE.get()?.load()
}

/// Takes a fresh placement. Cheap, the write happens later on another
/// thread.
pub(crate) fn save(placement: &WindowPlacement) {
    if PAUSED.load(Ordering::Relaxed) {
        return;
    }
    if let Some(store) = STORE.get() {
        store.save(placement);
    }
}

/// Writes what still waits, on the calling thread. The event loop calls
/// it on its way out, so the last placement is on disk when the process
/// ends.
pub(crate) fn flush() {
    if let Some(store) = STORE.get() {
        store.flush();
    }
}

/// A UI test run started inside a real app resizes the window of that
/// app. Those sizes are not what the user chose.
pub(crate) fn pause() {
    PAUSED.store(true, Ordering::Relaxed);
}

pub(crate) fn resume() {
    PAUSED.store(false, Ordering::Relaxed);
}

/// What waits for the disk. This is the rule that keeps a burst of
/// resizes from writing every time, with no clock and no thread in it.
#[derive(Default)]
struct Queue {
    waiting:    Option<WindowPlacement>,
    on_disk:    Option<WindowPlacement>,
    timer_runs: bool,
}

impl Queue {
    /// Keeps the newest placement. `true` asks the caller to start the
    /// one timer of this burst, every later call inside it only replaces
    /// what waits.
    fn push(&mut self, placement: WindowPlacement) -> bool {
        if self.waiting.is_none() && self.on_disk.as_ref() == Some(&placement) {
            return false;
        }
        self.waiting = Some(placement);
        !replace(&mut self.timer_runs, true)
    }

    /// What to write now, `None` when the disk already has it.
    fn take(&mut self) -> Option<WindowPlacement> {
        let placement = self.waiting.take()?;
        if self.on_disk.as_ref() == Some(&placement) {
            return None;
        }
        self.on_disk = Some(placement.clone());
        Some(placement)
    }

    fn timer_fired(&mut self) -> Option<WindowPlacement> {
        self.timer_runs = false;
        self.take()
    }
}

struct PlacementStore {
    file:    PathBuf,
    delay:   Duration,
    queue:   Mutex<Queue>,
    /// Held for the whole write. The exit flush waits here for a write
    /// the timer thread is in, and the main thread never takes it during
    /// a resize, so a slow disk cannot hold a frame.
    writing: Mutex<()>,
}

impl PlacementStore {
    fn new(file: PathBuf, delay: Duration) -> Arc<Self> {
        Arc::new(Self {
            file,
            delay,
            queue: Mutex::default(),
            writing: Mutex::default(),
        })
    }

    /// A file that cannot be read is the same as no file. A broken one
    /// must never stop the app from opening.
    fn load(&self) -> Option<WindowPlacement> {
        let json = match read_to_string(&self.file) {
            Ok(json) => json,
            Err(err) if err.kind() == ErrorKind::NotFound => return None,
            Err(err) => {
                warn!("failed to read {}: {err}", self.file.display());
                return None;
            }
        };
        match serde_json::from_str::<WindowPlacement>(&json) {
            Ok(placement) => {
                self.queue.lock().on_disk = Some(placement.clone());
                Some(placement)
            }
            Err(err) => {
                warn!("{} is not a window placement: {err}", self.file.display());
                None
            }
        }
    }

    fn save(self: &Arc<Self>, placement: &WindowPlacement) {
        if !self.queue.lock().push(placement.clone()) {
            return;
        }
        let store = Arc::clone(self);
        let timer = Builder::new().name("window placement".to_string()).spawn(move || {
            sleep(store.delay);
            store.write(Queue::timer_fired, "after the wait");
        });
        if let Err(err) = timer {
            warn!("no thread to write the window placement, it waits for the exit: {err}");
            self.queue.lock().timer_runs = false;
        }
    }

    fn flush(&self) {
        self.write(Queue::take, "at the exit");
    }

    fn write(&self, take: fn(&mut Queue) -> Option<WindowPlacement>, when: &str) {
        let writing = self.writing.lock();
        let Some(placement) = take(&mut self.queue.lock()) else {
            return;
        };
        match write_file(&self.file, &placement) {
            Ok(()) => debug!("window placement saved {when}: {placement:?}"),
            Err(err) => {
                warn!("failed to write {}: {err:#}", self.file.display());
                // Not on disk after all, the next change tries again.
                self.queue.lock().on_disk = None;
            }
        }
        drop(writing);
    }
}

/// Through a second file and a rename, so a process that dies in the
/// middle leaves the old file whole.
fn write_file(file: &Path, placement: &WindowPlacement) -> Result<()> {
    let json = serde_json::to_string_pretty(placement)?;
    if let Some(dir) = file.parent() {
        create_dir_all(dir)?;
    }
    let fresh = file.with_extension("json.tmp");
    write(&fresh, json)?;
    rename(&fresh, file)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{env::temp_dir, fs::remove_dir_all, time::Instant};

    use super::*;

    /// Longer than any test runs, the timer never fires inside one.
    const NEVER: Duration = Duration::from_secs(3600);

    fn placement(width: f64) -> WindowPlacement {
        WindowPlacement {
            width,
            height: 700.0,
            x: 120.0,
            y: 80.0,
            maximized: false,
            monitor: Some("Built-in".to_string()),
        }
    }

    /// A folder of its own for each test, they run at the same time. The
    /// same one on every run, so the temp dir does not fill up.
    fn fresh_file(test: &str) -> Result<PathBuf> {
        let dir = temp_dir().join(format!("hilen-placement-test-{test}"));
        if dir.exists() {
            remove_dir_all(&dir)?;
        }
        Ok(dir.join("data").join(FILE))
    }

    #[test]
    fn a_saved_placement_loads_back_in_a_new_run() -> Result<()> {
        let file = fresh_file("round-trip")?;
        let mut saved = placement(1300.0);
        saved.maximized = true;

        let store = PlacementStore::new(file.clone(), NEVER);
        store.save(&saved);
        store.flush();

        // A new store is the next start of the app.
        assert_eq!(PlacementStore::new(file, NEVER).load(), Some(saved));
        Ok(())
    }

    #[test]
    fn nothing_saved_loads_nothing() -> Result<()> {
        let file = fresh_file("nothing")?;
        assert_eq!(PlacementStore::new(file, NEVER).load(), None);
        Ok(())
    }

    /// The app must still open, at its first start size.
    #[test]
    fn a_broken_file_loads_nothing() -> Result<()> {
        let file = fresh_file("broken")?;
        create_dir_all(file.parent().expect("the file has a folder"))?;
        write(&file, r#"{"width":13"#)?;
        assert_eq!(PlacementStore::new(file, NEVER).load(), None);
        Ok(())
    }

    #[test]
    fn a_burst_starts_1_timer_and_writes_only_the_last_placement() {
        let mut queue = Queue::default();

        let timers = (0..200_u32)
            .filter(|step| queue.push(placement(800.0 + f64::from(*step))))
            .count();

        assert_eq!(timers, 1);
        assert_eq!(queue.timer_fired(), Some(placement(999.0)));
        assert_eq!(queue.timer_fired(), None);

        // The burst is over, the next change starts a timer of its own.
        assert!(queue.push(placement(640.0)));
    }

    #[test]
    fn a_placement_the_disk_already_has_is_not_written_again() {
        let mut queue = Queue::default();
        assert!(queue.push(placement(900.0)));
        assert_eq!(queue.timer_fired(), Some(placement(900.0)));

        // The resize the restore itself fires at the next launch.
        assert!(!queue.push(placement(900.0)));
        assert_eq!(queue.take(), None);

        // Out and back inside 1 burst ends where the disk is.
        assert!(queue.push(placement(950.0)));
        assert!(!queue.push(placement(900.0)));
        assert_eq!(queue.timer_fired(), None);
    }

    #[test]
    fn a_burst_writes_nothing_until_the_wait_is_over_and_the_exit_writes_the_last() -> Result<()> {
        let file = fresh_file("burst")?;
        let store = PlacementStore::new(file.clone(), NEVER);

        for step in 0..200_u32 {
            store.save(&placement(800.0 + f64::from(step)));
        }
        assert!(!file.exists(), "a resize wrote to disk before the wait was over");

        store.flush();
        assert_eq!(PlacementStore::new(file, NEVER).load(), Some(placement(999.0)));
        Ok(())
    }

    #[test]
    fn the_timer_writes_the_last_placement_by_itself() -> Result<()> {
        let file = fresh_file("timer")?;
        let store = PlacementStore::new(file.clone(), Duration::from_millis(20));

        for step in 0..50_u32 {
            store.save(&placement(800.0 + f64::from(step)));
        }

        let reader = PlacementStore::new(file, NEVER);
        let started = Instant::now();
        while reader.load() != Some(placement(849.0)) {
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "the timer did not write the last placement"
            );
            sleep(Duration::from_millis(5));
        }
        Ok(())
    }

    #[test]
    fn a_headless_run_and_a_test_run_keep_no_window() {
        assert!(remembered(false, false, false));
        assert!(!remembered(true, false, false));
        assert!(!remembered(false, true, false));
        assert!(!remembered(false, false, true));
    }

    /// No test of this binary opens the memory, like a UI test runner,
    /// which starts through `Paths::use_test_storage`.
    #[test]
    fn a_run_that_never_opened_the_memory_reads_and_writes_nothing() {
        save(&placement(900.0));
        flush();
        assert_eq!(saved(), None);
    }
}
