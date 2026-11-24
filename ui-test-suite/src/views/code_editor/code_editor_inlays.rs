use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        BLACK, CodeEditor, CodePos, Color, DynamicColor, Inlay, Label, MarkKind, NamedKey, Point, Setup,
        Theme, ThemeMode, UIColor, ViewData, ViewTest, WHITE, view,
    },
    ui_test::{
        check_colors, checkpoint, inject_keys, inject_named_key, inject_touches, set_record_probe_count,
    },
};

use super::common::{SCAD, click_at, color_all, command, drag_over, screen_point, status_style};
use crate::text_points::click;

const RED: UIColor = UIColor::Dynamic(DynamicColor::new(Color::hex("#cf222e"), Color::hex("#ff7b72")));
const SAME_NAME: UIColor = UIColor::Dynamic(DynamicColor::new(Color::hex("#cfe3ff"), Color::hex("#1f3a5f")));

/// The line with `size = [40, 30, 10];`, the name ends at byte 4.
const SIZE_LINE: usize = 1;
/// The line with `cube(size);`, the argument starts at byte 13.
const CUBE_LINE: usize = 5;
/// The line with `translate([20, 15, -1])`, the argument starts at byte 18.
const MOVE_LINE: usize = 6;
/// The line with `plate();`, 8 bytes long.
const CALL_LINE: usize = 11;

/// A code editor with texts inside its lines that are no part of the file,
/// the way a language server shows a type or the name of a parameter:
/// `: vec3` after a name, `size: ` and `v: ` in front of an argument, and
/// ` -> part` at the end of a line, before the message of its mark. They
/// are gray on a gray ground. A press on one puts the caret at its byte, a
/// typed char moves it with its code, a selection goes over it and leaves
/// it out of the text, and a change over 2 lines drops it.
#[view]
struct CodeEditorInlays {
    #[init]
    editor: CodeEditor,
    status: Label,
}

fn size_type() -> Inlay {
    Inlay::new(4, ": vec3")
}

fn cube_name() -> Inlay {
    Inlay::leading(13, "size: ")
}

fn move_name() -> Inlay {
    Inlay::leading(18, "v: ")
}

fn call_type() -> Inlay {
    Inlay::new(8, " -> part")
}

impl Setup for CodeEditorInlays {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);
        self.editor.place().t(10).lr(10).h(380);
        self.editor.set_text(SCAD);
        color_all(self.editor);
        self.editor.changed.val(move |_| color_all(self.editor));

        self.editor.set_inlays(SIZE_LINE, [size_type()]);
        self.editor.set_inlays(CUBE_LINE, [cube_name()]);
        self.editor.set_inlays(MOVE_LINE, [move_name()]);
        self.editor.set_inlays(CALL_LINE, [call_type()]);
        self.editor.set_background_runs(SIZE_LINE, [(0..4, SAME_NAME)]);
        self.editor.set_underlines(CUBE_LINE, [(13..17, RED)]);
        self.editor.set_mark_kind(CALL_LINE, MarkKind::Note, "1 call");

        status_style(self.status);
        self.status.place().t(400).lr(10).b(10);
    }
}

fn say(view: Weak<CodeEditorInlays>, text: &'static str) {
    from_main(move || {
        view.status.set_text(text);
    });
}

fn cursor(view: Weak<CodeEditorInlays>) -> CodePos {
    from_main(move || view.editor.cursor())
}

fn inlays(view: Weak<CodeEditorInlays>, line: usize) -> Vec<Inlay> {
    from_main(move || view.editor.inlays(line).to_vec())
}

fn line(view: Weak<CodeEditorInlays>, line: usize) -> String {
    from_main(move || view.editor.lines()[line].clone())
}

