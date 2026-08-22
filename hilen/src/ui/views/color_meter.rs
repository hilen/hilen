use ui_proc::view;

use crate::{
    AppRunner,
    deps::refs::Weak,
    ui::{Setup, UIEvents, ViewCallbacks, ViewData},
    window::Screenshot,
};

#[view]
pub struct ColorMeter {
    screenshot: Screenshot,
}

impl Setup for ColorMeter {
    fn setup(self: Weak<Self>) {
        self.update_screenshot();
        UIEvents::size_changed().sub(self, move || self.update_screenshot());
    }
}

impl ViewCallbacks for ColorMeter {
    fn update(&mut self) {
        let pos = AppRunner::cursor_position();

        if pos.is_negative() {
            return;
        }

        self.set_color(self.screenshot.get_pixel(pos));
    }
}

impl ColorMeter {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn update_screenshot(mut self: Weak<Self>) {
        crate::deps::hreads::spawn(async move {
            let Some(screenshot) = AppRunner::take_screenshot().ok() else {
                return;
            };

            crate::deps::hreads::on_main(move || {
                if self.is_null() {
                    return;
                }

                self.screenshot = screenshot;
            });
        });
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn update_screenshot(self: Weak<Self>) {}
}
