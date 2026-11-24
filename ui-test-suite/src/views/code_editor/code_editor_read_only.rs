use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{CodeEditor, CodePos, Label, NamedKey, Setup, ViewData, ViewTest, WHITE, view},
    ui_test::{checkpoint, inject_keys, inject_named_key},
};

use super::common::{SCAD, click_at, color_all, command, drag_over, status_style};

/// The line with `size = [40, 30, 10];`.
const LINE: usize = 1;

/// A code editor that can only be read. A press puts the caret, the arrows
/// move it and a drag selects, but no key changes the text: not a letter,
/// not Enter, Tab or a delete key, not cut, and undo has nothing to take
/// back. The switch off again lets the same keys through.
#[view]
struct CodeEditorReadOnly {
    #[init]
    editor: CodeEditor,
    status: Label,
}

impl Setup for CodeEditorReadOnly {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);
        self.editor.place().t(10).lr(10).h(380);
        self.editor.set_text(SCAD);
        color_all(self.editor);
        self.editor.set_read_only(true);

        status_style(self.status);
        self.status.place().t(400).lr(10).b(10);
    }
}

/// The text of the editor, its cursor and whether something is selected.
fn state(view: Weak<CodeEditorReadOnly>) -> (String, CodePos, bool) {
    from_main(move || {
        (
            view.editor.text(),
            view.editor.cursor(),
            view.editor.selection().is_some(),
        )
    })
}

fn say(view: Weak<CodeEditorReadOnly>, text: &'static str) {
    from_main(move || {
        view.status.set_text(text);
    });
}

impl ViewTest for CodeEditorReadOnly {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        ensure!(
            from_main(move || view.editor.is_read_only()),
            "the editor is not read only"
        );

        click_at(view.editor, CodePos::new(LINE, 4));
        inject_keys("x");
        inject_named_key(NamedKey::Enter);
        inject_named_key(NamedKey::Tab);
        inject_named_key(NamedKey::Backspace);
        inject_named_key(NamedKey::Delete);
        say(
            view,
            "read only: a press after size put the caret there, then x, Enter, Tab, Backspace and \
             Delete changed nothing",
        );
        let (text, cursor, _) = state(view);
        ensure!(text == SCAD, "a key changed the text:\n{text}");
        ensure!(cursor == CodePos::new(LINE, 4), "the caret is at {cursor:?}");
        checkpoint("read only, the caret after size, the text as it was")?;

        inject_named_key(NamedKey::ArrowRight);
        let (_, cursor, _) = state(view);
        ensure!(
            cursor == CodePos::new(LINE, 5),
            "the right arrow put the caret at {cursor:?}"
        );

        drag_over(view.editor, CodePos::new(LINE, 0), CodePos::new(LINE, 4));
        command('x');
        inject_keys("y");
        command('z');
        say(
            view,
            "read only: size is selected by a drag, then cut, y and undo changed nothing",
        );
        let (text, _, selected) = state(view);
        ensure!(text == SCAD, "cut, a letter or undo changed the text:\n{text}");
        ensure!(selected, "the selection is gone");
        ensure!(
            !from_main(move || view.editor.can_undo()),
            "there is something to undo"
        );
        checkpoint("read only, size is selected, the text as it was")?;

        from_main(move || {
            view.editor.set_read_only(false);
        });
        inject_keys("y");
        say(view, "the switch is off: y took the place of the selected size");
        let (text, cursor, _) = state(view);
        ensure!(
            text == SCAD.replacen("size", "y", 1),
            "the letter did not replace the selection:\n{text}"
        );
        ensure!(cursor == CodePos::new(LINE, 1), "the caret is at {cursor:?}");
        checkpoint("the switch is off, line 2 starts with y")?;
        Ok(())
    }
}
