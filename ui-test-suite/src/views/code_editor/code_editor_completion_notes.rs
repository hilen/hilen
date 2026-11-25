use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{CodeEditor, CodePos, Completion, Label, Setup, ViewData, ViewFrame, ViewTest, WHITE, view},
    ui_test::{check_colors, set_record_probe_count},
};

use super::common::{SCAD, click_at, color_all, status_style};

/// The line with `plate();`.
const LINE: usize = 11;
/// The width of a list whose rows are short.
const LEAST_WIDTH: f32 = 420.0;

/// The note of a completion: `Add` and `AddAssign` come from another
/// module, and right after each name stands its note in the dim color of
/// the details, `(use std::ops::Add)`, the way Zed shows the label details
/// of a language server. The row of `abs` has no note and looks as
/// before. The list is wide enough for the longest name with its note.
#[view]
struct CodeEditorCompletionNotes {
    #[init]
    editor: CodeEditor,
    status: Label,
}

impl Setup for CodeEditorCompletionNotes {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);
        self.editor.place().t(10).lr(10).h(380);
        self.editor.set_text(SCAD);
        color_all(self.editor);

        status_style(self.status);
        self.status.place().t(400).lr(10).b(10);
    }
}

fn rows() -> Vec<Completion> {
    [
        ("Add", "(use std::ops::Add)", "trait"),
        ("AddAssign", "(use std::ops::AddAssign)", "trait"),
        ("abs", "", "const fn(self) -> i32"),
    ]
    .iter()
    .map(|(label, note, detail)| Completion {
        label:  (*label).to_string(),
        note:   (*note).to_string(),
        detail: (*detail).to_string(),
        icon:   String::new(),
    })
    .collect()
}

impl ViewTest for CodeEditorCompletionNotes {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);
        click_at(view.editor, CodePos::new(LINE, 2));
        from_main(move || {
            view.editor.show_completions(CodePos::new(LINE, 0), rows());
            view.status.set_text(
                "3 names: Add and AddAssign with their notes in dim text right after the name, abs with no \
                 note. The details stand at the right",
            );
        });
        wait_for_next_frame();
        let width = from_main(move || view.editor.panel().absolute_frame().size.width);
        ensure!(width > LEAST_WIDTH, "the list with notes is {width} wide");
        check_colors(NOTES)?;
        Ok(())
    }
}

/// Recorded with `--record-colors`.
const NOTES: &str = r"
             592    4 - #ffffff
             396   12 - #ffffff
             112   24 - #6f7881
             168   24 - #747c86
             200   24 - #747c86
              76   44 - #282d33
             496   64 - #ffffff
              72   84 - #cf222e
              72   88 - #cf232f
              80   88 - #cf222e
              92   88 - #d02632
             140   88 - #8453e0
              88  108 - #8250df
             124  112 - #a17ce7
             180  132 - #282d33
             592  144 - #ffffff
              28  148 - #bbc1c8
             124  156 - #a17ce7
             136  156 - #8251df
             168  156 - #8251df
             320  172 - #ffffff
             456  172 - #ffffff
             204  176 - #a17ce7
             284  176 - #5b5f63
              32  196 - #8d96a0
              72  264 - #8251df
              88  284 - #616c74
             172  284 - #a7b4bf
             248  284 - #8c959f
             256  284 - #a7b4bf
             588  284 - #d3d3d3
             248  288 - #8c959f
             588  300 - #d3d3d3
              88  308 - #6c6f74
             168  312 - #98a0a9
             172  312 - #b3b9c0
             296  312 - #ffffff
             588  316 - #d3d3d3
              84  332 - #43474c
             588  332 - #d3d3d3
             588  344 - #d3d3d3
             132  348 - #acacac
             228  348 - #acacac
             272  348 - #acacac
             412  348 - #acacac
             580  348 - #acacac
             184  356 - #e8e8e8
             316  356 - #e8e8e8
             368  356 - #e8e8e8
             456  356 - #e8e8e8
             520  356 - #e8e8e8
             280  484 - #000000
             384  484 - #000000
              76  488 - #ffffff
             424  488 - #000000
             456  488 - #010101
             492  488 - #ffffff
             556  488 - #2b2b2b
             192  500 - #ffffff
              36  504 - #000000
             132  504 - #3d3d3d
             156  504 - #2e2e2e
             240  504 - #ffffff
             316  504 - #040404
";
