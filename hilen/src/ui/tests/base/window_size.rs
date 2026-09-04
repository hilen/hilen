use anyhow::{Result, ensure};

use crate::{
    deps::{hreads::from_main, refs::Weak},
    gm::{LossyConvert, flat::Size},
    ui::{Setup, ViewTest, view},
    window::{INITIAL_FIT, Window},
};

/// A fresh desktop window is `App::initial_size` in physical pixels, the
/// same size the headless surface has, or as much of it as the display
/// fits. The placement that opens it is in logical points, and reading
/// the size as points once opened a window twice as big on a 2x display.
#[view]
struct WindowSize {}

impl Setup for WindowSize {
    fn setup(self: Weak<Self>) {}
}

impl ViewTest for WindowSize {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        let (inner, wanted) = from_main(|| {
            let initial = crate::app::app().initial_size();
            let display = Window::winit_window().and_then(winit::window::Window::primary_monitor);
            let wanted = match display {
                Some(monitor) => {
                    let fit: f32 = INITIAL_FIT.lossy_convert();
                    let display = monitor.size();
                    let width: f32 = display.width.lossy_convert();
                    let height: f32 = display.height.lossy_convert();
                    Size::new(initial.width.min(width * fit), initial.height.min(height * fit))
                }
                None => initial,
            };
            (Window::inner_size(), wanted)
        });

        // A logical size rounds to whole physical pixels on its way back.
        ensure!(
            (inner.width - wanted.width).abs() <= 1.0 && (inner.height - wanted.height).abs() <= 1.0,
            "the window is {inner:?}, wanted {wanted:?}"
        );

        Ok(())
    }
}
