use std::{thread::sleep, time::Duration};

use anyhow::{Result, ensure};
use hilen::{
    OnceEvent,
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        BLACK, CodeEditor, CodePos, Label, ModalView, NamedKey, Setup, Size, TextField, UIColor, UIManager,
        ViewData, ViewTest, WHITE, view,
    },
    ui_test::{checkpoint, inject_keys, inject_named_key, inject_touches, system_input::wait_until},
};

use super::common::{SCAD, click_at, color_all, status_style};

/// The line with `plate();`.
const LINE: usize = 11;

/// A dialog with a field that takes the keys, like the file finder of an
/// app with a code editor. Escape closes it.
#[view]
struct AskName {
    event: OnceEvent<Option<String>>,

    #[init]
    title: Label,
    field: TextField,
}

impl Setup for AskName {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE).set_corner_radius(12);
        self.title.set_text("a dialog, its field has the keys").set_text_size(16);
        self.title.place().lrt(10).h(30);
        self.field.place().t(50).lr(10).h(36);
        self.field.focus();
    }
}

impl ModalView<(), Option<String>> for AskName {
    fn modal_event(&self) -> &OnceEvent<Option<String>> {
        &self.event
    }

    fn modal_size() -> Size {
        (320, 110).into()
    }

    fn modal_scrim_color() -> UIColor {
        BLACK.with_alpha(0.3).into()
    }

    fn modal_cancel(self: Weak<Self>) -> Option<Option<String>> {
        Some(None)
    }
}

/// A code editor gets the keys back when a dialog over it closes. The
/// editor has the keys and a letter goes into the code. A dialog with a
/// field opens, the field takes the keys and the caret of the code is
/// gone. Escape closes the dialog: the caret is back and the next letter
/// goes into the code, with no press on the editor. The same after a press
/// outside a second dialog, which closes it while its field still has the
/// keys. Last an editor that had no keys when a dialog opened has none
/// after it.
#[view]
struct CodeEditorKeysBack {
    closed: usize,
    dialog: Weak<AskName>,

    #[init]
    editor: CodeEditor,
    status: Label,
}

impl Setup for CodeEditorKeysBack {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);
        self.editor.place().t(10).lr(10).h(380);
        self.editor.set_text(SCAD);
        color_all(self.editor);

        status_style(self.status);
        self.status.place().t(400).lr(10).b(10);
    }
}

/// Opens the dialog and waits until its field has the keys.
fn open_dialog(view: Weak<CodeEditorKeysBack>) -> Result<()> {
    from_main(move || {
        let mut view = view;
        let dialog = AskName::prepare_modally_with_input(());
        dialog.modal_event().val(move |_| view.closed += 1);
        view.dialog = dialog;
    });
    wait_until("the field of the dialog has the keys", move || {
        view.dialog.is_ok() && view.dialog.field.is_editing()
    })
}

/// Closes the dialog with Escape and waits until it is gone. The Escape of
/// a field that is edited belongs to the field and ends its editing, so
/// the dialog closes on the second one, with no view that has the keys.
fn close_dialog(view: Weak<CodeEditorKeysBack>) -> Result<()> {
    let closed = from_main(move || view.closed);
    inject_named_key(NamedKey::Escape);
    wait_for_next_frame();
    inject_named_key(NamedKey::Escape);
    wait_until("the dialog closed", move || view.closed == closed + 1)?;
    wait_for_next_frame();
    Ok(())
}

fn say(view: Weak<CodeEditorKeysBack>, text: &'static str) {
    from_main(move || {
        view.status.set_text(text);
    });
}

fn line(view: Weak<CodeEditorKeysBack>) -> String {
    from_main(move || view.editor.lines()[LINE].clone())
}

impl ViewTest for CodeEditorKeysBack {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        click_at(view.editor, CodePos::new(LINE, 2));
        inject_keys("a");
        ensure!(line(view) == "plaate();", "the first letter: {:?}", line(view));

        open_dialog(view)?;
        let editing = from_main(move || view.editor.is_editing());
        ensure!(!editing, "the editor has the keys under a dialog with a field");
        say(
            view,
            "the letter a went into plate in line 12. Then a dialog opened, its field has the keys, the code \
             has no caret",
        );
        wait_for_next_frame();
        checkpoint("a dialog is open over the code, the code has no caret")?;

        close_dialog(view)?;
        wait_until("the editor has the keys again after Escape", move || {
            view.editor.is_editing()
        })?;
        inject_keys("b");
        let state = from_main(move || (view.editor.lines()[LINE].clone(), view.editor.cursor()));
        ensure!(
            state == ("plabate();".to_string(), CodePos::new(LINE, 4)),
            "after the dialog: {state:?}"
        );
        say(
            view,
            "Escape closed the dialog. The caret is back in line 12 and the letter b went in after pla, with \
             no press on the code",
        );
        wait_for_next_frame();
        checkpoint("the dialog is gone, the caret is in the code after plab")?;

        // A press outside the dialog closes it while its field has the keys.
        open_dialog(view)?;
        inject_touches("300 560 b\n300 560 e");
        wait_until("the dialog closed on a press outside it", move || {
            view.closed == 2
        })?;
        wait_until("the editor has the keys again after the press", move || {
            view.editor.is_editing()
        })?;
        inject_keys("c");
        ensure!(
            line(view) == "plabcate();",
            "after the second dialog: {:?}",
            line(view)
        );

        from_main(UIManager::unselect_view);
        open_dialog(view)?;
        close_dialog(view)?;
        // The keys come back a moment after a dialog closed, so none came
        // only when that moment is over.
        sleep(Duration::from_millis(300));
        let editing = from_main(move || view.editor.is_editing());
        ensure!(
            !editing,
            "an editor that had no keys before the dialog has them after it"
        );
        inject_keys("d");
        ensure!(
            line(view) == "plabcate();",
            "a letter with no keys: {:?}",
            line(view)
        );
        Ok(())
    }
}
