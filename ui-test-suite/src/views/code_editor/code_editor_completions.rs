use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        CodeEditor, CodePos, Color, Completion, Label, NamedKey, Setup, TextAlignment, UIColor, ViewData,
        ViewFrame, ViewTest, WHITE, view,
    },
    ui_test::{check_colors, inject_keys, inject_named_key, set_record_probe_count},
};

use super::common::{SCAD, click_at, color_all, status_style};

const NOTE: UIColor = UIColor::Plain(Color::hex("#fff8c5"));
const NOTE_BORDER: UIColor = UIColor::Plain(Color::hex("#d4a72c"));

/// The line with `plate();`.
const LINE: usize = 11;
/// The line with `module plate() {`.
const MODULE_LINE: usize = 3;

/// The list of completions of a code editor and the panel it floats in.
/// The list opens under the word the caret is in, with its first row
/// marked. Down marks the next row. A letter still goes into the code.
/// Enter picks the marked row and closes the list, with no new line in
/// the code. Escape closes a list too, and the editor keeps the keys. Last
/// a plain panel of the app, a yellow note under a name.
#[view]
struct CodeEditorCompletions {
    picked: Option<usize>,

    #[init]
    editor: CodeEditor,
    status: Label,
}

impl Setup for CodeEditorCompletions {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        self.editor.place().t(10).lr(10).h(380);
        self.editor.set_text(SCAD);
        color_all(self.editor);
        self.editor.completion_picked.val(move |index| self.picked = Some(index));

        status_style(self.status);
        self.status.place().t(400).lr(10).b(10);
    }
}

fn items() -> Vec<Completion> {
    [
        ("plate", "module"),
        ("plate_with_holes", "module"),
        ("place", "function"),
    ]
    .into_iter()
    .map(|(label, detail)| Completion {
        label:  label.to_string(),
        detail: detail.to_string(),
        icon:   String::new(),
    })
    .collect()
}

fn open(view: Weak<CodeEditorCompletions>) {
    from_main(move || {
        view.editor.show_completions(CodePos::new(LINE, 0), items());
    });
    wait_for_next_frame();
}

fn say(view: Weak<CodeEditorCompletions>, text: &'static str) {
    from_main(move || {
        view.status.set_text(text);
    });
}

impl ViewTest for CodeEditorCompletions {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);
        click_at(view.editor, CodePos::new(LINE, 2));
        open(view);
        say(
            view,
            "a list of 3 names opened under plate in line 12, each with its kind at the right, the first \
             row is marked",
        );
        let shown = from_main(move || (view.editor.completions_shown(), view.editor.marked_completion()));
        ensure!(shown == (true, Some(0)), "the list is {shown:?}");
        wait_for_next_frame();
        check_colors(LIST)?;

        inject_named_key(NamedKey::ArrowDown);
        inject_keys("a");
        say(
            view,
            "Down marked the second row, then the letter a went into the code after pl, the list stays",
        );
        let state = from_main(move || {
            (
                view.editor.marked_completion(),
                view.editor.lines()[LINE].clone(),
                view.editor.cursor(),
            )
        });
        ensure!(
            state == (Some(1), "plaate();".to_string(), CodePos::new(LINE, 3)),
            "after Down and a letter: {state:?}"
        );
        wait_for_next_frame();
        check_colors(SECOND)?;

        inject_named_key(NamedKey::Enter);
        let state = from_main(move || {
            (
                view.picked,
                view.editor.completions_shown(),
                view.editor.line_count(),
                view.editor.panel().is_ok(),
            )
        });
        ensure!(
            state == (Some(1), false, SCAD.lines().count(), false),
            "after Enter: {state:?}"
        );

        open(view);
        inject_named_key(NamedKey::ArrowUp);
        inject_named_key(NamedKey::Escape);
        let state = from_main(move || {
            (
                view.picked,
                view.editor.completions_shown(),
                view.editor.is_editing(),
            )
        });
        ensure!(state == (Some(1), false, true), "after Escape: {state:?}");

        from_main(move || {
            let note = Label::new();
            note.set_text("module plate()")
                .set_text_size(14)
                .set_alignment(TextAlignment::Center);
            note.set_color(NOTE).set_corner_radius(6);
            note.set_border_color(NOTE_BORDER).set_border_width(1);
            note.set_size(200, 36);
            view.editor.show_panel(CodePos::new(MODULE_LINE, 7), note);
            view.status.set_text(
                "Enter picked the second row and Escape closed a second list, both with no change of the \
                 code. Now a yellow note of the app floats under plate in line 4",
            );
        });
        wait_for_next_frame();
        let frame = from_main(move || {
            let panel = view.editor.panel();
            (
                panel.is_ok(),
                panel.absolute_frame().size.width,
                panel.absolute_frame().size.height,
            )
        });
        ensure!(frame == (true, 200.0, 36.0), "the panel is {frame:?}");
        check_colors(PANEL)?;

        let gone = from_main(move || {
            view.editor.hide_panel();
            view.editor.panel().is_ok()
        });
        ensure!(!gone, "the panel is still there");
        Ok(())
    }
}

