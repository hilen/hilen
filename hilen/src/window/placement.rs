use serde::{Deserialize, Serialize};

use crate::gm::flat::Size;

/// The share of a display a fresh window may take. The rest is room for
/// the window frame and the taskbar, which the display size includes.
pub(crate) const INITIAL_FIT: f64 = 0.9;

/// Where a desktop window sits and how big it is, in logical points.
///
/// Logical points travel between monitors with different scale factors,
/// physical pixels do not. A window saved on a 2x display and restored on a
/// 1x one would come back twice as big.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WindowPlacement {
    pub width:     f64,
    pub height:    f64,
    pub x:         f64,
    pub y:         f64,
    pub maximized: bool,
    /// The display the window was on. A saved placement whose monitor is
    /// gone is not applied as is, see `resolve`.
    #[serde(default)]
    pub monitor:   Option<String>,
}

/// One attached display in logical points.
#[derive(Clone, Debug, PartialEq)]
pub struct MonitorInfo {
    pub name:   Option<String>,
    pub x:      f64,
    pub y:      f64,
    pub width:  f64,
    pub height: f64,
}

impl MonitorInfo {
    #[cfg(desktop)]
    fn from_handle(handle: &winit::monitor::MonitorHandle) -> Self {
        let scale = handle.scale_factor();
        let size = handle.size();
        let pos = handle.position();
        Self {
            name:   handle.name(),
            x:      f64::from(pos.x) / scale,
            y:      f64::from(pos.y) / scale,
            width:  f64::from(size.width) / scale,
            height: f64::from(size.height) / scale,
        }
    }
}

/// What to apply on launch for a saved placement. When the saved monitor is
/// still attached the placement comes back as is. When it is gone, the
/// window would land off screen, so it is centered on the primary display
/// at 80 percent of its size instead.
pub fn resolve(
    saved: &WindowPlacement,
    monitors: &[MonitorInfo],
    primary: Option<&MonitorInfo>,
) -> WindowPlacement {
    let present = match &saved.monitor {
        None => true,
        Some(name) => monitors.iter().any(|m| m.name.as_deref() == Some(name)),
    };
    if present {
        return saved.clone();
    }
    let Some(primary) = primary else {
        return WindowPlacement {
            width:     900.0,
            height:    600.0,
            x:         50.0,
            y:         50.0,
            maximized: false,
            monitor:   None,
        };
    };
    centered(primary, primary.width * 0.8, primary.height * 0.8)
}

/// Where a window with nothing saved opens: `size`, in logical points, on
/// the primary display, shrunk to fit it, and centered there. Left to the
/// window manager a window taller than the display it picks opens with its
/// title bar above the screen edge, which `WSLg` does with a 1000 point
/// window on a 1080 pixel display.
pub fn initial(size: Size, primary: Option<&MonitorInfo>) -> WindowPlacement {
    let width = f64::from(size.width);
    let height = f64::from(size.height);
    let Some(primary) = primary else {
        return WindowPlacement {
            width,
            height,
            x: 50.0,
            y: 50.0,
            maximized: false,
            monitor: None,
        };
    };
    centered(
        primary,
        width.min(primary.width * INITIAL_FIT),
        height.min(primary.height * INITIAL_FIT),
    )
}

/// The display a fresh window opens on. The reported primary when the
/// platform names one with a real size. X11 under `WSLg` sets no
/// `RandR` primary and winit answers with a zero sized nameless dummy,
/// so the largest display stands in there.
pub fn primary_or_largest(monitors: &[MonitorInfo], reported: Option<MonitorInfo>) -> Option<MonitorInfo> {
    if let Some(reported) = reported
        && reported.width > 0.0
        && reported.height > 0.0
    {
        return Some(reported);
    }
    monitors
        .iter()
        .max_by(|a, b| (a.width * a.height).total_cmp(&(b.width * b.height)))
        .cloned()
}

fn centered(monitor: &MonitorInfo, width: f64, height: f64) -> WindowPlacement {
    WindowPlacement {
        width,
        height,
        x: monitor.x + (monitor.width - width) / 2.0,
        y: monitor.y + (monitor.height - height) / 2.0,
        maximized: false,
        monitor: monitor.name.clone(),
    }
}

#[cfg(desktop)]
impl super::Window {
    /// The current placement of the real window. `None` when headless.
    pub fn placement() -> Option<WindowPlacement> {
        let window = Self::winit_window()?;
        let scale = window.scale_factor();
        let size = window.inner_size();
        let pos = window.outer_position().unwrap_or_default();
        Some(WindowPlacement {
            width:     f64::from(size.width) / scale,
            height:    f64::from(size.height) / scale,
            x:         f64::from(pos.x) / scale,
            y:         f64::from(pos.y) / scale,
            maximized: window.is_maximized(),
            monitor:   window.current_monitor().and_then(|m| m.name()),
        })
    }

    pub(crate) fn apply_placement(&mut self, saved: &WindowPlacement) {
        let (monitors, primary) = self.monitors();
        let target = resolve(saved, &monitors, primary.as_ref());
        self.place(&target);
    }

