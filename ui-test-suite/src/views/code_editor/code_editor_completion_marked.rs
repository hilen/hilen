use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{CodeEditor, CodePos, Completion, Label, NamedKey, Setup, ViewData, ViewTest, WHITE, view},
    ui_test::{checkpoint, inject_named_key},
};

use super::common::{SCAD, click_at, color_all, status_style};

/// The line with `plate();`.
const LINE: usize = 11;
/// More rows than the 8 the list shows at once.
const ROWS: usize = 10;

/// The event of the marked row of a completion list. A list that shows
/// marks its first row, Down and Up move the mark 1 row each, an Up on
/// the first row and a Down on the last move nothing and tell nothing,
/// and a Down past the rows in view scrolls the list and tells the new
/// row too. A new list marks its first row again. The status label lists
/// every mark the event told.
#[view]
struct CodeEditorCompletionMarked {
    marks: Vec<usize>,

    #[init]
    editor: CodeEditor,
    status: Label,
}

impl Setup for CodeEditorCompletionMarked {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        // Below the top 40 points, a tap there hits the status bar of a
        // phone.
        self.editor.place().t(50).lr(10).h(380);
        self.editor.set_text(SCAD);
        color_all(self.editor);
        self.editor.completion_marked.val(move |index| self.marks.push(index));

        status_style(self.status);
        // Under the list, which floats below line 12 down to y 508.
        self.status.place().t(520).lr(10).b(10);
    }
}

fn rows() -> Vec<Completion> {
    (0..ROWS)
        .map(|row| Completion {
            label:  format!("plate_{row}"),
            note:   String::new(),
            detail: "module".to_string(),
            icon:   String::new(),
        })
        .collect()
}

impl CodeEditorCompletionMarked {
    fn open(self: Weak<Self>) {
        let editor = self.editor;
        from_main(move || editor.show_completions(CodePos::new(LINE, 0), rows()));
        wait_for_next_frame();
    }

    fn press(key: NamedKey, times: usize) {
        for _ in 0..times {
            inject_named_key(key);
        }
        wait_for_next_frame();
    }

    /// The marks told so far, also on the status label, and a hold named
    /// `step`.
    fn marks(self: Weak<Self>, step: &'static str) -> Result<Vec<usize>> {
        let marks = from_main(move || {
            let marks = self.marks.clone();
            let told: Vec<String> = marks.iter().map(ToString::to_string).collect();
            self.status.set_text(format!("{step}\nmarks told: {}", told.join(", ")));
            marks
        });
        wait_for_next_frame();
        checkpoint(step)?;
        Ok(marks)
    }
}

impl ViewTest for CodeEditorCompletionMarked {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        click_at(view.editor, CodePos::new(LINE, 5));
        view.open();
        let marks = view.marks("the list shows with its first row marked")?;
        ensure!(marks == [0], "a list that shows told {marks:?}");

        Self::press(NamedKey::ArrowDown, 2);
        Self::press(NamedKey::ArrowUp, 1);
        let marks = view.marks("Down 2 times, then Up: rows 1, 2, then 1")?;
        ensure!(marks == [0, 1, 2, 1], "Down and Up told {marks:?}");

        Self::press(NamedKey::ArrowUp, 2);
        let marks = view.marks("Up 2 times: row 0, the second Up tells nothing")?;
        ensure!(marks == [0, 1, 2, 1, 0], "an Up on the first row told {marks:?}");

        Self::press(NamedKey::ArrowDown, ROWS);
        let marks =
            view.marks("Down 10 times: rows 1 to 9 past the 8 in view, the last Down tells nothing")?;
        ensure!(
            marks[5..] == [1, 2, 3, 4, 5, 6, 7, 8, 9],
            "the Downs to the last row told {:?}",
            &marks[5..]
        );
        let marked = from_main(move || view.editor.marked_completion());
        ensure!(marked == Some(ROWS - 1), "the list marks {marked:?}");

        view.open();
        let marks = view.marks("a new list marks its first row again")?;
        ensure!(
            marks.last() == Some(&0) && marks.len() == 15,
            "a new list told {marks:?}"
        );
        Ok(())
    }
}
