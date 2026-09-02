use std::{
    any::type_name,
    sync::atomic::{AtomicBool, Ordering},
};

use log::{info, trace};
use parking_lot::Mutex;
use web_time::Instant;

use crate::{
    UI_TESTS,
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::{Own, Weak},
    },
    gm::{LossyConvert, color::GRAY_BLUE},
    ui::{MaybeUITest, Setup, UIManager, View, ViewData},
    ui_test::{clear_state, hold_for_human, human_mode, reset_record_probe_count, set_record_canvas},
    window::Window,
};

pub(crate) static TEST_NAME: Mutex<String> = Mutex::new(String::new());

/// Name of the test currently running, for failure attribution from a panic
/// hook where the returned error is not available.
pub fn current_test_name() -> String {
    TEST_NAME.lock().clone()
}

/// Like [`current_test_name`], but never blocks. For the wasm panic beacon,
/// where the panicking thread may hold the lock and a hook that blocks or
/// panics again aborts the instance with the beacon unsent.
#[cfg(target_arch = "wasm32")]
pub(crate) fn current_test_name_nonblocking() -> Option<String> {
    let name = TEST_NAME.try_lock()?;
    if name.is_empty() { None } else { Some(name.clone()) }
}

struct FpsRecord {
    name:    String,
    frames:  u32,
    seconds: f32,
}

static FPS_REPORT: AtomicBool = AtomicBool::new(false);
static FPS_RECORDS: Mutex<Vec<FpsRecord>> = Mutex::new(Vec::new());
static FPS_SPAN: Mutex<Option<(String, u32, Instant)>> = Mutex::new(None);

/// Record fps of every test and print a report at the end of the run.
pub fn enable_fps_report() {
    FPS_REPORT.store(true, Ordering::Relaxed);
}

fn record_test_boundary(new_test: Option<String>) {
    if !FPS_REPORT.load(Ordering::Relaxed) {
        return;
    }

    let frames = from_main(|| Window::current().frame_drawn());
    let now = Instant::now();

    let mut span = FPS_SPAN.lock();

    if let Some((name, start_frames, start_time)) = span.take() {
        FPS_RECORDS.lock().push(FpsRecord {
            name,
            frames: frames - start_frames,
            seconds: (now - start_time).as_secs_f32(),
        });
    }

    if let Some(name) = new_test {
        *span = Some((name, frames, now));
    }
}

fn print_fps_report() {
    let records = FPS_RECORDS.lock();

    if records.is_empty() {
        return;
    }

    let width = records.iter().map(|r| r.name.len()).max().unwrap_or(0).max(4);

    println!();
    println!("FPS report:");
    println!("{:<width$}  frames     secs    fps", "test");

    for r in records.iter() {
        let frames: f32 = r.frames.lossy_convert();
        let fps = if r.seconds > 0.0 { frames / r.seconds } else { 0.0 };
        println!(
            "{:<width$}  {:>6}  {:>7.2}  {:>5.1}",
            r.name, r.frames, r.seconds, fps
        );
    }
}

pub struct UITest;