/// Recorded with `--record-colors`.
const LIST: &str = r"
             592    4 - #ffffff
             396   12 - #ffffff
              80   24 - #6e7781
             112   24 - #6e7781
             168   24 - #747c86
             184   24 - #6e7781
             200   24 - #747c86
              76   44 - #282d33
              72   84 - #cf222e
              72   88 - #cf222e
              80   88 - #cf222e
              92   88 - #d02632
             140   88 - #8453e0
              88  108 - #8250df
             124  112 - #a17ce7
             144  112 - #8453e0
             140  132 - #8453e0
             180  132 - #282d33
             448  144 - #ffffff
              28  148 - #bbc1c8
             124  156 - #a17ce7
             168  156 - #8250df
             320  172 - #ffffff
             592  172 - #ffffff
             204  176 - #a17ce7
             284  176 - #5b5f63
              32  196 - #8d96a0
              72  264 - #8251df
             472  284 - #d3d3d3
             396  288 - #9facb6
             472  296 - #d3d3d3
              44  300 - #e8e8e8
             168  308 - #45494e
             472  308 - #d3d3d3
             144  312 - #3d4147
             396  312 - #a8aeb6
             472  320 - #d3d3d3
             472  336 - #d3d3d3
             384  340 - #ffffff
              56  348 - #acacac
             164  348 - #acacac
             204  348 - #acacac
             352  348 - #acacac
             428  348 - #acacac
             112  356 - #e8e8e8
             260  356 - #e8e8e8
             292  356 - #e8e8e8
             324  356 - #e8e8e8
             592  368 - #ffffff
              44  484 - #000000
             260  484 - #080808
              68  488 - #ffffff
             108  488 - #ffffff
             120  488 - #2b2b2b
             220  488 - #000000
             320  488 - #404040
             392  488 - #404040
             440  488 - #ffffff
             472  488 - #404040
             556  488 - #000000
              28  504 - #000000
             132  504 - #000000
             180  504 - #ffffff
             512  592 - #ffffff
";

/// Recorded with `--record-colors`.
const SECOND: &str = r"
             592    4 - #ffffff
             396   12 - #ffffff
              80   24 - #6f7881
             112   24 - #6f7881
             168   24 - #747c86
             184   24 - #6f7881
             200   24 - #747c86
              76   44 - #282d33
              72   84 - #cf222e
              72   88 - #cf222e
              80   88 - #cf222e
              92   88 - #d02632
             140   88 - #8453e0
              88  108 - #8250df
             124  112 - #a17ce7
             144  112 - #8453e0
             172  112 - #ffffff
             140  132 - #8453e0
             180  132 - #282d33
              28  148 - #bbc1c8
             456  152 - #ffffff
             124  156 - #a17ce7
             136  156 - #8251df
             168  156 - #8251df
             320  172 - #ffffff
             184  176 - #8453e0
             204  176 - #a17ce7
             284  176 - #5b5f63
              32  196 - #8d96a0
             592  216 - #ffffff
              80  264 - #252a30
              92  264 - #282d33
             236  276 - #d0d7de
             308  276 - #d0d7de
             460  276 - #d0d7de
             396  288 - #a8aeb6
              44  300 - #e8e8e8
             168  308 - #40474e
             144  312 - #394047
             396  312 - #9facb6
             384  340 - #ffffff
              56  348 - #acacac
             164  348 - #acacac
             212  348 - #acacac
             328  348 - #acacac
             424  348 - #acacac
             464  348 - #acacac
             112  356 - #e8e8e8
             272  356 - #e8e8e8
             592  376 - #ffffff
              28  484 - #2c2c2c
              72  484 - #000000
             212  484 - #000000
             380  484 - #000000
             504  484 - #000000
             112  488 - #ffffff
             148  488 - #ffffff
             184  488 - #ffffff
             272  488 - #000000
             352  488 - #000000
             440  488 - #272727
             520  488 - #ffffff
              44  504 - #000000
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const PANEL: &str = r"
             420    4 - #ffffff
             592    4 - #ffffff
              80   24 - #6f7881
             112   24 - #6f7881
             168   24 - #747c86
             184   24 - #6f7881
             200   24 - #747c86
              72   84 - #cf222e
              72   88 - #cf222e
              80   88 - #cf222e
              92   88 - #d02632
             128   88 - #8251df
             140   88 - #8453e0
             116  100 - #d4a72c
             148  100 - #d4a72c
             164  100 - #d4a72c
             180  100 - #d4a72c
             212  100 - #d4a72c
             256  100 - #d4a72c
             284  104 - #fff8c5
              88  108 - #8250df
             192  112 - #fff8c5
             232  112 - #fff8c5
             120  120 - #fff8c5
             204  120 - #fff8c5
             220  120 - #000000
             240  120 - #fcf5c3
             144  128 - #fff8c5
             168  132 - #fff8c5
             268  132 - #fff8c5
             300  132 - #fff8c5
             460  144 - #ffffff
              28  148 - #bbc1c8
             124  156 - #a17ce7
             144  156 - #8453e0
             168  156 - #8251df
             184  176 - #8453e0
             204  176 - #a17ce7
             284  176 - #5b5f63
             316  176 - #ffffff
              32  196 - #8d96a0
             556  252 - #ffffff
              92  264 - #282d33
              76  280 - #ffffff
              56  288 - #24292f
             104  288 - #272c32
             148  288 - #5b5f63
             400  320 - #ffffff
             268  364 - #ffffff
             180  484 - #000000
             524  484 - #000000
              36  488 - #343434
             124  488 - #1a1a1a
             352  488 - #ffffff
             416  488 - #343434
             448  488 - #ffffff
              68  492 - #181818
              96  500 - #ffffff
              36  504 - #000000
             172  504 - #000000
             248  504 - #ffffff
             308  504 - #ffffff
             492  504 - #1e1e1e
             560  504 - #000000
";