impl ViewTest for CodeEditorInlays {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);
        let text = from_main(move || view.editor.text());
        ensure!(text == SCAD, "the inlays are in the text:\n{text}");
        let read = inlays(view, SIZE_LINE);
        ensure!(read == vec![size_type()], "the inlays of line 2 are {read:?}");
        say(
            view,
            "light theme, gray on gray and no part of the file: `: vec3` after size in line 2 where the blue \
             ground ends at size, `size: ` in cube( in line 6 where the red line starts at size, `v: ` in \
             translate( in line 7, ` -> part` after plate(); with the note 1 call behind it",
        );
        wait_for_next_frame();
        check_colors(LIGHT)?;

        click_at(view.editor, CodePos::new(SIZE_LINE, 7));
        let at = cursor(view);
        ensure!(
            at == CodePos::new(SIZE_LINE, 7),
            "a press on [ behind the inlay put the caret at {at:?}"
        );
        // The middle of `: vec3`, 3 chars behind the end of the name.
        let (on_inlay, found_on_inlay, found_on_code) = from_main(move || {
            let editor = view.editor;
            let caret = |byte| editor.point_of(CodePos::new(SIZE_LINE, byte)).expect("line 2 is laid out");
            let char_width = caret(1).x - caret(0).x;
            let name_end = screen_point(editor, CodePos::new(SIZE_LINE, 4));
            let local = Point::new(caret(4).x + char_width * 3.0, caret(4).y);
            let on_code = Point::new(caret(7).x + 1.0, caret(7).y);
            (
                (name_end.0 + char_width * 3.0, name_end.1),
                editor.text_at(local),
                editor.text_at(on_code),
            )
        });
        ensure!(
            found_on_inlay.is_none(),
            "a point on the inlay is the text at {found_on_inlay:?}"
        );
        ensure!(
            found_on_code == Some(CodePos::new(SIZE_LINE, 7)),
            "a point on [ is the text at {found_on_code:?}"
        );
        inject_touches(click(on_inlay));
        wait_for_next_frame();
        let at = cursor(view);
        ensure!(
            at == CodePos::new(SIZE_LINE, 4),
            "a press on the inlay put the caret at {at:?}"
        );
        say(
            view,
            "a press on `: vec3` put the caret in front of it, at the end of size",
        );
        checkpoint("the caret stands between size and `: vec3`")?;

        check_edits(view)?;

        from_main(move || {
            view.editor.set_inlays(MOVE_LINE, [move_name()]);
            Theme::set_mode(ThemeMode::Dark);
            view.set_color(BLACK);
            view.status.set_text_color(WHITE);
            view.status
                .set_text("the app set `v: ` again, and the same in the dark theme: light gray on dark gray");
        });
        wait_for_next_frame();
        check_colors(DARK)?;

        let left = from_main(move || {
            let mut editor = view.editor;
            editor.clear_inlays();
            view.status
                .set_text("the inlays are gone, every line stands as it is in the file");
            (0..editor.line_count()).map(|line| editor.inlays(line).len()).sum::<usize>()
        });
        ensure!(left == 0, "{left} inlays are left after the clear");
        wait_for_next_frame();
        check_colors(CLEARED)?;

        from_main(|| Theme::set_mode(ThemeMode::System));
        Ok(())
    }
}

