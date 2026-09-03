use std::path::Path;

use anyhow::Result;

/// One on screen app window as the window server reports it.
pub struct WindowInfo {
    pub id:    u32,
    pub pid:   i32,
    pub owner: String,
    pub title: String,
}

/// Resolve `query` to exactly one on screen window. The query is the
/// owner app name, matched case insensitively, or a bare window id
/// from an earlier candidate listing. Several matches list the
/// candidates and error so a lazy query never hits the wrong window.
pub fn find_window(query: &str) -> Result<WindowInfo> {
    use anyhow::bail;

    let windows = mac::list_windows();
    let matches: Vec<&WindowInfo> = if let Ok(id) = query.parse::<u32>() {
        windows.iter().filter(|window| window.id == id).collect()
    } else {
        let needle = query.to_lowercase();
        windows
            .iter()
            .filter(|window| window.owner.to_lowercase().contains(&needle))
            .collect()
    };

    match matches.as_slice() {
        [window] => Ok(WindowInfo {
            id:    window.id,
            pid:   window.pid,
            owner: window.owner.clone(),
            title: window.title.clone(),
        }),
        [] => bail!(
            "no on screen window matches {query:?}, windows: {}",
            describe(&windows)
        ),
        candidates => bail!(
            "several windows match {query:?}, pass one id instead: {}",
            describe(candidates.iter().map(|window| &**window))
        ),
    }
}

/// Capture one app window from the screen into a png at its retina
/// backing resolution. The window does not need to be frontmost, only
/// on screen. Needs the one time Screen Recording permission for the
/// terminal running the tool.
pub fn capture(query: &str, out: &Path) -> Result<()> {
    let window = find_window(query)?;
    capture_window(&window, out)
}

/// Capture an already resolved window.
pub fn capture_window(window: &WindowInfo, out: &Path) -> Result<()> {
    use std::process::Command;

    use anyhow::{Context, ensure};

    let status = Command::new("screencapture")
        .arg("-x")
        .arg("-o")
        .arg(format!("-l{}", window.id))
        .arg(out)
        .status()
        .context("running screencapture")?;
    ensure!(
        status.success(),
        "screencapture failed for window {} of {}, is Screen Recording allowed for this terminal?",
        window.id,
        window.owner,
    );
    Ok(())
}

fn describe<'w>(windows: impl IntoIterator<Item = &'w WindowInfo>) -> String {
    let lines: Vec<String> = windows
        .into_iter()
        .map(|window| format!("\n  {} {} {:?}", window.id, window.owner, window.title))
        .collect();
    lines.join("")
}

mod mac {
    use std::ffi::c_void;

    use objc2_core_foundation::{CFDictionary, CFNumber, CFString};
    use objc2_core_graphics::{
        CGWindowListCopyWindowInfo, CGWindowListOption, kCGNullWindowID, kCGWindowLayer, kCGWindowName,
        kCGWindowNumber, kCGWindowOwnerName, kCGWindowOwnerPID,
    };

    use super::WindowInfo;

    /// The normal on screen app windows, layer zero, desktop furniture
    /// excluded.
    pub fn list_windows() -> Vec<WindowInfo> {
        let Some(list) = CGWindowListCopyWindowInfo(
            CGWindowListOption::OptionOnScreenOnly | CGWindowListOption::ExcludeDesktopElements,
            kCGNullWindowID,
        ) else {
            return Vec::new();
        };

        let mut windows = Vec::new();
        for index in 0..list.count() {
            // The window server hands a CFArray of CFDictionary entries.
            let dict = unsafe { &*list.value_at_index(index).cast::<CFDictionary>() };
            if dict_i64(dict, unsafe { kCGWindowLayer }) != Some(0) {
                continue;
            }
            let Some(id) = dict_i64(dict, unsafe { kCGWindowNumber }) else {
                continue;
            };
            let Ok(id) = u32::try_from(id) else { continue };
            let Some(pid) = dict_i64(dict, unsafe { kCGWindowOwnerPID }) else {
                continue;
            };
            let Ok(pid) = i32::try_from(pid) else { continue };
            windows.push(WindowInfo {
                id,
                pid,
                owner: dict_string(dict, unsafe { kCGWindowOwnerName }).unwrap_or_default(),
                title: dict_string(dict, unsafe { kCGWindowName }).unwrap_or_default(),
            });
        }
        windows
    }

    fn dict_value(dict: &CFDictionary, key: &CFString) -> *const c_void {
        unsafe { dict.value(core::ptr::from_ref(key).cast()) }
    }

    fn dict_i64(dict: &CFDictionary, key: &CFString) -> Option<i64> {
        let value = dict_value(dict, key);
        if value.is_null() {
            return None;
        }
        unsafe { &*value.cast::<CFNumber>() }.as_i64()
    }

    fn dict_string(dict: &CFDictionary, key: &CFString) -> Option<String> {
        let value = dict_value(dict, key);
        if value.is_null() {
            return None;
        }
        Some(unsafe { &*value.cast::<CFString>() }.to_string())
    }
}
