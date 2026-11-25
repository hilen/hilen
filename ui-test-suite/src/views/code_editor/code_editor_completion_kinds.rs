use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        CodeEditor, CodePos, Completion, Label, NamedKey, Setup, ViewData, ViewFrame, ViewTest, WHITE, view,
    },
    ui_test::{check_colors, inject_named_key, set_record_probe_count},
};

use super::common::{SCAD, click_at, color_all, status_style};

/// The line with `plate();`.
const LINE: usize = 11;
/// The width of a list whose rows are short.
const LEAST_WIDTH: f32 = 420.0;
const CANVAS_WIDTH: f32 = 600.0;

const FUNCTION: &str = "kind_function.svg";
const FIELD: &str = "kind_field.svg";

/// The icons and the width of the list of completions. Each row has the
/// icon of its kind in front of its name, a function or a field, and the
/// row with no icon keeps its name in line with the others. The list is as
/// wide as its longest row needs, wider than a list of short rows, so each
/// signature at the right shows whole. Down marks the third row. Last a
/// list of short rows with no icon has the least width.
#[view]
struct CodeEditorCompletionKinds {
    #[init]
    editor: CodeEditor,
    status: Label,
}

impl Setup for CodeEditorCompletionKinds {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);
        self.editor.place().t(10).lr(10).h(380);
        self.editor.set_text(SCAD);
        color_all(self.editor);

        status_style(self.status);
        self.status.place().t(400).lr(10).b(10);
    }
}

fn rows(rows: &[(&str, &str, &str)]) -> Vec<Completion> {
    rows.iter()
        .map(|(label, detail, icon)| Completion {
            label:  (*label).to_string(),
            note:   String::new(),
            detail: (*detail).to_string(),
            icon:   (*icon).to_string(),
        })
        .collect()
}

fn kinds() -> Vec<Completion> {
    rows(&[
        ("abs", "const fn(self) -> i32", FUNCTION),
        ("abs_diff", "const fn(self, i32) -> u32", FUNCTION),
        ("saturating_sub_unsigned", "const fn(self, u32) -> i32", FUNCTION),
        ("len", "usize", FIELD),
        ("plain", "no icon", ""),
    ])
}

fn short() -> Vec<Completion> {
    rows(&[("plate", "module", ""), ("place", "function", "")])
}

fn open(view: Weak<CodeEditorCompletionKinds>, items: Vec<Completion>) -> f32 {
    from_main(move || {
        view.editor.show_completions(CodePos::new(LINE, 0), items);
    });
    wait_for_next_frame();
    from_main(move || view.editor.panel().absolute_frame().size.width)
}

fn say(view: Weak<CodeEditorCompletionKinds>, text: &'static str) {
    from_main(move || {
        view.status.set_text(text);
    });
}

impl ViewTest for CodeEditorCompletionKinds {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);
        click_at(view.editor, CodePos::new(LINE, 2));
        let width = open(view, kinds());
        ensure!(
            width > LEAST_WIDTH && width < CANVAS_WIDTH,
            "the list with long rows is {width} wide"
        );
        say(
            view,
            "a list of 5 names: 3 functions and 1 field with the icon of their kind, plain with no icon and \
             its name in line. The list is wider than 420, every signature at the right shows whole",
        );
        wait_for_next_frame();
        check_colors(KINDS)?;

        inject_named_key(NamedKey::ArrowDown);
        inject_named_key(NamedKey::ArrowDown);
        let marked = from_main(move || view.editor.marked_completion());
        ensure!(marked == Some(2), "the marked row is {marked:?}");
        say(
            view,
            "Down twice marked the third row, the longest one, its icon and its whole signature still show",
        );
        wait_for_next_frame();
        check_colors(MARKED)?;

        inject_named_key(NamedKey::Escape);
        let width = open(view, short());
        ensure!(
            (width - LEAST_WIDTH).abs() < f32::EPSILON,
            "the list with short rows is {width} wide"
        );
        Ok(())
    }
}