/// Typing, Backspace, a selection, Enter and undo around the inlays.
fn check_edits(view: Weak<CodeEditorInlays>) -> Result<()> {
    inject_keys("s");
    let (text, read, at) = (line(view, SIZE_LINE), inlays(view, SIZE_LINE), cursor(view));
    ensure!(text == "sizes = [40, 30, 10];", "line 2 is {text:?}");
    ensure!(
        read == vec![Inlay::new(5, ": vec3")],
        "after the s the inlays of line 2 are {read:?}"
    );
    ensure!(at == CodePos::new(SIZE_LINE, 5), "the caret is at {at:?}");

    click_at(view.editor, CodePos::new(CUBE_LINE, 13));
    inject_keys("2");
    let (text, read, at) = (line(view, CUBE_LINE), inlays(view, CUBE_LINE), cursor(view));
    ensure!(text == "        cube(2size);", "line 6 is {text:?}");
    ensure!(
        read == vec![cube_name()],
        "after the 2 the inlays of line 6 are {read:?}"
    );
    ensure!(at == CodePos::new(CUBE_LINE, 14), "the caret is at {at:?}");
    say(
        view,
        "typed s behind size and 2 in front of the argument of cube: line 2 reads sizes: vec3, line 6 \
         reads cube(size: 2size) with the caret behind the 2. The blue ground and the red line went with \
         the change",
    );
    wait_for_next_frame();
    check_colors(TYPED)?;

    inject_named_key(NamedKey::Backspace);
    click_at(view.editor, CodePos::new(SIZE_LINE, 5));
    inject_named_key(NamedKey::Backspace);
    let text = from_main(move || view.editor.text());
    ensure!(text == SCAD, "2 Backspace did not give the text back:\n{text}");
    let (size, cube) = (inlays(view, SIZE_LINE), inlays(view, CUBE_LINE));
    ensure!(
        size == vec![size_type()],
        "after Backspace the inlays of line 2 are {size:?}"
    );
    ensure!(
        cube == vec![cube_name()],
        "after Backspace the inlays of line 6 are {cube:?}"
    );

    drag_over(
        view.editor,
        CodePos::new(SIZE_LINE, 0),
        CodePos::new(SIZE_LINE, 9),
    );
    let selected = from_main(move || view.editor.selected_text());
    ensure!(selected == "size = [4", "the selected text is {selected:?}");
    say(
        view,
        "2 Backspace gave the text back. A drag from the start of line 2 to behind [4 selected size = \
         [4, the selection goes over `: vec3`, which is no part of the selected text",
    );
    wait_for_next_frame();
    check_colors(SELECTED)?;

    click_at(view.editor, CodePos::new(MOVE_LINE, 18));
    inject_named_key(NamedKey::Enter);
    let (first, second, moved) = (
        inlays(view, MOVE_LINE),
        inlays(view, MOVE_LINE + 1),
        inlays(view, CALL_LINE + 1),
    );
    ensure!(
        first.is_empty() && second.is_empty(),
        "Enter in line 7 left the inlays {first:?} and {second:?}"
    );
    ensure!(
        moved == vec![call_type()],
        "the inlay of plate(); moved to {moved:?}"
    );
    command('z');
    let text = from_main(move || view.editor.text());
    ensure!(text == SCAD, "undo did not give the text back:\n{text}");
    let (first, call) = (inlays(view, MOVE_LINE), inlays(view, CALL_LINE));
    ensure!(first.is_empty(), "undo brought the inlays {first:?} back");
    ensure!(
        call == vec![call_type()],
        "after undo the inlay of plate(); is {call:?}"
    );
    say(
        view,
        "Enter in front of the argument of translate and undo: the text is as before, `v: ` in line 7 is \
         gone until the app sets it again, ` -> part` is back in line 12",
    );
    checkpoint("line 7 has no `v: `, the other 3 inlays stand")?;
    Ok(())
}

/// Recorded with `--record-colors`.
const LIGHT: &str = r"
             404    4 - #ffffff
             112   24 - #6f7881
             168   24 - #747c86
             184   24 - #6f7881
             200   24 - #747c86
              52   36 - #cfe3ff
              64   36 - #cfe3ff
              80   36 - #cfe3ff
              56   40 - #cfe3ff
              72   40 - #cfe3ff
              80   40 - #cfe3ff
              60   44 - #cfe3ff
              64   44 - #cfe3ff
             108   44 - #687078
             592   76 - #ffffff
              72   84 - #cf222e
              72   88 - #cf232f
              80   88 - #cf222e
              92   88 - #d02632
             140   88 - #8453e0
              88  108 - #8250df
             124  112 - #a17ce7
             140  132 - #8453e0
             180  132 - #687078
             204  140 - #cf222e
             212  140 - #cf222e
             220  140 - #cf222e
             224  140 - #cf222e
             228  140 - #cf222e
             232  140 - #cf222e
              28  148 - #bbc1c8
             316  152 - #ffffff
             124  156 - #a17ce7
             168  156 - #8251df
             444  160 - #ffffff
             184  176 - #8453e0
             204  176 - #a17ce7
             284  176 - #5b5f63
              32  196 - #8d96a0
              72  264 - #8251df
             164  264 - #878e96
             232  264 - #0a6ada
             244  264 - #ddf4ff
             104  288 - #272c32
             148  288 - #5b5f63
             544  288 - #ffffff
             392  316 - #ffffff
             592  392 - #ffffff
             344  464 - #000000
              40  468 - #000000
             240  468 - #191919
             424  468 - #000000
              92  472 - #ffffff
             496  472 - #ffffff
             164  484 - #2d2d2d
             288  488 - #ffffff
             524  488 - #1a1a1a
             212  504 - #ffffff
             244  504 - #383838
             356  504 - #000000
             448  504 - #ffffff
             560  504 - #020202
              48  524 - #383838
             108  524 - #050505
