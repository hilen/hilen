use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        CodeEditor, CodePos, Label, NamedKey, Setup, View, ViewData, ViewFrame, ViewSubviews, ViewTest,
        WHITE, WeakView, view,
    },
    ui_test::{check_colors, inject_named_key, set_record_probe_count},
};

use super::common::{click_at, runs, status_style};

const LINES: usize = 10_000;
/// The line that is far longer than the editor is wide.
const LONG: usize = 4999;
/// Fewer views than this means only the lines on screen are views.
const MOST_LABELS: usize = 300;

fn long_file() -> String {
    (0..LINES)
        .map(|index| {
            if index == LONG {
                format!("translate([{index}, 0, 0]) {}", "cube(1); ".repeat(20))
            } else {
                format!("translate([{index}, 0, 0]) cube(1);")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The labels under `view` that show exactly `text` and are not hidden.
fn number_labels(view: WeakView, text: &str) -> usize {
    if view.is_hidden() {
        return 0;
    }
    let own = usize::from(view.downcast_view::<Label>().is_some_and(|label| label.text() == text));
    own + view.subviews().iter().map(|sub| number_labels(sub.weak(), text)).sum::<usize>()
}

fn labels(view: WeakView) -> usize {
    let own = usize::from(view.downcast_view::<Label>().is_some());
    own + view.subviews().iter().map(|sub| labels(sub.weak())).sum::<usize>()
}

/// A file of 10 000 lines. Only the lines on screen are views, the gutter
/// is as wide as 5 digits, a jump to line 5000 scrolls there, and End on
/// a line longer than the editor scrolls the code sideways while the
/// gutter stays.
#[view]
struct CodeEditorLongFile {
    #[init]
    editor: CodeEditor,
    status: Label,
}

impl Setup for CodeEditorLongFile {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        self.editor.place().t(10).lr(10).h(400);
        self.editor.set_text(&long_file());
        for index in 0..LINES {
            let line = self.editor.lines()[index].clone();
            self.editor.set_color_runs(index, runs(&line));
        }

        status_style(self.status);
        self.status.place().t(420).lr(10).b(10);
    }
}

impl ViewTest for CodeEditorLongFile {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);
        wait_for_next_frame();

        let count = from_main(move || {
            view.status.set_text("10 000 lines, the first ones on screen");
            labels(view.editor.weak_view())
        });
        ensure!(count < MOST_LABELS, "{count} labels for 10 000 lines");

        from_main(move || {
            let mut editor = view.editor;
            editor.set_cursor(CodePos::new(LONG, 0));
            view.status.set_text("the cursor set to line 5000, it scrolls there");
        });
        wait_for_next_frame();
        let numbers = from_main(move || number_labels(view.editor.weak_view(), "5000"));
        ensure!(numbers == 1, "{numbers} gutter labels show 5000");
        check_colors(LINE_5000)?;

        click_at(view.editor, CodePos::new(LONG, 0));
        inject_named_key(NamedKey::End);
        let (cursor, point, width) = from_main(move || {
            view.status
                .set_text("End on the long line 5000, the code moves left, the gutter stays");
            let cursor = view.editor.cursor();
            (cursor, view.editor.point_of(cursor), view.editor.width())
        });
        wait_for_next_frame();
        ensure!(cursor.byte > 200, "End put the cursor at {cursor:?}");
        let point = point.ok_or_else(|| anyhow::anyhow!("line 5000 is not on screen"))?;
        ensure!(
            point.x > 0.0 && point.x < width,
            "the caret is at {} out of the width {width}",
            point.x
        );
        check_colors(SIDEWAYS)?;

        Ok(())
    }
}

/// Recorded with `--record-colors`.
const LINE_5000: &str = r"
             500    4 - #ffffff
             176   12 - #0852af
             280   12 - #8250df
             100   16 - #c7b0f1
              32   56 - #d5d9dd
              84   60 - #a17ce7
             292   80 - #8453e0
             592   84 - #ffffff
             176  100 - #0852af
              32  120 - #d5d9dd
              84  124 - #a17ce7
             252  124 - #ffffff
              32  188 - #d5d9dd
              84  188 - #a17ce7
             168  188 - #b1c8e6
             252  188 - #ffffff
             420  188 - #ffffff
             584  208 - #a6a6a6
             184  232 - #0852af
              84  236 - #a17ce7
             252  256 - #ffffff
              32  276 - #d5d9dd
             192  276 - #b1c8e6
             292  300 - #8453e0
              84  320 - #a17ce7
             184  320 - #0852af
              32  364 - #d5d9dd
             176  364 - #0852af
              28  384 - #8c959f
              44  388 - #8c959f
             272  388 - #ffffff
             364  388 - #8453e0
             428  388 - #c6aff0
             492  388 - #c1a9ef
             580  388 - #8453e0
              16  404 - #a0a1a3
              52  404 - #a0a1a3
              80  404 - #a6a6a6
             104  404 - #a6a6a6
             128  404 - #a6a6a6
             148  404 - #a6a6a6
             200  404 - #a6a6a6
              32  500 - #4d4d4d
             148  500 - #545454
             180  500 - #8a8a8a
             264  500 - #080808
             268  500 - #212121
              44  504 - #ffffff
             148  504 - #545454
             152  504 - #6e6e6e
             200  504 - #000000
             208  504 - #bbbbbb
             220  504 - #212121
             268  504 - #212121
             284  504 - #000000
              28  508 - #000000
              68  508 - #606060
             148  508 - #545454
             152  508 - #6e6e6e
             160  508 - #191919
             220  508 - #212121
             268  508 - #212121
             424  592 - #ffffff
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const SIDEWAYS: &str = r"
             356    4 - #ffffff
             564    4 - #ffffff
              32   12 - #d5d9dd
             204   16 - #ffffff
              56   52 - #bbc1c8
              32   76 - #d5d9dd
              56   96 - #bbc1c8
              32  120 - #d5d9dd
             424  160 - #ffffff
              32  164 - #d5d9dd
             256  184 - #ffffff
              56  192 - #c2c7cd
             584  208 - #a6a6a6
              52  232 - #f6f8fa
              56  272 - #bbc1c8
              56  316 - #bbc1c8
              32  320 - #d5d9dd
              28  384 - #8c959f
              44  388 - #8c959f
              76  388 - #c1a9ef
              92  388 - #8453e0
             152  388 - #8250df
             164  388 - #8453e0
             216  388 - #ffffff
             228  388 - #c6aff0
             236  388 - #8453e0
             292  388 - #c1a9ef
             300  388 - #c6aff0
             308  388 - #8453e0
             368  388 - #8250df
             380  388 - #8453e0
             436  388 - #c1a9ef
             444  388 - #c6aff0
             516  388 - #c6aff0
             404  392 - #ffffff
             468  392 - #ffffff
             548  392 - #ffffff
             396  404 - #a6a6a6
             424  404 - #a6a6a6
             480  404 - #a6a6a6
             500  404 - #a6a6a6
             528  404 - #a6a6a6
             576  404 - #a6a6a6
              44  408 - #8c959f
              32  500 - #a4a4a4
             264  500 - #d1d1d1
              48  504 - #ffffff
             120  504 - #000000
             192  504 - #ffffff
             284  504 - #000000
             396  504 - #121212
             424  504 - #000000
              36  508 - #343434
              68  508 - #747474
              72  508 - #4c4c4c
             124  508 - #b2b2b2
             224  508 - #000000
             280  508 - #a9a9a9
             284  508 - #171717
             344  508 - #252525
             372  508 - #8d8d8d
             424  508 - #010101
             452  508 - #000000
             592  592 - #ffffff
";
