use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        BLACK, CodeEditor, Color, DynamicColor, Label, MarkKind, Setup, Theme, ThemeMode, UIColor, ViewData,
        ViewTest, WHITE, view,
    },
    ui_test::{check_colors, set_record_probe_count},
};

use super::common::{SCAD, color_all, status_style};

const RED: UIColor = UIColor::Dynamic(DynamicColor::new(Color::hex("#cf222e"), Color::hex("#ff7b72")));
const YELLOW: UIColor = UIColor::Dynamic(DynamicColor::new(Color::hex("#9a6700"), Color::hex("#d29922")));
const SAME_NAME: UIColor = UIColor::Dynamic(DynamicColor::new(Color::hex("#cfe3ff"), Color::hex("#1f3a5f")));

/// The line with `size = [40, 30, 10];` and the 4 bytes of the name.
const SIZE_LINE: usize = 1;
/// The line with `cube(size);` and where `size` is on it.
const CUBE_LINE: usize = 5;
/// The line with `cylinder(h = 12, r = 5);` and where `r = 5` is on it.
const RADIUS_LINE: usize = 7;
/// The line with `plate();`, 8 bytes long.
const CALL_LINE: usize = 11;
const BAD_LINE: usize = 12;

/// A code editor with what a language server puts on the code: a mark of
/// each kind, an error, a warning and a note, each in its own colors, a
/// line under the exact place of a problem, also one with no width at the
/// end of a line, and a color behind the 2 uses of a name. Then the same in
/// the dark theme, then with the lines and the colors taken off again.
#[view]
struct CodeEditorDecor {
    #[init]
    editor: CodeEditor,
    status: Label,
}

impl Setup for CodeEditorDecor {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);
        self.editor.place().t(10).lr(10).h(380);
        self.editor.set_text(SCAD);
        color_all(self.editor);

        self.editor
            .set_mark_kind(BAD_LINE, MarkKind::Error, "unknown module undefined_part");
        self.editor.set_underlines(BAD_LINE, [(0..14, RED)]);
        self.editor
            .set_mark_kind(RADIUS_LINE, MarkKind::Warning, "r is smaller than the wall");
        self.editor.set_underlines(RADIUS_LINE, [(29..34, YELLOW)]);
        self.editor.set_mark_kind(SIZE_LINE, MarkKind::Note, "used in 2 places");
        self.editor.set_background_runs(SIZE_LINE, [(0..4, SAME_NAME)]);
        self.editor.set_background_runs(CUBE_LINE, [(13..17, SAME_NAME)]);
        self.editor.set_underlines(CALL_LINE, [(8..8, RED)]);

        status_style(self.status);
        self.status.place().t(400).lr(10).b(10);
    }
}

impl ViewTest for CodeEditorDecor {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);
        let read = from_main(move || {
            view.status.set_text(
                "light theme: line 2 blue with a note and size on blue, size on blue in line 6, line 8 \
                 yellow with a warning and a yellow line under r = 5, a red line 1 char wide after \
                 plate();, line 13 red with an error and a red line under undefined_part",
            );
            (
                view.editor.mark_kind(RADIUS_LINE),
                view.editor.mark(SIZE_LINE).map(str::to_string),
                view.editor.underlines(BAD_LINE).to_vec(),
                view.editor.background_runs(CUBE_LINE).to_vec(),
            )
        });
        ensure!(
            read.0 == Some(MarkKind::Warning),
            "the kind of the mark is {:?}",
            read.0
        );
        ensure!(
            read.1.as_deref() == Some("used in 2 places"),
            "the note is {:?}",
            read.1
        );
        ensure!(read.2 == vec![(0..14, RED)], "the underlines are {:?}", read.2);
        ensure!(
            read.3 == vec![(13..17, SAME_NAME)],
            "the backgrounds are {:?}",
            read.3
        );
        wait_for_next_frame();
        check_colors(LIGHT)?;

        from_main(move || {
            Theme::set_mode(ThemeMode::Dark);
            view.set_color(BLACK);
            view.status.set_text_color(WHITE);
            view.status.set_text("the same in the dark theme");
        });
        wait_for_next_frame();
        check_colors(DARK)?;

        let left = from_main(move || {
            let mut editor = view.editor;
            editor.clear_underlines();
            editor.clear_background_runs();
            view.status
                .set_text("the lines under the code and the colors behind size are gone, the 3 marks stay");
            (
                editor.underlines(BAD_LINE).len(),
                editor.background_runs(SIZE_LINE).len(),
                editor.mark_kind(BAD_LINE),
            )
        });
        ensure!(left == (0, 0, Some(MarkKind::Error)), "after the clear: {left:?}");
        wait_for_next_frame();
        check_colors(CLEARED)?;

        from_main(|| Theme::set_mode(ThemeMode::System));
        Ok(())
    }
}

