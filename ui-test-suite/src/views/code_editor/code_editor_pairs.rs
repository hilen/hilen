use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        BracketRules, CodeEditor, CodeHighlighter, CodePos, Label, NamedKey, Setup, ViewData, ViewTest,
        WHITE, view,
    },
    ui_test::{checkpoint, inject_keys, inject_named_key},
};

use super::common::{click_at, quoted, status_style};

/// Line 3 holds 4 spaces, the place the typing starts.
const CODE: &str = "fn main() {\n    let s = \"a /\";\n    \n}";

/// The pairs of Rust in a code editor, the way the Zed editor has them.
/// `f(` gets its `)`, and the typed `)` steps over it. Enter between `{`
/// and `}` puts the `}` on its own line. Backspace right after a `[` the
/// editor closed takes out both. `(` over a selected word wraps it. In
/// the string of line 2 a typed `/*` opens no comment, the app tells the
/// editor where the string is with the scopes of its highlighter. The
/// status label shows the lines, each in quotes.
#[view]
struct CodeEditorPairs {
    #[init]
    editor: CodeEditor,
    status: Label,
}

impl Setup for CodeEditorPairs {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);
        self.editor.place().t(10).lr(10).h(300);
        self.editor.set_text(CODE);
        self.editor.set_brackets(BracketRules::for_file("main.rs"));
        self.color();
        self.editor.changed.val(move |_| self.color());

        status_style(self.status);
        self.status.place().t(320).lr(10).b(10);
    }
}

impl CodeEditorPairs {
    /// Colors every line again and sets its scopes, like an app on every
    /// change.
    fn color(self: Weak<Self>) {
        let Some(mut rust) = CodeHighlighter::for_file("main.rs") else {
            return;
        };
        let lines = self.editor.lines().to_vec();
        for (index, line) in lines.iter().enumerate() {
            let runs = rust.runs(line);
            self.editor.set_color_runs(index, runs.colors);
            self.editor.set_scope_runs(index, runs.scopes);
        }
    }

    /// The lines and the cursor, the lines also on the status label.
    fn state(self: Weak<Self>, step: &'static str) -> (Vec<String>, CodePos) {
        wait_for_next_frame();
        from_main(move || {
            let lines = self.editor.lines().to_vec();
            self.status.set_text(format!("{step}\n{}", quoted(&lines)));
            (lines, self.editor.cursor())
        })
    }
}

fn line(lines: &[String], index: usize) -> &str {
    lines.get(index).map_or("", String::as_str)
}

impl ViewTest for CodeEditorPairs {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let editor = view.editor;
        click_at(editor, CodePos::new(2, 4));
        inject_keys("f(");
        let (lines, cursor) = view.state("f( typed: the editor put the ) in after the cursor");
        ensure!(line(&lines, 2) == "    f()", "after f(: {lines:?}");
        ensure!(cursor == CodePos::new(2, 6), "the cursor after f( is {cursor:?}");
        checkpoint("f( typed, the ) is there and the cursor between")?;

        inject_keys("a)");
        let (lines, cursor) = view.state("a) typed: the ) stepped over the one the editor put in");
        ensure!(line(&lines, 2) == "    f(a)", "after a): {lines:?}");
        ensure!(cursor == CodePos::new(2, 8), "the cursor after a) is {cursor:?}");
        checkpoint("a) typed, still 1 ), the cursor after it")?;

        inject_keys(" {");
        inject_named_key(NamedKey::Enter);
        let (lines, cursor) = view.state("{ and Enter: the } went to its own line");
        ensure!(
            line(&lines, 2) == "    f(a) {" && line(&lines, 3) == "        " && line(&lines, 4) == "    }",
            "after Enter between braces: {lines:?}"
        );
        ensure!(
            cursor == CodePos::new(3, 8),
            "the cursor after Enter is {cursor:?}"
        );
        checkpoint("Enter between { and }, the } on its own line")?;

        inject_keys("[");
        let (lines, _) = view.state("[ typed: the editor put the ] in");
        ensure!(line(&lines, 3) == "        []", "after [: {lines:?}");
        inject_named_key(NamedKey::Backspace);
        let (lines, _) = view.state("Backspace right after the [ took out both");
        ensure!(line(&lines, 3) == "        ", "after Backspace: {lines:?}");
        checkpoint("Backspace took out the [ and the ] together")?;

        inject_keys("x");
        from_main(move || {
            let mut editor = editor;
            editor.select(CodePos::new(3, 8), CodePos::new(3, 9));
        });
        inject_keys("(");
        let (lines, _) = view.state("x selected and ( typed: the x is wrapped");
        ensure!(line(&lines, 3) == "        (x)", "after the wrap: {lines:?}");
        checkpoint("( over the selected x wrapped it in ( and )")?;

        // Between the / and the closing quote of the string on line 2.
        click_at(editor, CodePos::new(1, 16));
        inject_keys("*");
        let (lines, _) = view.state("* typed after the / in the string: no */ came");
        ensure!(
            line(&lines, 1) == r#"    let s = "a /*";"#,
            "in the string: {lines:?}"
        );
        checkpoint("/* in a string opened no comment")?;
        Ok(())
    }
}
