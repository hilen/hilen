use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{CodeEditor, CodePos, Label, Modifiers, NamedKey, Setup, ViewData, ViewTest, WHITE, view},
    ui_test::{checkpoint, inject_keys, inject_modifiers, inject_named_key},
};

use super::common::{click_at, quoted, status_style};

/// Line 3 holds 4 spaces and `fn`, the word a completion takes the place of.
const CODE: &str = "struct P;\n\nimpl Add for P {\n    fn\n}";

/// A snippet in a code editor, the way a completion of a language server
/// puts one in. `fn` gives way to the body of a method with 2 places:
/// the name of its parameter is selected first, typed `x` takes its
/// place, Tab selects the type, Shift Tab goes back to the name, Tab
/// twice ends at the empty line in the body, with the indent of the
/// method. A second snippet ends on Escape, and Tab then indents again.
/// The status label shows the lines, each in quotes.
#[view]
struct CodeEditorSnippet {
    #[init]
    editor: CodeEditor,
    status: Label,
}

impl Setup for CodeEditorSnippet {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);
        self.editor.place().t(10).lr(10).h(300);
        self.editor.set_text(CODE);
        status_style(self.status);
        self.status.place().t(320).lr(10).b(10);
    }
}

impl CodeEditorSnippet {
    /// The lines, the selected text and whether the snippet is on, the
    /// lines also on the status label.
    fn state(self: Weak<Self>, step: &'static str) -> (Vec<String>, String, bool) {
        wait_for_next_frame();
        from_main(move || {
            let lines = self.editor.lines().to_vec();
            self.status.set_text(format!("{step}\n{}", quoted(&lines)));
            (lines, self.editor.selected_text(), self.editor.snippet_active())
        })
    }
}

impl ViewTest for CodeEditorSnippet {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let mut editor = view.editor;
        click_at(editor, CodePos::new(3, 6));
        from_main(move || {
            editor.insert_snippet(
                CodePos::new(3, 4),
                CodePos::new(3, 6),
                "fn add(${1:rhs}: ${2:Self}) -> Self {\n    $0\n}",
            );
        });
        let (lines, selected, active) = view.state("the snippet is in, rhs is selected");
        ensure!(
            lines[3..6] == ["    fn add(rhs: Self) -> Self {", "        ", "    }"],
            "the snippet: {lines:?}"
        );
        ensure!(selected == "rhs" && active, "first place: {selected:?} {active}");
        checkpoint("the body is in, with the indent of the method, rhs is selected")?;

        inject_keys("x");
        inject_named_key(NamedKey::Tab);
        let (lines, selected, _) = view.state("x typed over rhs, Tab selected Self");
        ensure!(lines[3] == "    fn add(x: Self) -> Self {", "after x: {lines:?}");
        ensure!(selected == "Self", "the second place: {selected:?}");
        checkpoint("x took the place of rhs, Tab selected the type")?;

        inject_modifiers(Modifiers::SHIFT);
        inject_named_key(NamedKey::Tab);
        inject_modifiers(Modifiers::empty());
        let (_, selected, _) = view.state("Shift Tab went back to the name");
        ensure!(selected == "x", "back at the first place: {selected:?}");
        checkpoint("Shift Tab selected the name again")?;

        inject_named_key(NamedKey::Tab);
        inject_named_key(NamedKey::Tab);
        let (_, selected, active) = view.state("Tab twice: the cursor is in the body, the snippet ended");
        let cursor = from_main(move || view.editor.cursor());
        ensure!(cursor == CodePos::new(4, 8), "the end of the snippet: {cursor:?}");
        ensure!(
            selected.is_empty() && !active,
            "after the end: {selected:?} {active}"
        );
        checkpoint("the cursor is on the empty line of the body, the snippet ended")?;

        from_main(move || {
            editor.insert_snippet(CodePos::new(4, 8), CodePos::new(4, 8), "${1:a}$0");
        });
        inject_named_key(NamedKey::Escape);
        let (_, _, active) = view.state("a second snippet, Escape ended it");
        ensure!(!active, "Escape left the snippet on");
        inject_named_key(NamedKey::Tab);
        let (lines, _, _) = view.state("Tab after the end indents again");
        ensure!(lines[4] == "        a   ", "Tab after Escape: {lines:?}");
        checkpoint("Escape ended the second snippet, Tab indents again")?;
        Ok(())
    }
}