/// Recorded with `--record-colors`.
const KINDS: &str = r"
             592    4 - #ffffff
             396   12 - #ffffff
              80   24 - #6f7881
             112   24 - #6f7881
             168   24 - #747c86
             200   24 - #747c86
              72   84 - #cf222e
              72   88 - #cf232f
              80   88 - #cf222e
              92   88 - #d02632
             140   88 - #8453e0
              88  108 - #8250df
             124  112 - #a17ce7
             144  112 - #8453e0
             496  136 - #ffffff
              28  148 - #bbc1c8
             124  156 - #a17ce7
             168  156 - #8251df
             320  172 - #ffffff
             204  176 - #a17ce7
             284  176 - #5b5f63
              32  196 - #8d96a0
             592  200 - #ffffff
              84  264 - #8453e0
             436  288 - #ddf4ff
             504  288 - #8f98a2
             520  292 - #aab8c2
             136  312 - #52565b
             360  312 - #959da7
              44  316 - #e8e8e8
             520  316 - #b6bcc2
             116  336 - #282d33
             228  336 - #24292f
             276  336 - #ffffff
             360  336 - #959da7
             504  336 - #9099a2
             552  348 - #dadada
             100  360 - #24292f
             484  360 - #a7aeb5
             512  364 - #b6bcc2
              44  380 - #e8e8e8
             108  384 - #676a6f
             284  396 - #acacac
             332  396 - #acacac
             372  396 - #acacac
             412  396 - #acacac
             456  396 - #acacac
             544  400 - #cccccc
             164  404 - #e8e8e8
             236  404 - #e8e8e8
              40  472 - #000000
             260  472 - #050505
             560  476 - #000000
             116  480 - #ffffff
             392  492 - #ffffff
             188  496 - #333333
             216  496 - #525252
             320  496 - #ffffff
             432  496 - #525252
             464  496 - #212121
             516  496 - #525252
              72  516 - #000000
             144  516 - #000000
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const MARKED: &str = r"
             404    4 - #ffffff
             592    4 - #ffffff
              80   24 - #6f7881
             112   24 - #6f7881
             168   24 - #747c86
             200   24 - #747c86
              76   44 - #282d33
              72   84 - #cf222e
              72   88 - #cf232f
              80   88 - #cf222e
              92   88 - #d02632
             140   88 - #8453e0
              88  108 - #8250df
             172  108 - #ffffff
             124  112 - #a17ce7
             144  112 - #8453e0
             140  132 - #8453e0
             492  136 - #ffffff
              28  148 - #bbc1c8
             124  156 - #a17ce7
             168  156 - #8251df
             320  172 - #ffffff
             204  176 - #a17ce7
             284  176 - #5b5f63
              32  196 - #8d96a0
             592  200 - #ffffff
              72  264 - #8251df
             196  276 - #d0d7de
             304  276 - #d0d7de
             392  276 - #d0d7de
             456  284 - #ffffff
             108  288 - #ffffff
             504  288 - #9099a2
             520  292 - #b6bcc2
             136  308 - #52565b
             136  312 - #52565b
             360  312 - #959da7
             520  316 - #b6bcc2
             116  336 - #282d33
             228  336 - #24292f
             276  336 - #ddf4ff
             360  336 - #929ca7
             504  336 - #8f98a2
             100  360 - #24292f
             484  360 - #a7aeb5
             512  364 - #b6bcc2
             108  384 - #676a6f
              56  396 - #acacac
             252  396 - #acacac
             316  396 - #acacac
             460  396 - #acacac
             540  396 - #acacac
             180  404 - #e8e8e8
             388  404 - #e8e8e8
             172  484 - #000000
             124  488 - #000000
             208  488 - #191919
             300  488 - #ffffff
             384  488 - #3b3b3b
             444  488 - #000000
             508  488 - #010101
              36  504 - #080808
              80  504 - #000000
             592  592 - #ffffff
";
