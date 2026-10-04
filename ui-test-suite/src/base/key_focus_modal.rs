use anyhow::{Result, ensure};
use hilen::{
    OnceEvent,
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        BLACK, Button, Focus, Label, ModalView, NamedKey, Setup, Size, UIColor, View, ViewData, ViewFrame,
        ViewTest, WHITE, view,
    },
    ui_test::{checkpoint, inject_named_key},
};

#[view]
struct FocusForm {
    event: OnceEvent<Option<String>>,

    #[init]
    title:  Label,
    cancel: Button,
    save:   Button,
}

impl Setup for FocusForm {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE).set_corner_radius(12);
        self.title.set_text("modal, the ring stays inside").set_text_size(20);
        self.title.place().lrt(10).h(40);
        self.cancel.set_text("Cancel").set_text_size(20);
        self.cancel.place().bl(16).size(120, 44);
        self.cancel.on_tap(move || self.hide_modal(None));
        self.save.set_text("Save").set_text_size(20);
        self.save.place().br(16).size(120, 44);
        self.save.on_tap(move || self.hide_modal(Some("saved".into())));
    }
}

impl ModalView<(), Option<String>> for FocusForm {
    fn modal_event(&self) -> &OnceEvent<Option<String>> {
        &self.event
    }

    fn modal_size() -> Size {
        (340, 170).into()
    }

    fn modal_scrim_color() -> UIColor {
        BLACK.with_alpha(0.3).into()
    }

    fn modal_cancel(self: Weak<Self>) -> Option<Option<String>> {
        Some(None)
    }
}

/// A modal that opens takes the ring, the arrows never reach a view under
/// it, and the ring goes back when the modal closes.
#[view]
struct KeyFocusModal {
    result: String,
    form:   Weak<FocusForm>,

    #[init]
    open:  Button,
    other: Button,
    state: Label,
}

impl KeyFocusModal {
    fn focused(self: Weak<Self>) -> String {
        from_main(move || {
            let focused = Focus::focused();
            let is = |view: &dyn View| focused.is_ok() && focused.raw() == view.weak_view().raw();
            let name = if is(&*self.open) {
                "open"
            } else if is(&*self.other) {
                "other"
            } else if self.form.is_ok() && is(&*self.form.cancel) {
                "cancel"
            } else if self.form.is_ok() && is(&*self.form.save) {
                "save"
            } else {
                "none"
            };
            self.state.set_text(format!("ring: {name}, result: {}", self.result));
            name.to_string()
        })
    }

    fn press(self: Weak<Self>, key: NamedKey, expected: &str) -> Result<()> {
        inject_named_key(key);
        wait_for_next_frame();
        wait_for_next_frame();
        let focused = self.focused();
        ensure!(
            focused == expected,
            "after {key:?} the ring is on {focused}, expected {expected}"
        );
        wait_for_next_frame();
        Ok(())
    }
}

impl Setup for KeyFocusModal {
    fn setup(mut self: Weak<Self>) {
        self.result = "nothing".into();

        self.open.set_text("Open the modal").set_text_size(20);
        self.open.set_color("#cfd8e3").set_corner_radius(10);
        self.open.set_frame((40, 60, 240, 60));
        self.open.on_tap(move || {
            let form = FocusForm::prepare_modally();
            form.modal_event().val(move |result: Option<String>| {
                self.result = result.unwrap_or_else(|| "cancel".into());
            });
            self.form = form;
        });

        self.other.set_text("under the modal").set_text_size(20);
        self.other.set_color("#cfd8e3").set_corner_radius(10);
        self.other.set_frame((320, 60, 240, 60));
        self.other.on_tap(move || {
            self.result = "other".into();
        });

        self.state.set_text_size(20);
        self.state.place().lrb(10).h(40);
    }
}

impl ViewTest for KeyFocusModal {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        view.press(NamedKey::ArrowRight, "open")?;
        checkpoint("the ring is on the open button")?;

        // Enter opens the modal, the ring moves into it by itself.
        view.press(NamedKey::Enter, "cancel")?;
        checkpoint("the modal is open, the ring is on Cancel")?;

        view.press(NamedKey::ArrowRight, "save")?;
        view.press(NamedKey::ArrowUp, "save")?;
        view.press(NamedKey::ArrowRight, "save")?;
        checkpoint("the ring stays in the modal, on Save")?;

        // Escape is the Back key of a remote, it cancels the modal and
        // the ring goes back to the screen under it.
        view.press(NamedKey::Escape, "open")?;
        ensure!(
            from_main(move || view.result.clone()) == "cancel",
            "Escape did not cancel the modal"
        );
        checkpoint("the modal is gone, the ring is back on the open button")?;

        view.press(NamedKey::ArrowRight, "other")?;

        Ok(())
    }
}