";

/// Recorded with `--record-colors`.
const TYPED: &str = r"
             592    4 - #ffffff
             396   12 - #ffffff
              80   24 - #6f7881
             112   24 - #6f7881
             168   24 - #747c86
             184   24 - #6f7881
             200   24 - #747c86
              76   44 - #282d33
             116   44 - #687078
              72   84 - #cf222e
              72   88 - #cf232f
              80   88 - #cf222e
              92   88 - #d02632
             140   88 - #8453e0
              88  108 - #8250df
             124  112 - #a17ce7
             144  112 - #8453e0
             140  132 - #8453e0
             180  132 - #687078
             236  132 - #0a53b0
             592  132 - #ffffff
              28  148 - #bbc1c8
             316  152 - #ffffff
             468  152 - #ffffff
             124  156 - #a17ce7
             144  156 - #8453e0
             168  156 - #8251df
             184  176 - #8453e0
             204  176 - #a17ce7
             284  176 - #5b5f63
              32  196 - #8d96a0
             584  256 - #ddf4ff
              84  264 - #8453e0
             164  264 - #878e96
             232  264 - #0a6ada
             244  264 - #ddf4ff
              56  288 - #24292f
             104  288 - #272c32
             144  288 - #252a30
             416  300 - #ffffff
             592  384 - #ffffff
             100  472 - #1a1a1a
             444  472 - #000000
              28  476 - #000000
             176  476 - #000000
             240  476 - #ffffff
             268  476 - #ffffff
             368  480 - #ffffff
              68  492 - #000000
             160  492 - #ffffff
             476  492 - #000000
              88  496 - #525252
             184  496 - #525252
             216  496 - #525252
             328  496 - #525252
             404  496 - #000000
             448  496 - #525252
             504  496 - #000000
             528  496 - #525252
             256  508 - #ffffff
             104  512 - #000000
              36  516 - #1a1a1a
              60  516 - #414141
             296  516 - #ffffff
";

/// Recorded with `--record-colors`.
const SELECTED: &str = r"
             592    4 - #ffffff
             396   12 - #ffffff
              80   24 - #6f7881
             112   24 - #6f7881
             168   24 - #747c86
             184   24 - #6f7881
             200   24 - #747c86
             100   32 - #b8d2ff
             120   32 - #b8d2ff
             148   32 - #b8d2ff
             168   40 - #80a9e6
              60   44 - #b8d2ff
              76   44 - #272c33
             164   48 - #b8d2ff
             104   52 - #b8d2ff
             120   52 - #b8d2ff
             136   52 - #b8d2ff
              72   84 - #cf222e
              72   88 - #cf222e
              80   88 - #cf222e
              92   88 - #d02632
             140   88 - #8453e0
              88  112 - #8250df
             124  112 - #a17ce7
             140  132 - #8453e0
             180  132 - #687078
             228  132 - #282d33
              28  148 - #bbc1c8
             316  152 - #ffffff
             476  152 - #ffffff
             124  156 - #a17ce7
             168  156 - #8251df
             196  176 - #8453e0
             204  176 - #a17ce7
             284  176 - #5b5f63
              32  196 - #8d96a0
             592  260 - #ffffff
              72  264 - #8251df
             164  264 - #878e96
             232  264 - #0a6ada
             244  264 - #ddf4ff
              56  288 - #24292f
             120  288 - #24292f
             148  288 - #5b5f63
             420  312 - #ffffff
             592  384 - #ffffff
             508  472 - #000000
              80  476 - #ffffff
             136  476 - #000000
             160  476 - #383838
             340  476 - #010101
             380  476 - #ffffff
              56  496 - #010101
             128  496 - #202020
             196  496 - #525252
             252  496 - #010101
             312  496 - #525252
             348  496 - #525252
             412  496 - #ffffff
             456  496 - #ffffff
             492  496 - #0a0a0a
             536  496 - #525252
              40  516 - #ffffff
              92  516 - #000000
