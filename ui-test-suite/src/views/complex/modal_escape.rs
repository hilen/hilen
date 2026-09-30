use anyhow::{Result, ensure};
use hilen::{
    OnceEvent,
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        BLACK, Button, Color, Label, ModalView, NamedKey, Question, Setup, Size, UIColor, ViewData, ViewTest,
        WHITE, view,
    },
    ui_test::{checkpoint, inject_named_key},
};
use parking_lot::Mutex;

/// Every result a modal handed back, in order, shown under the modals.
static LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn log(entry: impl Into<String>) {
    LOG.lock().push(entry.into());
}

fn logged() -> String {
    LOG.lock().join(", ")
}

/// A form whose cancel gives `None`, the value Escape ends it with.
#[view]
struct EscapeForm {
    event: OnceEvent<Option<String>>,

    #[init]
    title:  Label,
    cancel: Button,
    save:   Button,
}

impl Setup for EscapeForm {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE).set_corner_radius(12);
        self.title.set_text("form, Escape cancels").set_text_size(20);
        self.title.place().lrt(10).h(40);
        self.cancel.set_text("Cancel").set_text_size(20);
        self.cancel.place().bl(10).size(120, 40);
        self.cancel.on_tap(move || self.hide_modal(None));
        self.save.set_text("Save").set_text_size(20);
        self.save.place().br(10).size(120, 40);
        self.save.on_tap(move || self.hide_modal(Some("saved".into())));
    }
}

impl ModalView<(), Option<String>> for EscapeForm {
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

/// A modal that keeps the default, no Escape.
#[view]
struct EscapeOff {
    event: OnceEvent,

    #[init]
    title: Label,
}

impl Setup for EscapeOff {
    fn setup(self: Weak<Self>) {
        self.set_color(Color::hex("#f0d060")).set_corner_radius(12);
        self.title.set_text("no cancel, Escape does nothing").set_text_size(20);
        self.title.place().back();
    }
}

impl ModalView for EscapeOff {
    fn modal_event(&self) -> &OnceEvent<()> {
        &self.event
    }

    fn modal_size() -> Size {
        (360, 120).into()
    }
}

#[view]
struct ModalEscape {
    #[init]
    results: Label,
}

impl Setup for ModalEscape {
    fn setup(self: Weak<Self>) {
        self.results.set_text_size(20).set_multiline(true);
        self.results.place().lrb(10).h(80);
    }
}

impl ModalEscape {
    fn show_log(self: Weak<Self>) {
        from_main(move || {
            self.results.set_text(format!("results: {}", logged()));
        });
        wait_for_next_frame();
    }

    fn open_form() -> Weak<EscapeForm> {
        let form = from_main(|| {
            let form = EscapeForm::prepare_modally();
            form.modal_event().val(|result: Option<String>| {
                log(format!("form {}", result.unwrap_or_else(|| "cancel".into())));
            });
            form
        });
        wait_for_next_frame();
        form
    }

    fn escape(self: Weak<Self>) {
        inject_named_key(NamedKey::Escape);
        wait_for_next_frame();
        self.show_log();
    }
}

impl ViewTest for ModalEscape {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        LOG.lock().clear();
        view.show_log();

        let form = Self::open_form();
        checkpoint("the form is open")?;
        view.escape();
        ensure!(logged() == "form cancel", "Escape on the form: {}", logged());
        ensure!(from_main(move || form.is_null()), "the form is still open");
        checkpoint("Escape closed the form, results show form cancel")?;

        let form = Self::open_form();
        from_main(|| {
            Question::ask("Question over the form").callback(|yes| log(format!("question {yes}")));
        });
        wait_for_next_frame();
        checkpoint("a question over the form")?;
        view.escape();
        ensure!(
            logged() == "form cancel, question false",
            "Escape on the question: {}",
            logged()
        );
        ensure!(
            from_main(move || form.is_ok()),
            "Escape closed the form under the question"
        );
        checkpoint("Escape answered the question with no, the form is still open")?;
        view.escape();
        ensure!(
            logged() == "form cancel, question false, form cancel",
            "second Escape: {}",
            logged()
        );
        checkpoint("the second Escape closed the form")?;

        let off = from_main(EscapeOff::prepare_modally);
        wait_for_next_frame();
        view.escape();
        ensure!(
            from_main(move || off.is_ok()),
            "Escape closed a modal with no cancel"
        );
        checkpoint("the yellow modal has no cancel and stays open after Escape")?;
        from_main(move || off.hide_modal(()));
        wait_for_next_frame();

        Ok(())
    }
}
