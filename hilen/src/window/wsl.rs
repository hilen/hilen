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

/// The scale percent Windows applies to the first monitor, from the line
/// the compositor logs when the RDP client reports its monitors:
/// `rdpMonitor[0]: desktopScaleFactor:150, deviceScaleFactor:140`.
/// The last one wins, the client reports again on every change.
fn windows_scale(log: &str) -> Option<u32> {
    log.lines()
        .filter(|line| line.contains("rdpMonitor[0]:"))
        .filter_map(|line| line.split_once("desktopScaleFactor:"))
        .filter_map(|(_, rest)| rest.split(',').next()?.trim().parse().ok())
        .next_back()
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOG: &str = r"
[10:28:54.567] 	rdpMonitor[0]: desktopScaleFactor:150, deviceScaleFactor:140
[10:28:54.567] 	rdpMonitor[0]: scale:1, client scale :1.00
[10:28:54.567] 	rdpMonitor[1]: desktopScaleFactor:100, deviceScaleFactor:100
";

    #[test]
    fn first_monitor_scale() {
        assert_eq!(windows_scale(LOG), Some(150));
    }

    #[test]
    fn last_report_wins() {
        let log =
            format!("{LOG}[11:00:00.000] \trdpMonitor[0]: desktopScaleFactor:125, deviceScaleFactor:120\n");
        assert_eq!(windows_scale(&log), Some(125));
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
