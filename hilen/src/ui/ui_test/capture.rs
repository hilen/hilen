use std::path::PathBuf;

use anyhow::Result;
use parking_lot::Mutex;

use crate::{AppRunner, window::Screenshot};

static SCREENSHOT_OUTPUT: Mutex<Option<PathBuf>> = Mutex::new(None);
static SCREENSHOT_CAPTURED: Mutex<bool> = Mutex::new(false);

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
