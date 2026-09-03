use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Label, Setup, ViewData, ViewTest, view},
};

/// True when the browser default survived the dispatch. `dispatchEvent`
/// returns false only when a listener called `preventDefault`, so a plain
/// key reads false through winit's canvas handler and a reload shortcut
/// must read true.
fn default_survives(ctrl: bool, meta: bool, shift: bool, code: &str, key: &str) -> bool {
    let init = web_sys::KeyboardEventInit::new();
    init.set_bubbles(true);
    init.set_cancelable(true);
    init.set_ctrl_key(ctrl);
    init.set_meta_key(meta);
    init.set_shift_key(shift);
    init.set_code(code);
    init.set_key(key);

    let event = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
        .expect("Failed to build a keyboard event");

    web_sys::window()
        .expect("Failed to get browser window")
        .document()
        .expect("Failed to get browser document")
        .query_selector("canvas")
        .expect("Failed to query the canvas")
        .expect("No canvas on the page")
        .dispatch_event(&event)
        .expect("Failed to dispatch the keyboard event")
}

#[view]
struct ReloadShortcutsTest {
    #[init]
    label: Label,
}

impl Setup for ReloadShortcutsTest {
    fn setup(self: Weak<Self>) {
        self.label.set_text("Reload shortcuts");
        self.label.place().center().size(500, 60);
    }
}

impl ViewTest for ReloadShortcutsTest {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        // A plain key proves the chain is live, winit still cancels it.
        assert!(!from_main(|| default_survives(false, false, false, "KeyR", "r")));

        assert!(from_main(|| default_survives(false, true, false, "KeyR", "r")));
        assert!(from_main(|| default_survives(false, true, true, "KeyR", "R")));
        assert!(from_main(|| default_survives(true, false, false, "KeyR", "r")));
        assert!(from_main(|| default_survives(true, false, true, "KeyR", "R")));
        assert!(from_main(|| default_survives(false, false, false, "F5", "F5")));

        Ok(())
    }
}
