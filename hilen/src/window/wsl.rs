//! Running on Windows through WSL. `WSLg` puts Linux windows on the Windows
//! desktop, and two of its behaviors need help. Its Wayland compositor
//! never presents this engine's first frame, so a window never appears,
//! while X11 through Xwayland shows it. And it reports scale 1 for every
//! monitor whatever Windows is set to, so a window comes out at one
//! physical pixel per logical pixel on a scaled display.

use std::{env, fs};

use log::info;

const WESTON_LOG: &str = "/mnt/wslg/weston.log";
const SCALE_VAR: &str = "WINIT_X11_SCALE_FACTOR";

/// Whether this process runs inside WSL. WSL sets the distro name for
/// every process it starts.
pub(crate) fn active() -> bool {
    env::var_os("WSL_DISTRO_NAME").is_some()
}

/// Point winit at X11 with the Windows display scale. Has to run before
/// the event loop exists, winit reads both variables when it starts.
pub(crate) fn prepare() {
    if !active() {
        return;
    }

    let preset = env::var(SCALE_VAR).ok();
    let log = fs::read_to_string(WESTON_LOG).ok();
    let percent = scale_to_set(preset.as_deref(), log.as_deref());

    // SAFETY: startup on the main thread, before the engine spawns any
    // thread, so nothing reads the environment concurrently.
    unsafe {
        env::remove_var("WAYLAND_DISPLAY");
        if let Some(percent) = percent {
            env::set_var(SCALE_VAR, (f64::from(percent) / 100.0).to_string());
        }
    }

    match percent {
        Some(percent) => info!("WSL: X11, scale {percent}% from Windows"),
        None => info!("WSL: X11, scale left to winit"),
    }
}

/// The scale percent to export. None when the user already set one, or
/// the log has none to offer.
fn scale_to_set(preset: Option<&str>, log: Option<&str>) -> Option<u32> {
    if preset.is_some() {
        return None;
    }
    windows_scale(log?)
}

/// The scale percent Windows applies to the primary monitor, from the
/// block the compositor logs when the RDP client reports its monitors:
/// `rdpMonitor[1]: x:0, y:0, width:3840, height:2160, is_primary:1`
/// `rdpMonitor[1]: desktopScaleFactor:150, deviceScaleFactor:140`.
/// The last report wins, the client reports again on every change.
/// Weston echoes the layout it computed in the same shape with every
/// desktop scale at zero, so a zero never counts.
fn windows_scale(log: &str) -> Option<u32> {
    let mut primary = None;
    let mut scale = None;
    for (monitor, rest) in log.lines().filter_map(monitor_line) {
        if rest.contains("is_primary:1") {
            primary = Some(monitor);
        } else if rest.contains("is_primary:0") && primary == Some(monitor) {
            primary = None;
        }
        if primary != Some(monitor) {
            continue;
        }
        if let Some(percent) = desktop_scale(rest).filter(|percent| *percent > 0) {
            scale = Some(percent);
        }
    }
    scale
}

/// The monitor tag and the rest of a `rdpMonitor[N]: ...` line.
fn monitor_line(line: &str) -> Option<(&str, &str)> {
    let start = line.find("rdpMonitor[")?;
    line[start..].split_once(": ")
}

/// The value of `desktopScaleFactor:N` in a monitor line.
fn desktop_scale(rest: &str) -> Option<u32> {
    let (_, value) = rest.split_once("desktopScaleFactor:")?;
    value.split(',').next()?.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOG: &str = r"
[10:31:40.358] disp_monitor_sanity_check_layout:---INPUT---
[10:31:40.358] 	rdpMonitor[0]: x:-1920, y:628, width:1920, height:1080, is_primary:0
[10:31:40.358] 	rdpMonitor[0]: physicalWidth:476, physicalHeight:268, orientation:0
[10:31:40.358] 	rdpMonitor[0]: desktopScaleFactor:100, deviceScaleFactor:100
[10:31:40.358] 	rdpMonitor[0]: scale:1, client scale :1.00
[10:31:40.358] 	rdpMonitor[1]: x:0, y:0, width:3840, height:2160, is_primary:1
[10:31:40.358] 	rdpMonitor[1]: physicalWidth:620, physicalHeight:340, orientation:0
[10:31:40.358] 	rdpMonitor[1]: desktopScaleFactor:150, deviceScaleFactor:140
[10:31:40.358] 	rdpMonitor[1]: scale:1, client scale :1.00
";

    const COMPUTED: &str = r"
[10:31:40.567] disp_monitor_validate_and_compute_layout:---OUTPUT---
[10:31:40.567] 	rdpMonitor[0]: x:-1920, y:628, width:1920, height:1080, is_primary:0
[10:31:40.567] 	rdpMonitor[0]: weston x:0, y:628, width:1920, height:1080
[10:31:40.567] 	rdpMonitor[0]: desktopScaleFactor:0, deviceScaleFactor:100
[10:31:40.567] 	rdpMonitor[0]: scale:1, clientScale:1.00
[10:31:40.567] 	rdpMonitor[1]: x:0, y:0, width:3840, height:2160, is_primary:1
[10:31:40.567] 	rdpMonitor[1]: weston x:1920, y:0, width:3840, height:2160
[10:31:40.567] 	rdpMonitor[1]: desktopScaleFactor:0, deviceScaleFactor:140
[10:31:40.567] 	rdpMonitor[1]: scale:1, clientScale:1.00
";

    #[test]
    fn primary_monitor_scale() {
        assert_eq!(windows_scale(LOG), Some(150));
    }

    const LATER: &str = r"
[11:00:00.000] 	rdpMonitor[0]: x:0, y:0, width:1920, height:1080, is_primary:1
[11:00:00.000] 	rdpMonitor[0]: desktopScaleFactor:125, deviceScaleFactor:120
[11:00:00.000] 	rdpMonitor[1]: x:1920, y:0, width:3840, height:2160, is_primary:0
[11:00:00.000] 	rdpMonitor[1]: desktopScaleFactor:150, deviceScaleFactor:140
";

    #[test]
    fn last_report_wins() {
        assert_eq!(windows_scale(&format!("{LOG}{LATER}")), Some(125));
    }

    #[test]
    fn computed_layout_zero_never_counts() {
        assert_eq!(windows_scale(&format!("{LOG}{COMPUTED}")), Some(150));
        assert_eq!(windows_scale(COMPUTED), None);
    }

    #[test]
    fn no_scale_line() {
        assert_eq!(
            windows_scale("[10:05:41.456] launching weston-desktop-shell\n"),
            None
        );
    }

    #[test]
    fn preset_wins() {
        assert_eq!(scale_to_set(Some("2"), Some(LOG)), None);
        assert_eq!(scale_to_set(None, Some(LOG)), Some(150));
        assert_eq!(scale_to_set(None, None), None);
    }
}