    /// The launch placement of an app with nothing saved. Headless has no
    /// display to place on and takes the size as is.
    /// `size` is in physical pixels, the surface the app renders into,
    /// the unit the headless size shares. A placement is in logical
    /// points, so the window scale divides it out first, read as points
    /// a 2x display opens the window twice as big.
    pub(crate) fn apply_initial_size(&mut self, size: Size) {
        use crate::gm::LossyConvert;

        let Some(scale) = self.screen.winit_window().map(winit::window::Window::scale_factor) else {
            self.set_size(size.lossy_convert());
            return;
        };
        let scale: f32 = scale.lossy_convert();
        let logical = Size::new(size.width / scale, size.height / scale);
        let (_, primary) = self.monitors();
        self.place(&initial(logical, primary.as_ref()));
    }

    fn monitors(&self) -> (Vec<MonitorInfo>, Option<MonitorInfo>) {
        let Some(window) = self.screen.winit_window() else {
            return (Vec::new(), None);
        };
        let monitors: Vec<MonitorInfo> =
            window.available_monitors().map(|m| MonitorInfo::from_handle(&m)).collect();
        let reported = window.primary_monitor().map(|m| MonitorInfo::from_handle(&m));
        let primary = primary_or_largest(&monitors, reported);
        (monitors, primary)
    }

    fn place(&mut self, target: &WindowPlacement) {
        use winit::dpi::{LogicalPosition, LogicalSize};

        self.request_inner_size(LogicalSize::new(target.width, target.height));
        let Some(window) = self.screen.winit_window() else {
            return;
        };
        window.set_outer_position(LogicalPosition::new(target.x, target.y));
        if target.maximized {
            window.set_maximized(true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(name: &str, x: f64, width: f64) -> MonitorInfo {
        MonitorInfo {
            name: Some(name.to_string()),
            x,
            y: 0.0,
            width,
            height: 1000.0,
        }
    }

    fn saved(monitor: Option<&str>) -> WindowPlacement {
        WindowPlacement {
            width:     1300.0,
            height:    866.0,
            x:         2000.0,
            y:         153.0,
            maximized: false,
            monitor:   monitor.map(str::to_string),
        }
    }

    #[test]
    fn saved_monitor_still_attached_keeps_placement() {
        let monitors = [
            monitor("Built-in", 0.0, 1512.0),
            monitor("External", 1512.0, 2560.0),
        ];
        let placement = saved(Some("External"));
        assert_eq!(resolve(&placement, &monitors, Some(&monitors[0])), placement);
    }

    #[test]
    fn no_saved_monitor_keeps_placement() {
        let monitors = [monitor("Built-in", 0.0, 1512.0)];
        let placement = saved(None);
        assert_eq!(resolve(&placement, &monitors, Some(&monitors[0])), placement);
    }

    #[test]
    fn missing_monitor_centers_on_primary() {
        let monitors = [monitor("Built-in", 100.0, 1500.0)];
        let placement = saved(Some("External"));
        let target = resolve(&placement, &monitors, Some(&monitors[0]));
        assert_eq!(
            target,
            WindowPlacement {
                width:     1200.0,
                height:    800.0,
                x:         250.0,
                y:         100.0,
                maximized: false,
                monitor:   Some("Built-in".to_string()),
            }
        );
    }

    #[test]
    fn missing_monitor_without_primary_uses_default() {
        let target = resolve(&saved(Some("External")), &[], None);
        assert_eq!(
            (target.width, target.height, target.x, target.y),
            (900.0, 600.0, 50.0, 50.0)
        );
        assert!(!target.maximized);
    }

    #[test]
    fn initial_centers_on_primary() {
        let primary = monitor("Built-in", 100.0, 1500.0);
        let target = initial((1200, 800).into(), Some(&primary));
        assert_eq!(
            target,
            WindowPlacement {
                width:     1200.0,
                height:    800.0,
                x:         250.0,
                y:         100.0,
                maximized: false,
                monitor:   Some("Built-in".to_string()),
            }
        );
    }

    #[test]
    fn initial_shrinks_to_fit_the_display() {
        let primary = MonitorInfo {
            name:   Some("Small".to_string()),
            x:      0.0,
            y:      418.0,
            width:  1280.0,
            height: 720.0,
        };
        let target = initial((1200, 1000).into(), Some(&primary));
        assert_eq!((target.width, target.height), (1152.0, 648.0));
        assert_eq!((target.x, target.y), (64.0, 454.0));
    }

    #[test]
    fn initial_without_display_uses_default() {
        let target = initial((1200, 1000).into(), None);
        assert_eq!(
            (target.width, target.height, target.x, target.y),
            (1200.0, 1000.0, 50.0, 50.0)
        );
    }

    #[test]
    fn dummy_primary_falls_back_to_the_largest_display() {
        let monitors = [monitor("Small", 0.0, 1280.0), monitor("Large", 1280.0, 2560.0)];
        let dummy = MonitorInfo {
            name:   None,
            x:      0.0,
            y:      0.0,
            width:  0.0,
            height: 0.0,
        };
        assert_eq!(
            primary_or_largest(&monitors, Some(dummy)),
            Some(monitors[1].clone())
        );
        assert_eq!(primary_or_largest(&monitors, None), Some(monitors[1].clone()));
        assert_eq!(
            primary_or_largest(&monitors, Some(monitors[0].clone())),
            Some(monitors[0].clone())
        );
        assert_eq!(primary_or_largest(&[], None), None);
    }

    #[test]
    fn json_without_monitor_field_loads() {
        let json = r#"{"width":1300.0,"height":866.0,"x":500.0,"y":153.0,"maximized":false}"#;
        let placement: WindowPlacement = serde_json::from_str(json).unwrap();
        assert_eq!(placement.monitor, None);
    }
}
