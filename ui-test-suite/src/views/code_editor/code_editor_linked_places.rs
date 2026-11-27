use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{CodeEditor, CodePos, Label, NamedKey, Setup, ViewData, ViewTest, WHITE, view},
    ui_test::{checkpoint, inject_keys, inject_named_key},
};

use super::common::{click_at, command, quoted, status_style};

const CODE: &str = "struct Meters(f64);\n\n";

/// A snippet with a place used twice, the way a trait with a type
/// parameter goes in after `impl`: the `T` of `From<T>` and the `T` of
/// `value: T` are 1 place. What is typed in the first copy goes into the
/// second, a Backspace and a picked completion too, and 1 undo takes back
/// a run of typing or of deleting in both. The status label shows the
/// lines, each in quotes.
#[view]
struct CodeEditorLinkedPlaces {
    #[init]
    editor: CodeEditor,
    status: Label,
}

impl Setup for CodeEditorLinkedPlaces {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);
        self.editor.place().t(10).lr(10).h(300);
        self.editor.set_text(CODE);
        status_style(self.status);
        self.status.place().t(320).lr(10).b(10);
    }
}

impl CodeEditorLinkedPlaces {
    /// The 2 lines of the impl header and of `from`, and the selected
    /// text, the lines also on the status label.
    fn state(self: Weak<Self>, step: &'static str) -> (String, String, String) {
        wait_for_next_frame();
        from_main(move || {
            let lines = self.editor.lines().to_vec();
            self.status.set_text(format!("{step}\n{}", quoted(&lines)));
            (lines[2].clone(), lines[3].clone(), self.editor.selected_text())
        })
    }
}

impl ViewTest for CodeEditorLinkedPlaces {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let mut editor = view.editor;
        click_at(editor, CodePos::new(2, 0));
        from_main(move || {
            editor.insert_snippet(
                CodePos::new(2, 0),
                CodePos::new(2, 0),
                "impl From<${1:T}> for ${2:Meters} {\n    fn from(value: ${1:T}) -> Self {\n        $0\n    }\n}",
            );
        });
        let (header, from, selected) = view.state("the impl is in, the first T is selected");
        ensure!(header == "impl From<T> for Meters {", "header: {header:?}");
        ensure!(from == "    fn from(value: T) -> Self {", "from: {from:?}");
        ensure!(selected == "T", "first place: {selected:?}");
        checkpoint("the impl is in, the T of From<T> is selected")?;

        inject_keys("f64");
        let (header, from, _) = view.state("f64 typed: both copies of T hold it");
        ensure!(header == "impl From<f64> for Meters {", "header: {header:?}");
        ensure!(from == "    fn from(value: f64) -> Self {", "from: {from:?}");
        checkpoint("f64 typed in From<T>, the T of value: T is f64 too")?;

        inject_named_key(NamedKey::Backspace);
        let (header, from, _) = view.state("Backspace: both copies lost the 4");
        ensure!(header == "impl From<f6> for Meters {", "header: {header:?}");
        ensure!(from == "    fn from(value: f6) -> Self {", "from: {from:?}");
        checkpoint("Backspace took the 4 out of both copies")?;

        // A completion the user picks, the app puts it in place of the
        // typed f6 the way a code editor app does.
        from_main(move || editor.replace(CodePos::new(2, 10), CodePos::new(2, 12), "f64"));
        let (header, from, _) = view.state("the completion f64 picked: both copies hold it");
        ensure!(header == "impl From<f64> for Meters {", "header: {header:?}");
        ensure!(from == "    fn from(value: f64) -> Self {", "from: {from:?}");
        checkpoint("the picked completion f64 went into both copies")?;

        command('z');
        let (header, from, _) = view.state("Cmd Z: the pick is gone from both copies");
        ensure!(header == "impl From<f6> for Meters {", "header: {header:?}");
        ensure!(from == "    fn from(value: f6) -> Self {", "from: {from:?}");
        checkpoint("1 undo took the picked completion out of both copies")?;

        command('z');
        let (header, from, _) = view.state("Cmd Z: the 4 is back in both copies");
        ensure!(header == "impl From<f64> for Meters {", "header: {header:?}");
        ensure!(from == "    fn from(value: f64) -> Self {", "from: {from:?}");
        checkpoint("1 undo put the 4 back in both copies")?;

        command('z');
        let (header, from, _) = view.state("Cmd Z: the typed 64 is gone from both copies");
        ensure!(header == "impl From<f> for Meters {", "header: {header:?}");
        ensure!(from == "    fn from(value: f) -> Self {", "from: {from:?}");
        checkpoint("1 more undo took the typed 64 out of both copies")?;
        Ok(())
    }
}
