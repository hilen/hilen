use anyhow::{Result, ensure};
use hilen::{
    OnceEvent,
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        BLACK, Label, ModalView, NamedKey, Setup, Size, TextField, UIColor, ViewData, ViewTest, ViewTouch,
        WHITE, view,
    },
    ui_test::{checkpoint, inject_keys, inject_named_key},
};

/// A form with a name field. Escape while typing ends the editing and
/// leaves the form open, the next Escape cancels it.
#[view]
struct NameForm {
    event: OnceEvent<Option<String>>,

    #[init]
    title: Label,
    name:  TextField,
}

impl Setup for NameForm {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE).set_corner_radius(12);
        self.title.set_text("form with a text field").set_text_size(20);
        self.title.place().lrt(10).h(40);
        self.name.set_text_size(20);
        self.name.place().lr(10).t(70).h(44);
    }
}

impl ModalView<(), Option<String>> for NameForm {
    fn modal_event(&self) -> &OnceEvent<Option<String>> {
        &self.event
    }

    fn modal_size() -> Size {
        (320, 160).into()
    }

    fn modal_scrim_color() -> UIColor {
        BLACK.with_alpha(0.3).into()
    }

    fn modal_cancel(self: Weak<Self>) -> Option<Option<String>> {
        Some(None)
    }
}

#[view]
struct ModalEscapeTextField {}

impl Setup for ModalEscapeTextField {
    fn setup(self: Weak<Self>) {}
}

impl ViewTest for ModalEscapeTextField {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        let form = from_main(NameForm::prepare_modally);
        wait_for_next_frame();
        from_main(move || form.name.focus());
        inject_keys("Ann");
        checkpoint("typing Ann into the field of the form")?;

        inject_named_key(NamedKey::Escape);
        wait_for_next_frame();
        ensure!(
            from_main(move || form.is_ok()),
            "the first Escape closed the form"
        );
        ensure!(
            from_main(move || !form.name.is_selected()),
            "the first Escape did not end the editing"
        );
        checkpoint("the first Escape ended the editing, the form is still open")?;

        inject_named_key(NamedKey::Escape);
        wait_for_next_frame();
        ensure!(
            from_main(move || form.is_null()),
            "the second Escape left the form open"
        );
        checkpoint("the second Escape closed the form")?;

        Ok(())
    }
}