impl UITest {
    pub fn start<T: View + Default + 'static>() -> Weak<T> {
        Self::set(T::new(), 600, 600, true, get_test_name::<T>())
    }

    /// A canvas other than the default 600 by 600. It must still fit the
    /// smallest supported screen, which is 640 by 1136 pixels.
    pub fn start_sized<T: View + Default + 'static>(width: u32, height: u32) -> Weak<T> {
        Self::set(T::new(), width, height, true, get_test_name::<T>())
    }

    pub fn reload<T: View + Default + 'static>() -> Weak<T> {
        Self::set(T::new(), 600, 600, false, get_test_name::<T>())
    }

    /// Rebuild the view on a different canvas without starting a new test.
    pub fn reload_sized<T: View + Default + 'static>(width: u32, height: u32) -> Weak<T> {
        Self::set(T::new(), width, height, false, get_test_name::<T>())
    }

    pub fn set<T: View + 'static>(
        view: Own<T>,
        width: u32,
        height: u32,
        test_start: bool,
        new_test_name: String,
    ) -> Weak<T> {
        let weak = view.weak();
        Self::install_root(view, width, height, test_start, new_test_name);
        weak
    }

    /// Start a test whose root is not the test view itself, see
    /// [`ViewTest::make_root`](crate::ui::ViewTest::make_root).
    pub fn set_root(view: Own<dyn View>, width: u32, height: u32, new_test_name: String) {
        Self::install_root(view, width, height, true, new_test_name);
    }

    fn install_root(view: Own<dyn View>, width: u32, height: u32, test_start: bool, new_test_name: String) {
        if test_start {
            let test_name = TEST_NAME.lock().clone();

            if !test_name.is_empty() {
                info!("{test_name}: OK");
                hold_for_human();
            }

            // Info, not debug. Android's logger filters debug out, and these
            // two lines are what names a hung test in logcat.
            info!("{new_test_name}: Started");

            reset_record_probe_count();

            if human_mode() {
                Window::set_title_prefix(new_test_name.clone());
            }

            record_test_boundary(Some(new_test_name.clone()));
        }

        TEST_NAME.lock().clone_from(&new_test_name);

        set_record_canvas(width, height);

        clear_state();

        wait_for_next_frame();

        from_main(move || {
            let mut root = UIManager::root_view();
            root.clear_root();
            root.reset_background();
            root.set_test_canvas((width, height).into());
            UIManager::set_clear_color(GRAY_BLUE);
            // A level is not in the view tree, so one started by a test
            // would keep drawing under every test after it.
            #[cfg(feature = "level")]
            crate::level::LevelManager::stop_level();
            #[cfg(feature = "scene")]
            crate::scene::SceneManager::stop_scene();
            root.add_subview_to_root(view).place().back();

            trace!("{width} - {height}");
        });
    }

    /// Installs a view over the whole window with nothing else around it,
    /// no canvas, no test name, no holds. What `--present` shows.
    pub fn present_root(view: Own<dyn View>) {
        from_main(move || {
            let mut root = UIManager::root_view();
            root.clear_root();
            root.reset_background();
            root.clear_test_canvas();
            UIManager::set_clear_color(GRAY_BLUE);
            root.add_subview_to_root(view).place().back();
        });
    }

    pub fn finish() {
        let test_name = TEST_NAME.lock().clone();

        if !test_name.is_empty() {
            info!("{test_name}: OK");
            hold_for_human();
        }

        TEST_NAME.lock().clear();

        record_test_boundary(None);
        print_fps_report();
    }
}

/// One registered UI test.
#[derive(Clone, Copy)]
pub struct UITestEntry {
    pub run:     fn() -> anyhow::Result<()>,
    /// Builds the view full screen for a human to play with, never runs
    /// `perform_test`.
    pub present: fn(),
    /// Source file of the `impl ViewTest` that declared it.
    pub file:    &'static str,
}

/// Registers `T` if, and only if, it implements [`ViewTest`].
///
/// Called from a ctor that `#[view]` puts on every view, so `impl ViewTest for
/// X` is the whole declaration of a test. There is no attribute to forget and
/// no second list to update, which is what stops a test from quietly ceasing to
/// exist.
pub fn register_if_test<T: View + Default + 'static>(file: &'static str) {
    let (Some(run), Some(present)) = (
        <T as MaybeUITest>::__ui_test(),
        <T as MaybeUITest>::__ui_present(),
    ) else {
        return;
    };

    let name = get_test_name::<T>();

    assert!(
        UI_TESTS
            .lock()
            .insert(name.clone(), UITestEntry { run, present, file })
            .is_none(),
        "Duplicate ui test: {name}. The registry keys on the type name alone, so \
         two test views sharing one name collide even from different crates.",
    );
}

/// The one rule turning a view's ident into the name its test answers to.
///
/// `ScrollViewTest` becomes `Scroll view test`. Already spaced input is
/// returned unchanged, so `--test-name` takes either spelling and callers never
/// have to know which one they hold.
pub fn spaced_test_name(ident: &str) -> String {
    ident
        .chars()
        .enumerate()
        .map(|(i, c)| {
            if i > 0 && c.is_uppercase() {
                format!(" {}", c.to_ascii_lowercase())
            } else {
                c.to_string()
            }
        })
        .collect()
}

/// The name a view test answers to.
///
/// `#[view]` registers under this and `UITest::start` reports under it, so
/// the name printed by a failing test is always a name `--test-name` accepts.
/// Anything that derives one of the two on its own drifts from the other.
pub fn get_test_name<T>() -> String {
    let input = type_name::<T>();

    let last_part = input.split("::").last().unwrap();

    spaced_test_name(last_part)
}
