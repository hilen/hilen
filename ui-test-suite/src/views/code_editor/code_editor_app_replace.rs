use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{CodeEditor, CodePos, Label, Setup, ViewData, ViewTest, WHITE, view},
    ui_test::checkpoint,
};

use super::common::{click_at, command, status_style};

const LINES: usize = 120;

fn numbered() -> String {
    (0..LINES).map(|line| format!("line {line}")).collect::<Vec<_>>().join("\n")
}

/// A replace of the app, like a format on save, in a long file scrolled
/// to its middle. 3 lines above the screen give way to 1, 2 spaces go in
/// before a selection, and a line is added above while the caret is off
/// screen. The lines on screen stay where they are, the caret and the
/// selection keep their place in the text, the view never jumps to the
/// caret, `changed` fires each time, and Cmd Z takes a replace back.
#[view]
struct CodeEditorAppReplace {
    changes: usize,

    #[init]
    editor: CodeEditor,
    status: Label,
}

impl Setup for CodeEditorAppReplace {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        // Below the top 40 points, a tap there hits the status bar of a
        // phone.
        self.editor.place().t(50).lr(10).h(380);
        self.editor.set_text(&numbered());
        self.editor.changed.val(move |_| self.changes += 1);

        status_style(self.status);
        self.status.place().t(440).lr(10).b(10);
    }
}

/// What a step reads of the editor.
struct Seen {
    lines:     Vec<String>,
    cursor:    CodePos,
    selection: Option<(CodePos, CodePos)>,
    changes:   usize,
}

impl CodeEditorAppReplace {
    fn seen(self: Weak<Self>) -> Seen {
        wait_for_next_frame();
        let editor = self.editor;
        from_main(move || Seen {
            lines:     editor.lines().to_vec(),
            cursor:    editor.cursor(),
            selection: editor.selection(),
            changes:   self.changes,
        })
    }

    /// Where `line` is from the top of the editor, negative above it.
    fn y_of(self: Weak<Self>, line: usize) -> f32 {
        let editor = self.editor;
        from_main(move || editor.point_of(CodePos::new(line, 0)).map_or(f32::NAN, |point| point.y))
    }

    fn replace(self: Weak<Self>, from: CodePos, to: CodePos, text: &'static str) {
        let mut editor = self.editor;
        from_main(move || editor.replace(from, to, text));
    }

    fn hold(self: Weak<Self>, step: &'static str) -> Result<()> {
        from_main(move || {
            self.status.set_text(step);
        });
        checkpoint(step)
    }
}

impl ViewTest for CodeEditorAppReplace {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let mut editor = view.editor;
        from_main(move || editor.scroll_to_line(40));
        wait_for_next_frame();
        click_at(editor, CodePos::new(50, 2));
        let top = view.y_of(40);
        let seen = view.seen();
        ensure!(
            seen.cursor == CodePos::new(50, 2),
            "the click put the caret at {:?}",
            seen.cursor
        );
        view.hold("Scrolled to line 41, the caret in `line 50`")?;

        // 3 lines above the screen give way to 1, 2 lines less.
        view.replace(CodePos::new(5, 0), CodePos::new(8, 0), "lines 5 to 7\n");
        let seen = view.seen();
        ensure!(
            seen.lines[5] == "lines 5 to 7" && seen.lines[6] == "line 8",
            "the replace gave {:?}",
            &seen.lines[4..8]
        );
        ensure!(seen.lines[38] == "line 40", "`line 40` is not on line 39 now");
        let moved_top = view.y_of(38);
        ensure!(
            (moved_top - top).abs() < 0.5,
            "`line 40` moved on the screen from {top} to {moved_top}"
        );
        ensure!(
            seen.cursor == CodePos::new(48, 2),
            "the caret is at {:?}, not in `line 50` on line 49",
            seen.cursor
        );
        ensure!(seen.changes == 1, "`changed` fired {} times", seen.changes);
        view.hold(
            "3 lines above the screen became 1: `line 40` stays at the top, the caret stays in `line 50`",
        )?;

        // 2 spaces in place of 1, right before the selected `50`.
        from_main(move || editor.select(CodePos::new(48, 5), CodePos::new(48, 7)));
        view.replace(CodePos::new(48, 4), CodePos::new(48, 5), "  ");
        let seen = view.seen();
        ensure!(seen.lines[48] == "line  50", "line 49 is {:?}", seen.lines[48]);
        ensure!(
            seen.selection == Some((CodePos::new(48, 6), CodePos::new(48, 8))),
            "the selection is {:?}, not `50`",
            seen.selection
        );
        view.hold("2 spaces before the selected `50`: the selection keeps `50`")?;

        // The caret off screen, a line added above the screen.
        from_main(move || editor.scroll_to_line(80));
        wait_for_next_frame();
        let top = view.y_of(80);
        view.replace(CodePos::new(10, 0), CodePos::new(10, 0), "added\n");
        let seen = view.seen();
        ensure!(seen.lines[10] == "added", "line 11 is {:?}", seen.lines[10]);
        let moved_top = view.y_of(81);
        ensure!(
            (moved_top - top).abs() < 0.5,
            "the top line moved on the screen from {top} to {moved_top}"
        );
        let caret = view.y_of(seen.cursor.line);
        ensure!(caret < 0.0, "the view went to the caret, its line is at {caret}");
        ensure!(seen.changes == 3, "`changed` fired {} times", seen.changes);
        view.hold("A line added above while the caret is off screen: the view stays, no jump to the caret")?;

        command('z');
        let seen = view.seen();
        // 2 lines less from the first replace, so `line 12` is on line 11.
        ensure!(
            seen.lines[10] == "line 12" && !seen.lines.iter().any(|line| line == "added"),
            "Cmd Z left line 11 as {:?}",
            seen.lines[10]
        );
        ensure!(
            seen.lines[48] == "line  50",
            "Cmd Z took more than the last replace back"
        );
        view.hold("Cmd Z: the added line is gone in 1 step, the 2 spaces stay")?;

        Ok(())
    }
}
