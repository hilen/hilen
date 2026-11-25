use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{CodeEditor, CodePos, Label, Modifiers, Setup, ViewData, ViewTest, WHITE, view},
    ui_test::{checkpoint, inject_keys, inject_modifiers},
};

use super::common::{click_at, command, quoted, status_style};

const CODE: &str = "fn main() {\n    let map = \n}";

const IMPORT: &str = "use std::collections::HashMap;\n\n";

/// A late change that joins the undo step before it, the way a language
/// server sends the `use` line of a completion after its name went in.
/// The name goes in at the caret, then the `use` line at the top with
/// the version of the text right after the name. Cmd Z takes both back
/// in 1 step and Cmd Shift Z puts both back. Then `::new();` is typed
/// before a second late line comes, which is a step of its own, so Cmd Z
/// takes only that line back. Last a late snippet with a place joins the
/// step of `x` typed before it, `Self` is selected, and Cmd Z takes both
/// back. The status label shows the lines, each in quotes.
#[view]
struct CodeEditorJoinedUndo {
    #[init]
    editor: CodeEditor,
    status: Label,
}

impl Setup for CodeEditorJoinedUndo {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);
        // Below the top 40 points, a tap there hits the status bar of a
        // phone.
        self.editor.place().t(50).lr(10).h(260);
        self.editor.set_text(CODE);
        status_style(self.status);
        self.status.place().t(320).lr(10).b(10);
    }
}

impl CodeEditorJoinedUndo {
    /// The lines and the caret, the lines also on the status label, and a
    /// hold named `step`.
    fn show(self: Weak<Self>, step: &'static str) -> Result<(Vec<String>, CodePos)> {
        wait_for_next_frame();
        let seen = from_main(move || {
            let lines = self.editor.lines().to_vec();
            self.status.set_text(format!("{step}\n{}", quoted(&lines)));
            (lines, self.editor.cursor())
        });
        checkpoint(step)?;
        Ok(seen)
    }

    /// The `use` line at the top, joined to the step at `version`.
    fn late_line(self: Weak<Self>, version: u64, text: &'static str) {
        let mut editor = self.editor;
        from_main(move || {
            editor.replace_joined(version, CodePos::new(0, 0), CodePos::new(0, 0), text);
        });
    }
}

impl ViewTest for CodeEditorJoinedUndo {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let mut editor = view.editor;
        click_at(editor, CodePos::new(1, 14));
        let version = from_main(move || {
            editor.insert("HashMap");
            editor.version()
        });
        view.late_line(version, IMPORT);
        let (both, cursor) = view.show("the name went in, then its use line came")?;
        ensure!(
            both == [
                "use std::collections::HashMap;",
                "",
                "fn main() {",
                "    let map = HashMap",
                "}"
            ],
            "the name and its use line gave {both:?}"
        );
        ensure!(cursor == CodePos::new(3, 21), "the caret is at {cursor:?}");

        command('z');
        let (lines, cursor) = view.show("Cmd Z: the use line and the name are gone in 1 step")?;
        ensure!(lines.join("\n") == CODE, "1 undo left {lines:?}");
        ensure!(cursor == CodePos::new(1, 14), "the caret is back at {cursor:?}");
        ensure!(!from_main(move || editor.can_undo()), "the undo left a step");

        inject_modifiers(Modifiers::META | Modifiers::SHIFT);
        inject_keys('Z');
        inject_modifiers(Modifiers::empty());
        let (lines, _) = view.show("Cmd Shift Z: the name and the use line are back in 1 step")?;
        ensure!(lines == both, "1 redo gave {lines:?}");
        ensure!(!from_main(move || editor.can_redo()), "the redo left a step");

        let version = from_main(move || editor.version());
        inject_keys("::new();");
        view.late_line(version, "// typed meanwhile\n");
        let (lines, _) = view.show("::new(); typed, then a late line: it is a step of its own")?;
        ensure!(
            lines[0] == "// typed meanwhile" && lines[4] == "    let map = HashMap::new();",
            "the typing and the late line gave {lines:?}"
        );

        command('z');
        let (lines, _) = view.show("Cmd Z: only the late line is gone, the typing stays")?;
        ensure!(
            lines[0] == "use std::collections::HashMap;" && lines[3] == "    let map = HashMap::new();",
            "the undo of the late line gave {lines:?}"
        );

        click_at(editor, CodePos::new(4, 1));
        inject_keys("x");
        let version = from_main(move || editor.version());
        from_main(move || {
            editor.insert_snippet_joined(
                version,
                CodePos::new(4, 1),
                CodePos::new(4, 2),
                "type T = ${1:Self};",
            );
        });
        let (lines, _) = view.show("a late snippet over the typed x: `Self` is selected")?;
        let selected = from_main(move || editor.selected_text());
        ensure!(
            lines[4] == "}type T = Self;" && selected == "Self",
            "the late snippet gave {lines:?} with {selected:?} selected"
        );
        command('z');
        let (lines, _) = view.show("Cmd Z: the snippet and the typed x are gone in 1 step")?;
        ensure!(lines[4] == "}", "the undo of the late snippet gave {lines:?}");
        Ok(())
    }
}
