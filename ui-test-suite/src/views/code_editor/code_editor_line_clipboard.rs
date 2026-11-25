use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    system::Clipboard,
    ui::{CodeEditor, CodePos, Label, Setup, ViewData, ViewTest, WHITE, view},
    ui_test::checkpoint,
};

use super::common::{click_at, command, quoted, status_style};

const CODE: &str = "first\nsecond\nthird";

/// Cmd C and Cmd X with nothing selected take the whole line of the
/// cursor with its break, the way Zed has it. Cmd C in `first` and Cmd V
/// in the middle of `third` put `first` above `third`, and the cursor
/// stays in `third`. Cmd X in `second` takes that line out, and Cmd V at
/// the end puts it above the last line. A copy of a selection pastes as
/// it is, also with nothing selected. The status label shows the lines,
/// each in quotes.
#[view]
struct CodeEditorLineClipboard {
    #[init]
    editor: CodeEditor,
    status: Label,
}

impl Setup for CodeEditorLineClipboard {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);
        self.editor.place().t(50).lr(10).h(200);
        self.editor.set_text(CODE);
        status_style(self.status);
        self.status.place().t(260).lr(10).b(10);
    }
}

impl CodeEditorLineClipboard {
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

impl ViewTest for CodeEditorLineClipboard {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(|| Clipboard::set_in_process(true));
        let editor = view.editor;

        click_at(editor, CodePos::new(0, 2));
        command('c');
        let copied = from_main(Clipboard::get_text)?;
        ensure!(copied == "first\n", "Cmd C took {copied:?}");
        click_at(editor, CodePos::new(2, 3));
        command('v');
        let (lines, cursor) = view.show("Cmd C in `first`, Cmd V in `third`: `first` is above `third`")?;
        ensure!(
            lines == ["first", "second", "first", "third"],
            "the paste gave {lines:?}"
        );
        ensure!(cursor == CodePos::new(3, 3), "the cursor is at {cursor:?}");

        click_at(editor, CodePos::new(1, 3));
        command('x');
        let cut = from_main(Clipboard::get_text)?;
        let (lines, _) = view.show("Cmd X in `second`: the line is gone")?;
        ensure!(cut == "second\n", "Cmd X took {cut:?}");
        ensure!(lines == ["first", "first", "third"], "the cut gave {lines:?}");
        click_at(editor, CodePos::new(2, 5));
        command('v');
        let (lines, _) = view.show("Cmd V at the end of `third`: `second` is above it")?;
        ensure!(
            lines == ["first", "first", "second", "third"],
            "the paste gave {lines:?}"
        );

        // A selection copied is no whole line, it goes in at the cursor.
        from_main(move || {
            let mut editor = editor;
            editor.select(CodePos::new(3, 0), CodePos::new(3, 2));
        });
        command('c');
        click_at(editor, CodePos::new(0, 5));
        command('v');
        let (lines, _) = view.show("a copy of `th` pastes at the cursor, at the end of `first`")?;
        ensure!(lines[0] == "firstth", "the paste of a selection gave {lines:?}");
        Ok(())
    }
}
