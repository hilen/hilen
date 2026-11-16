use std::{thread::sleep, time::Duration};

use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Label, Setup, ViewData, ViewTest, view},
    window::Window,
};

/// Long enough that the next frame syncs the screen reader layer, which
/// keeps in step at most every 300 ms.
const SYNC_DUE: Duration = Duration::from_millis(400);

/// Past the 300 ms the layer waits after a change that came too early.
const CATCH_UP: Duration = Duration::from_millis(600);

/// A change right after the screen reader layer of a page was synced still
/// reaches the layer, from a timer that draws no frame. The timer used to
/// ask for a frame, 1 more frame of a still screen.
#[view]
struct AccessibilityLateChange {
    #[init]
    text: Label,
}

impl Setup for AccessibilityLateChange {
    fn setup(self: Weak<Self>) {
        self.text.set_text("first").set_text_size(24);
        self.text.place().t(20).l(10).size(300, 40);
    }
}

impl ViewTest for AccessibilityLateChange {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        sleep(SYNC_DUE);
        // The frame of this change syncs the layer, the next one comes too
        // early for it.
        from_main(move || {
            view.text.set_text("second");
        });
        from_main(move || {
            view.text.set_text("third");
        });
        let before = Window::render_frame();
        sleep(CATCH_UP);
        let drawn = Window::render_frame() - before;
        let layer = from_main(layer_text);
        ensure!(layer.contains("third"), "the screen reader layer reads {layer:?}");
        // 1 frame is the one the label change of `third` asks for.
        ensure!(
            drawn <= 1,
            "the still screen drew {drawn} frames for the screen reader layer"
        );
        Ok(())
    }
}

fn layer_text() -> String {
    web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id("hilen-accessibility"))
        .and_then(|layer| layer.text_content())
        .unwrap_or_default()
}
