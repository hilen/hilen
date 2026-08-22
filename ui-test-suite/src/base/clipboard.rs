use anyhow::Result;
use hilen::{
    dispatch::from_main,
    link_button,
    refs::Weak,
    system::Clipboard,
    ui::{Button, Label, Setup, ViewFrame, ViewTest, view},
    ui_test::inject_touches,
};

const COPIED_TEXT: &str = "hilen clipboard test";

#[view]
struct ClipboardTest {
    #[init]
    button: Button,
    label:  Label,
}

impl ClipboardTest {
    fn copy_tapped(self: Weak<Self>) -> Result<()> {
        Clipboard::set_text(COPIED_TEXT)?;
        self.label.set_text("copied");
        Ok(())
    }
}

impl Setup for ClipboardTest {
    fn setup(self: Weak<Self>) {
        self.button.set_frame((50, 50, 100, 100));
        self.button.set_text("Copy");

        self.label.set_frame((200, 50, 100, 100));
        self.label.set_text("Label");

        link_button!(self, button, copy_tapped);
    }
}

impl ViewTest for ClipboardTest {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // A headless CI box runs with no display server and has no clipboard
        // service at all, so the tap below could never flip the label.
        if from_main(|| Clipboard::set_text("probe")).is_err() {
            println!("Clipboard test skipped: no clipboard service");
            return Ok(());
        }

        inject_touches(
            "
                100  100  b
                100  100  e
            ",
        );

        let label = from_main(move || view.label.text.clone());
        assert_eq!(label, "copied");

        // The browser clipboard reads back only through an async permission
        // prompt, so the wasm lane verifies just the write path above.
        #[cfg(not_wasm)]
        assert_eq!(from_main(Clipboard::get_text)?, COPIED_TEXT);

        Ok(())
    }
}
