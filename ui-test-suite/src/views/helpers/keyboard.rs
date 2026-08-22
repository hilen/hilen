use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{KeyboardView, Setup, ViewData, ViewTest, view},
};
use log::debug;

#[view]
struct KeyboardViewTest {
    #[init]
    keyboard: KeyboardView,
}

impl Setup for KeyboardViewTest {
    fn setup(self: Weak<Self>) {
        self.keyboard.place().back();
    }
}

impl ViewTest for KeyboardViewTest {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        //  record_ui_test().await;

        debug!("Keyboard view: OK");

        Ok(())
    }
}