/// Recorded with `--record-colors`.
const LIGHT: &str = r"
             592    4 - #ffffff
              80   24 - #6f7881
             112   24 - #6f7881
             168   24 - #747c86
             200   24 - #747c86
              52   36 - #cfe3ff
              64   40 - #cfe3ff
             264   40 - #0969da
              76   44 - #272d33
             240   44 - #0969da
             288   44 - #0c6bdb
             348   44 - #0d6cdb
             476   52 - #ddf4ff
             140   88 - #8453e0
              88  108 - #8250df
             124  112 - #a17ce7
             156  124 - #cfe3ff
             184  124 - #cfe3ff
             128  132 - #8250df
             164  132 - #cfe3ff
             168  132 - #cfe3ff
              28  148 - #bbc1c8
             124  156 - #a17ce7
             168  156 - #8251df
             204  176 - #a178dd
             364  176 - #b38a36
             424  176 - #9a6801
             452  176 - #b38a36
             480  176 - #9e6c08
             496  176 - #9c6903
             524  176 - #9c6a04
             552  176 - #9a6801
             324  180 - #fff1d6
             284  184 - #9a6700
             296  184 - #9a6700
             304  184 - #9a6700
             316  184 - #9a6700
              32  196 - #8d96a0
              72  264 - #8251df
             116  272 - #cf222e
             336  284 - #cf222e
             148  288 - #5b5a5e
             240  288 - #d02531
             296  288 - #cf222e
             384  288 - #d02531
             428  288 - #db545d
             592  336 - #ffffff
              44  472 - #000000
             212  472 - #1a1a1a
             160  476 - #ffffff
             468  476 - #000000
             328  492 - #383838
             252  496 - #ffffff
             296  496 - #525252
             408  496 - #525252
             516  496 - #050505
             564  496 - #525252
              36  516 - #383838
              84  516 - #050505
             144  516 - #ffffff
             188  516 - #191919
             368  516 - #000000
             456  516 - #ffffff
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const DARK: &str = r"
             160   20 - #0d1117
             112   24 - #8b949e
             184   24 - #8b949e
             200   24 - #868f99
             500   32 - #12233a
              68   36 - #1f3a5f
              80   36 - #1f3a5f
              60   44 - #1f3a5f
             240   44 - #58a6ff
             264   44 - #58a6ff
             288   44 - #57a4fc
             348   44 - #57a3fb
              72   84 - #ff7b72
             128   88 - #d1a7fe
             156   88 - #0d1117
              88  112 - #d2a8ff
             124  112 - #a182c5
             156  124 - #1f3a5f
             164  132 - #1f3a5f
             168  132 - #1f3a5f
             180  132 - #e2e9f0
              28  148 - #474d57
             292  152 - #0d1117
             124  156 - #a182c5
             244  164 - #3a2c12
             368  164 - #3a2c12
             196  176 - #cfa6fa
             204  176 - #ac89c4
             284  176 - #bbbdbb
             424  176 - #d19922
             452  176 - #ac7e1e
             480  176 - #cc9521
             496  176 - #d09722
             520  176 - #cc9521
             552  176 - #d19922
              80  180 - #3a2c12
             160  180 - #3a2c12
             288  184 - #d29922
             300  184 - #d29922
             320  184 - #d29922
              32  196 - #6d7580
              84  264 - #cea5fa
             120  272 - #ff7b72
             180  284 - #3d1d1f
             308  284 - #3d1d1f
              56  288 - #e6edf3
             104  288 - #e3eaf0
             148  288 - #bcb9be
             240  288 - #fc7a71
             336  288 - #ff7b72
             368  288 - #3d1d1f
             428  288 - #ce635d
             544  292 - #3d1d1f
              12  372 - #161b22
             592  396 - #000000
              28  492 - #ffffff
             164  492 - #d9d9d9
             416  492 - #000000
              44  496 - #adadad
              76  496 - #ffffff
             116  496 - #000000
             192  496 - #adadad
             212  496 - #ffffff
             592  592 - #000000
";

/// Recorded with `--record-colors`.
const CLEARED: &str = r"
             160   20 - #0d1117
              80   24 - #8b949e
             112   24 - #8b949e
             184   24 - #8b949e
             200   24 - #868f99
             500   32 - #12233a
             264   40 - #58a6ff
             252   44 - #57a3fb
             288   44 - #57a4fc
             348   44 - #57a3fb
             588   52 - #12233a
              72   88 - #ff7b72
             140   88 - #cea5fa
              88  112 - #d2a8ff
             124  112 - #a182c5
             172  112 - #0d1117
             120  132 - #0d1117
              28  148 - #474d57
             124  156 - #a182c5
             168  156 - #d2a8ff
             244  164 - #3a2c12
             324  164 - #3a2c12
             456  164 - #3a2c12
             204  176 - #ac89c4
             284  176 - #bbbdbb
             364  176 - #ac7e1e
             424  176 - #d19922
             444  176 - #cf9722
             452  176 - #ac7e1e
             480  176 - #cc9521
             496  176 - #d09722
             520  176 - #cc9521
             552  176 - #d19922
              80  180 - #3a2c12
              32  196 - #6d7580
              72  264 - #d1a7fe
             532  280 - #3d1d1f
             180  284 - #3d1d1f
             308  284 - #3d1d1f
              56  288 - #e6edf3
              88  288 - #3d1d1f
             120  288 - #e6edf3
             148  288 - #bcb9be
             240  288 - #fc7a71
             276  288 - #3d1d1f
             336  288 - #ff7b72
             364  288 - #3d1d1f
             424  288 - #ff7b72
             428  288 - #ce635d
             592  368 - #000000
              12  372 - #161b22
             480  388 - #0d1117
             572  480 - #000000
              28  492 - #ffffff
             276  492 - #ffffff
             100  496 - #ffffff
             136  496 - #fafafa
             192  496 - #adadad
             248  496 - #adadad
             308  496 - #000000
             348  496 - #000000
             396  496 - #ffffff
             460  496 - #fafafa
             592  592 - #000000
";
