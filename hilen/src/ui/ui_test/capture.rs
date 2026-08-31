use std::{
    fs::create_dir_all,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

use anyhow::Result;
use parking_lot::Mutex;

use crate::{AppRunner, ui_test::TEST_NAME, window::Screenshot};

static SCREENSHOT_OUTPUT: Mutex<Option<PathBuf>> = Mutex::new(None);
static SCREENSHOT_CAPTURED: Mutex<bool> = Mutex::new(false);

static SHOTS_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);
static SHOT_TEST: Mutex<String> = Mutex::new(String::new());
static SHOT_INDEX: AtomicUsize = AtomicUsize::new(0);

/// Save screenshots captured by a UI test to this path.
pub fn enable_screenshot_capture(path: PathBuf) {
    *SCREENSHOT_OUTPUT.lock() = Some(path);
    *SCREENSHOT_CAPTURED.lock() = false;
}

/// Capture the current UI test frame and save it when screenshot capture is
/// enabled.
pub fn capture_screenshot() -> Result<Screenshot> {
    let screenshot = AppRunner::take_screenshot()?;

    if let Some(path) = SCREENSHOT_OUTPUT.lock().as_ref() {
        screenshot.save(path)?;
        *SCREENSHOT_CAPTURED.lock() = true;
        println!("Screenshot: {}", path.display());
    }

    Ok(screenshot)
}

/// Capture the final frame when the test did not choose an earlier capture
/// point.
pub fn capture_requested_screenshot() -> Result<()> {
    if SCREENSHOT_OUTPUT.lock().is_some() && !*SCREENSHOT_CAPTURED.lock() {
        capture_screenshot()?;
    }

    Ok(())
}

/// Save a clean frame into `dir` at every `check_colors` and every
/// `checkpoint`, so a headless run leaves every verified state on disk.
pub fn enable_shots(dir: PathBuf) -> Result<()> {
    create_dir_all(&dir)?;
    *SHOTS_DIR.lock() = Some(dir);
    Ok(())
}

pub fn shots_enabled() -> bool {
    SHOTS_DIR.lock().is_some()
}

/// Saves the current frame as `<test>-<NN>-<label>.png` in the shots
/// dir. `NN` counts up per test, so the files sort in run order. Does
/// nothing when the run collects no shots.
pub(crate) fn save_shot(label: &str) -> Result<()> {
    let Some(dir) = SHOTS_DIR.lock().clone() else {
        return Ok(());
    };

    let test_name = TEST_NAME.lock().clone();
    let index = next_shot_index(&test_name);
    let path = dir.join(format!("{}-{index:02}-{}.png", slug(&test_name), slug(label)));

    AppRunner::take_screenshot()?.save(&path)?;
    println!("Shot: {}", path.display());

    Ok(())
}

fn next_shot_index(test_name: &str) -> usize {
    let mut last = SHOT_TEST.lock();

    if *last != test_name {
        test_name.clone_into(&mut last);
        SHOT_INDEX.store(0, Ordering::Relaxed);
    }

    SHOT_INDEX.fetch_add(1, Ordering::Relaxed) + 1
}

fn slug(text: &str) -> String {
    let mut slug = String::new();

    for char in text.chars() {
        if char.is_ascii_alphanumeric() {
            slug.push(char.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }

    slug.trim_matches('-').to_string()
}
