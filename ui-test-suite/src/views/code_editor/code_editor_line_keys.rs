use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        CodeEditor, CodePos, CommentRules, Label, Modifiers, NamedKey, Setup, ViewData, ViewTest, WHITE, view,
    },
    ui_test::{checkpoint, inject_keys, inject_modifiers, inject_named_key},
};

use super::common::{click_at, command, command_with, quoted, status_style};

const CODE: &str = "fn main() {\n    let a = 1;\n    let b = 2;\n}";

/// The line keys of the Zed keymap in a code editor of Rust. Cmd / puts
/// `// ` in front of `let a = 1;` and takes it out again. Alt Down moves
/// the line under `let b`, Alt Up moves it back. Alt Shift Down puts a
/// copy under it and Cmd Shift K takes the copy out. Cmd Enter opens a
/// line under `let b` with
/// its indent, Cmd Shift Enter one above it. Cmd L selects the line of
/// the cursor, a second Cmd L the line under it too. The word key with
/// Backspace takes the word before the cursor, Alt on a Mac and Ctrl
/// elsewhere. The status label shows the lines, each in quotes.
#[view]
struct CodeEditorLineKeys {
    #[init]
    editor: CodeEditor,
    status: Label,
}

impl Setup for CodeEditorLineKeys {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);
        // Below the top 40 points, a tap there hits the status bar of a
        // phone.
        self.editor.place().t(50).lr(10).h(240);
        self.editor.set_text(CODE);
        self.editor.set_comments(CommentRules::rust());
        status_style(self.status);
        self.status.place().t(300).lr(10).b(10);
    }
}

impl CodeEditorLineKeys {
    /// The lines and the cursor, the lines also on the status label, and
    /// a hold named `step`.
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
}

fn with(modifiers: Modifiers, key: NamedKey) {
    inject_modifiers(modifiers);
    inject_named_key(key);
    inject_modifiers(Modifiers::empty());
}

impl ViewTest for CodeEditorLineKeys {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        click_at(view.editor, CodePos::new(1, 4));
        command('/');
        let (lines, cursor) = view.show("Cmd /: `let a` is a comment, the cursor stays in front of `let`")?;
        ensure!(lines[1] == "    // let a = 1;", "Cmd / gave {lines:?}");
        ensure!(cursor == CodePos::new(1, 7), "the cursor is at {cursor:?}");
        command('/');
        let (lines, _) = view.show("Cmd / again: the comment is gone")?;
        ensure!(lines[1] == "    let a = 1;", "the second Cmd / gave {lines:?}");

        with(Modifiers::ALT, NamedKey::ArrowDown);
        let (lines, cursor) = view.show("Alt Down: `let a` moved under `let b`, the cursor with it")?;
        ensure!(
            lines[1..3] == ["    let b = 2;", "    let a = 1;"],
            "Alt Down gave {lines:?}"
        );
        ensure!(cursor.line == 2, "the cursor is on line {}", cursor.line + 1);
        with(Modifiers::ALT, NamedKey::ArrowUp);
        let (lines, _) = view.show("Alt Up: `let a` is back above `let b`")?;
        ensure!(lines.join("\n") == CODE, "Alt Up gave {lines:?}");

        with(Modifiers::ALT | Modifiers::SHIFT, NamedKey::ArrowDown);
        let (lines, cursor) = view.show("Alt Shift Down: a copy of `let a`, the cursor on the lower one")?;
        ensure!(
            lines[1..3] == ["    let a = 1;", "    let a = 1;"],
            "Alt Shift Down gave {lines:?}"
        );
        ensure!(cursor.line == 2, "the cursor is on line {}", cursor.line + 1);
        command_with(Modifiers::SHIFT, 'k');
        let (lines, cursor) = view.show("Cmd Shift K: the copy is gone, the cursor on `let b`")?;
        ensure!(lines.join("\n") == CODE, "Cmd Shift K gave {lines:?}");
        ensure!(cursor.line == 2, "the cursor is on line {}", cursor.line + 1);

        with(Modifiers::META, NamedKey::Enter);
        inject_keys("x");
        let (lines, _) = view.show("Cmd Enter: a line under `let b` with its indent, x typed there")?;
        ensure!(lines[3] == "    x", "Cmd Enter gave {lines:?}");
        with(Modifiers::META | Modifiers::SHIFT, NamedKey::Enter);
        inject_keys("y");
        let (lines, _) = view.show("Cmd Shift Enter: a line above x with its indent, y typed there")?;
        ensure!(
            lines[3..5] == ["    y", "    x"],
            "Cmd Shift Enter gave {lines:?}"
        );

        click_at(view.editor, CodePos::new(1, 6));
        command('l');
        let selected = from_main(move || view.editor.selected_text());
        ensure!(selected == "    let a = 1;\n", "Cmd L selected {selected:?}");
        command('l');
        let selected = from_main(move || view.editor.selected_text());
        view.show("Cmd L twice: `let a` and `let b` are selected")?;
        ensure!(
            selected == "    let a = 1;\n    let b = 2;\n",
            "the second Cmd L selected {selected:?}"
        );

        click_at(view.editor, CodePos::new(1, 14));
        let word = if cfg!(target_os = "macos") {
            Modifiers::ALT
        } else {
            Modifiers::CONTROL
        };
        with(word, NamedKey::Backspace);
        with(word, NamedKey::Backspace);
        let (lines, _) = view.show("the word key with Backspace twice: `;`, then `1`")?;
        ensure!(lines[1] == "    let a = ", "the word deletes gave {lines:?}");
        Ok(())
    }
}
