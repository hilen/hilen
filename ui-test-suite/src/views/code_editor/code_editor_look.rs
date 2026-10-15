use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{CodeEditor, Label, Setup, Theme, ThemeMode, ViewData, ViewTest, WHITE, view},
    ui_test::{check_colors, set_record_probe_count},
};

use super::common::{SCAD, color_all, status_style};

/// The line of `SCAD` that calls a module nobody wrote.
const BAD_LINE: usize = 12;

/// A code editor with a `.scad` file: line numbers in the gutter, the
/// colors the app gave every line, a mark with a message on the line
/// with an error, and the lines of the hole lit as a picked part. Then
/// the same in the dark theme.
#[view]
struct CodeEditorLook {
    #[init]
    editor: CodeEditor,
    status: Label,
}

impl Setup for CodeEditorLook {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        self.editor.place().t(10).lr(10).h(380);
        self.editor.set_text(SCAD);
        color_all(self.editor);
        self.editor.set_mark(BAD_LINE, "unknown module undefined_part");
        self.editor.set_highlight(Some(6..8));

        status_style(self.status);
        self.status.place().t(400).lr(10).b(10);
    }
}

impl ViewTest for CodeEditorLook {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);
        let mark = from_main(move || {
            view.status.set_text(
                "light theme: numbers 1 to 13 in the gutter, the colors of the code, lines 7 and 8 lit \
                 yellow, line 13 red with its message",
            );
            view.editor.mark(BAD_LINE).map(str::to_string)
        });
        ensure!(
            mark.as_deref() == Some("unknown module undefined_part"),
            "the mark is {mark:?}"
        );
        wait_for_next_frame();
        check_colors(LIGHT)?;

        from_main(move || {
            Theme::set_mode(ThemeMode::Dark);
            view.set_color(hilen::ui::BLACK);
            view.status.set_text_color(WHITE);
            view.status.set_text("the same in the dark theme");
        });
        wait_for_next_frame();
        check_colors(DARK)?;

        Ok(())
    }
}

/// Recorded with `--record-colors`.
const LIGHT: &str = r"
             404    4 - #ffffff
             592    4 - #ffffff
              80   24 - #6f7881
             112   24 - #6f7881
             168   24 - #747c86
             200   24 - #747c86
              76   44 - #282d33
              28   48 - #ccd0d6
              32   84 - #d5d9dd
              56   88 - #eba1a6
             108   88 - #c7b1f1
             140   88 - #8453e0
             124  108 - #a17ce7
              88  112 - #8250df
             144  112 - #8453e0
             124  132 - #c1a9ef
             132  132 - #c6aff0
             180  132 - #282d33
             452  144 - #fff8c5
              28  148 - #bbc1c8
             204  152 - #fff8c5
             292  152 - #fff8c5
             124  156 - #a17ad8
             144  156 - #8453df
             172  176 - #fff8c5
             204  176 - #a17ad8
             284  176 - #5b5d55
             260  180 - #9bb5bc
             324  180 - #fff8c5
             396  184 - #fff8c5
             580  184 - #fff8c5
              32  196 - #8d96a0
              52  264 - #c7b1f1
             100  264 - #ffffff
              52  284 - #938b8d
             212  284 - #e7888d
             300  284 - #e7888d
             428  284 - #db545d
             100  288 - #9c9495
             120  288 - #24292f
             172  288 - #ffebe9
             240  288 - #d02531
             280  288 - #eb969a
             352  288 - #cf222e
             384  288 - #d02531
             424  288 - #cf232f
             428  288 - #db545d
             592  340 - #ffffff
             528  480 - #171717
             252  484 - #000000
             292  484 - #000000
             488  484 - #383838
              40  488 - #000000
             120  488 - #010101
             172  488 - #ffffff
             336  488 - #999999
             364  488 - #ffffff
             412  488 - #d7d7d7
             552  488 - #4c4c4c
              64  500 - #050505
              64  504 - #050505
              96  504 - #545454
             212  504 - #575757
             288  504 - #cccccc
";

/// Recorded with `--record-colors`.
const DARK: &str = r"
             436    4 - #000000
             160   20 - #0d1117
              80   24 - #8b939d
             112   24 - #8b939d
             196   24 - #464c54
              76   44 - #e2e9ef
              28   48 - #393f48
              72   84 - #ff7b72
              56   88 - #733e3e
             108   88 - #65557f
             140   88 - #cea5fa
             172  108 - #0d1117
              88  112 - #d2a8ff
             124  112 - #a182c5
             140  132 - #cea5fa
             180  132 - #e2e9ef
             452  144 - #3b2e00
             588  144 - #3b2e00
              28  148 - #474d57
             124  152 - #ac89bf
             156  152 - #3b2e00
             124  156 - #ac89bf
             180  176 - #7f6573
             204  176 - #ac89bf
             220  176 - #88846e
             284  176 - #bbbdb6
             260  180 - #546866
             324  180 - #3b2e00
             520  184 - #3b2e00
              32  196 - #6d7580
              52  264 - #65557f
              84  264 - #cea5fa
             504  276 - #3d1d1f
              52  284 - #918488
             120  284 - #e6edf3
             176  284 - #3d1d1f
             212  284 - #9d4b48
             300  284 - #9d4b48
             308  284 - #3d1d1f
             332  284 - #9d4b48
             400  284 - #ff7b72
             428  284 - #ce635d
             100  288 - #897b7f
             144  288 - #e5ecf2
             220  288 - #944744
             264  288 - #fc7a71
             280  288 - #8f4542
             352  288 - #ff7b72
             380  288 - #944744
             428  288 - #ce635d
             580  292 - #3d1d1f
              12  384 - #161b22
             560  444 - #000000
              32  492 - #b2b2b2
             416  492 - #000000
              28  496 - #ffffff
              44  496 - #adadad
              76  496 - #ffffff
             100  496 - #191919
             128  496 - #ffffff
             144  496 - #000000
             192  496 - #adadad
             212  496 - #ffffff
             592  592 - #000000
";
