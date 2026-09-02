use serde::{Deserialize, Serialize};

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
    let width = primary.width * 0.8;
    let height = primary.height * 0.8;
    WindowPlacement {
        width,
        height,
        x: primary.x + (primary.width - width) / 2.0,
        y: primary.y + (primary.height - height) / 2.0,
        maximized: false,
        monitor: primary.name.clone(),
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
        use winit::dpi::{LogicalPosition, LogicalSize};

        let Some(window) = self.screen.winit_window() else {
            return;
        };
        let monitors: Vec<MonitorInfo> =
            window.available_monitors().map(|m| MonitorInfo::from_handle(&m)).collect();
        let primary = window.primary_monitor().map(|m| MonitorInfo::from_handle(&m));
        let target = resolve(saved, &monitors, primary.as_ref());

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
    fn json_without_monitor_field_loads() {
        let json = r#"{"width":1300.0,"height":866.0,"x":500.0,"y":153.0,"maximized":false}"#;
        let placement: WindowPlacement = serde_json::from_str(json).unwrap();
        assert_eq!(placement.monitor, None);
    }
}