";

/// Recorded with `--record-colors`.
const DARK: &str = r"
             404    4 - #000000
              12   12 - #161b22
              80   24 - #8b939d
             112   24 - #8b939d
             168   24 - #868f99
             200   24 - #868f99
              76   44 - #e2e9ef
             108   44 - #89929c
              12   76 - #161b22
             592   76 - #000000
              72   84 - #ff7b72
              72   88 - #fe7b72
              80   88 - #ff7b72
              92   88 - #fa7970
             140   88 - #cea5fa
              88  108 - #d2a8ff
             172  108 - #0d1117
             124  112 - #a182c5
             464  128 - #0d1117
             140  132 - #cea5fa
             180  132 - #89929c
             228  132 - #e2e9ef
              28  148 - #474d57
             316  152 - #0d1117
             124  156 - #a182c5
             144  156 - #cfa6fb
             168  156 - #d1a7fe
             152  176 - #0d1117
             184  176 - #cfa6fb
             204  176 - #a182c5
             284  176 - #b0b6bc
              32  196 - #6d7580
             576  252 - #12233a
              84  264 - #cea5fb
             164  264 - #6f7781
             232  264 - #58a5fe
             244  264 - #12233a
             472  268 - #12233a
             356  272 - #12233a
              56  288 - #e6edf3
             104  288 - #e3eaf0
             144  288 - #e5ecf2
             172  288 - #0d1117
             592  356 - #000000
              12  368 - #161b22
             292  376 - #0d1117
             416  376 - #0d1117
             352  492 - #4c4c4c
             444  492 - #ffffff
             532  492 - #ffffff
              28  496 - #ffffff
              44  496 - #adadad
              68  496 - #ffffff
              96  496 - #adadad
             152  496 - #000000
             200  496 - #ffffff
             260  496 - #ffffff
             276  496 - #adadad
             296  496 - #d8d8d8
             320  496 - #adadad
             404  496 - #adadad
             428  496 - #000000
             472  496 - #ededed
             496  496 - #fefefe
";

/// Recorded with `--record-colors`.
const CLEARED: &str = r"
             400    4 - #000000
             592    4 - #000000
              12   12 - #161b22
             160   20 - #0d1117
              80   24 - #8b939d
             112   24 - #8b939d
             168   24 - #868f99
             184   24 - #8b939d
             200   24 - #868f99
              76   44 - #e2e9ef
              12   76 - #161b22
              72   84 - #ff7b72
              72   88 - #fe7b72
              80   88 - #ff7b72
              92   88 - #fa7970
             140   88 - #cea5fa
              88  112 - #d2a8ff
             124  112 - #a182c5
             172  112 - #0d1117
             496  112 - #0d1117
             140  132 - #cea5fa
             180  132 - #e2e9ef
              28  148 - #474d57
             292  152 - #0d1117
             120  156 - #0d1117
             124  156 - #a182c5
             144  156 - #cfa6fb
             168  156 - #d1a7fe
             152  176 - #0d1117
             184  176 - #cfa6fb
             204  176 - #a182c5
             284  176 - #b0b6bc
             316  176 - #0d1117
              32  196 - #6d7580
             464  252 - #12233a
             572  252 - #12233a
              72  264 - #d1a7fe
             168  264 - #58a5fe
             100  268 - #12233a
             264  272 - #12233a
             360  272 - #12233a
              72  284 - #e6edf3
              56  288 - #e6edf3
             120  288 - #e6edf3
             144  288 - #e5ecf2
             592  356 - #000000
              12  376 - #161b22
             440  376 - #0d1117
             532  472 - #000000
              28  492 - #ffffff
             316  492 - #ffffff
              44  496 - #adadad
             104  496 - #ffffff
             116  496 - #adadad
             140  496 - #ffffff
             152  496 - #ffffff
             172  496 - #adadad
             188  496 - #adadad
             228  496 - #adadad
             296  496 - #ededed
             312  496 - #ffffff
             368  496 - #ffffff
             400  496 - #adadad
             592  592 - #000000
";
