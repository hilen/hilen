use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        CodeEditor, CodePos, CommentRules, Label, Modifiers, NamedKey, Setup, ViewData, ViewTest, WHITE, view,
    },
    ui_test::{checkpoint, inject_keys, inject_modifiers, inject_named_key},
};

use super::common::{click_at, quoted, status_style};

const CODE: &str = "    let value = 1;\n    // more\nend";

/// The Ctrl letters of a Mac in a code editor, the keys of the text views
/// of the system that the Zed keymap has. Ctrl E and Ctrl A go to the
/// line end and to the start of the code. Ctrl F, B, N and P step right,
/// left, down and up. Ctrl K cuts to the line end and Ctrl Y puts it
/// back. Ctrl T swaps 2 chars, Ctrl H, D and W delete back, forward and
/// a word back, and Ctrl J joins the next line, which loses its `//`. A
/// Ctrl letter with no key of its own puts in nothing. Last Cmd Delete
/// and Cmd Backspace take the rest of the line after and before the
/// cursor. The status label shows the lines, each in quotes.
#[view]
struct CodeEditorControlKeys {
    #[init]
    editor: CodeEditor,
    status: Label,
}

impl Setup for CodeEditorControlKeys {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);
        self.editor.place().t(50).lr(10).h(200);
        self.editor.set_text(CODE);
        self.editor.set_comments(CommentRules::rust());
        status_style(self.status);
        self.status.place().t(260).lr(10).b(10);
    }
}

impl CodeEditorControlKeys {
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

    fn cursor(self: Weak<Self>) -> CodePos {
        from_main(move || self.editor.cursor())
    }
}

fn ctrl(keys: &str) {
    inject_modifiers(Modifiers::CONTROL);
    for key in keys.chars() {
        inject_keys(key);
    }
    inject_modifiers(Modifiers::empty());
}

fn cmd(key: NamedKey) {
    inject_modifiers(Modifiers::META);
    inject_named_key(key);
    inject_modifiers(Modifiers::empty());
}

impl ViewTest for CodeEditorControlKeys {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        click_at(view.editor, CodePos::new(0, 9));
        ctrl("e");
        ensure!(
            view.cursor() == CodePos::new(0, 18),
            "Ctrl E went to {:?}",
            view.cursor()
        );
        ctrl("a");
        ensure!(
            view.cursor() == CodePos::new(0, 4),
            "Ctrl A went to {:?}",
            view.cursor()
        );
        ctrl("ffb");
        ensure!(
            view.cursor() == CodePos::new(0, 5),
            "Ctrl F F B went to {:?}",
            view.cursor()
        );
        ctrl("np");
        let (_, cursor) = view.show("Ctrl E, A, F, F, B, N and P: the cursor after `l` of `let`")?;
        ensure!(cursor == CodePos::new(0, 5), "Ctrl N P went to {cursor:?}");

        ctrl("k");
        let (lines, _) = view.show("Ctrl K: the rest of the line is cut")?;
        ensure!(lines[0] == "    l", "Ctrl K gave {lines:?}");
        ctrl("y");
        let (lines, _) = view.show("Ctrl Y: it is back")?;
        ensure!(lines[0] == "    let value = 1;", "Ctrl Y gave {lines:?}");

        click_at(view.editor, CodePos::new(0, 5));
        ctrl("t");
        let (lines, _) = view.show("Ctrl T after `l`: `l` and `e` changed place")?;
        ensure!(lines[0] == "    elt value = 1;", "Ctrl T gave {lines:?}");
        ctrl("hd");
        let (lines, _) = view.show("Ctrl H and Ctrl D: the chars before and after the cursor are gone")?;
        ensure!(lines[0] == "    e value = 1;", "Ctrl H D gave {lines:?}");
        ctrl("q");
        let (lines, _) = view.show("Ctrl Q has no key, nothing went in")?;
        ensure!(lines[0] == "    e value = 1;", "Ctrl Q gave {lines:?}");

        click_at(view.editor, CodePos::new(0, 16));
        ctrl("w");
        ctrl("j");
        let (lines, _) = view.show("Ctrl W took `;`, Ctrl J joined `more` without its `//`")?;
        ensure!(lines[0] == "    e value = 1 more", "Ctrl W J gave {lines:?}");

        click_at(view.editor, CodePos::new(0, 11));
        cmd(NamedKey::Delete);
        let (lines, _) = view.show("Cmd Delete: the rest after `value` is gone")?;
        ensure!(lines[0] == "    e value", "Cmd Delete gave {lines:?}");
        cmd(NamedKey::Backspace);
        let (lines, cursor) = view.show("Cmd Backspace: the line before the cursor is gone too")?;
        ensure!(
            lines[0].is_empty() && cursor == CodePos::new(0, 0),
            "Cmd Backspace gave {lines:?}"
        );
        Ok(())
    }
}
