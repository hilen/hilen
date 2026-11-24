use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::{Own, Weak},
    ui::{
        CodeEditor, CodePos, Color, Completion, Label, NamedKey, Setup, TextAlignment, UIColor, ViewData,
        ViewFrame, ViewTest, WHITE, WeakView, view,
    },
    ui_test::{check_colors, checkpoint, inject_named_key, set_record_probe_count},
};

use super::common::{SCAD, click_at, color_all, screen_point, status_style};

const NOTE: UIColor = UIColor::Plain(Color::hex("#fff8c5"));
const NOTE_BORDER: UIColor = UIColor::Plain(Color::hex("#d4a72c"));
const PANEL: UIColor = UIColor::Plain(Color::hex("#ddf4ff"));
const PANEL_BORDER: UIColor = UIColor::Plain(Color::hex("#54aeff"));

/// The line with `module plate() {`, `plate` starts at byte 7.
const MODULE_LINE: usize = 3;
const PLATE: usize = 7;
/// The line with `plate();`, near the bottom of the window.
const CALL_LINE: usize = 11;
const NOTE_HEIGHT: f32 = 30.0;
/// A panel this high has no room under the line of `plate();`.
const TALL: f32 = 170.0;

/// The note of a code editor, a view that floats over a line while a
/// panel floats under it. A yellow note stands over `plate` in line 4.
/// The list of completions opens under the same word and both show. Near
/// the bottom of the window a blue panel has room only over its line, so
/// the note of that line gives way to it and comes back when the panel
/// hides. A note too high for the room over the first line goes under it.
/// Escape closes a note and the editor keeps the keys, and a note goes
/// when the editor loses them.
#[view]
struct CodeEditorNote {
    #[init]
    status: Label,
    editor: CodeEditor,
}

impl Setup for CodeEditorNote {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);
        status_style(self.status);
        self.status.place().t(10).lr(10).h(170);
        self.editor.place().t(190).lr(10).b(10);
        self.editor.set_text(SCAD);
        color_all(self.editor);
    }
}

/// A box with a text, the content of a note or of a panel.
fn boxed(text: &str, height: f32, fill: UIColor, border: UIColor) -> Own<Label> {
    let label = Label::new();
    label.set_text(text).set_text_size(14).set_alignment(TextAlignment::Center);
    label.set_color(fill).set_corner_radius(6);
    label.set_border_color(border).set_border_width(1);
    label.set_size(220, height);
    label
}

fn note(text: &str, height: f32) -> Own<Label> {
    boxed(text, height, NOTE, NOTE_BORDER)
}

/// The top and the height of the note, and of the panel, in the window.
/// A view that does not show has no height.
fn floats(view: Weak<CodeEditorNote>) -> ((f32, f32), (f32, f32)) {
    wait_for_next_frame();
    from_main(move || {
        let of = |float: WeakView| {
            if float.is_ok() {
                let frame = float.absolute_frame();
                (frame.y(), frame.height())
            } else {
                (0.0, 0.0)
            }
        };
        (of(view.editor.note()), of(view.editor.panel()))
    })
}

/// The top and the bottom of a line in the window.
fn line_edges(view: Weak<CodeEditorNote>, line: usize) -> (f32, f32) {
    from_main(move || {
        let (_, middle) = screen_point(view.editor, CodePos::new(line, 0));
        let half = view.editor.line_height() / 2.0;
        (middle - half, middle + half)
    })
}

fn say(view: Weak<CodeEditorNote>, text: &'static str) {
    from_main(move || {
        view.status.set_text(text);
    });
}

impl ViewTest for CodeEditorNote {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);
        let (module_top, module_bottom) = line_edges(view, MODULE_LINE);
        from_main(move || {
            view.editor.show_note(
                CodePos::new(MODULE_LINE, PLATE),
                note("note: module plate()", NOTE_HEIGHT),
            );
        });
        say(
            view,
            "a yellow note stands over plate in line 4, its left edge at the p",
        );
        let ((top, height), _) = floats(view);
        ensure!(
            height == NOTE_HEIGHT && top + height <= module_top,
            "the note is at {top} and {height} high, line 4 starts at {module_top}"
        );
        check_colors(NOTE_ONLY)?;

        from_main(move || {
            let items = ["plate", "plate_with_holes", "place"]
                .into_iter()
                .map(|label| Completion {
                    label:  label.to_string(),
                    detail: "module".to_string(),
                    icon:   String::new(),
                })
                .collect();
            view.editor.show_completions(CodePos::new(MODULE_LINE, PLATE), items);
        });
        say(
            view,
            "the list of completions opened under the same word: the note stays over line 4, the list is \
             under it, both show",
        );
        let ((note_top, note_height), (list_top, list_height)) = floats(view);
        ensure!(
            note_height == NOTE_HEIGHT && note_top + note_height <= module_top,
            "with the list the note is at {note_top} and {note_height} high"
        );
        ensure!(
            list_height > 0.0 && list_top >= module_bottom,
            "the list is at {list_top} and {list_height} high, line 4 ends at {module_bottom}"
        );
        check_colors(NOTE_AND_LIST)?;

        let (call_top, _) = line_edges(view, CALL_LINE);
        from_main(move || {
            view.editor.hide_completions();
            view.editor
                .show_note(CodePos::new(CALL_LINE, 0), note("note: plate()", NOTE_HEIGHT));
            view.editor.show_panel(
                CodePos::new(CALL_LINE, 0),
                boxed("a panel over line 12", TALL, PANEL, PANEL_BORDER),
            );
        });
        say(
            view,
            "line 12 is near the bottom: the blue panel has no room under it and lies over it, where the \
             note of that line would be, so the note does not show",
        );
        let ((_, note_height), (panel_top, panel_height)) = floats(view);
        ensure!(
            panel_height == TALL && panel_top + panel_height <= call_top,
            "the panel is at {panel_top} and {panel_height} high, line 12 starts at {call_top}"
        );
        ensure!(
            note_height == 0.0,
            "the note shows under the panel, {note_height} high"
        );
        check_colors(PANEL_OVER)?;

        from_main(move || view.editor.hide_panel());
        say(view, "the panel hid, the note of line 12 is back over its line");
        let ((top, height), (_, panel_height)) = floats(view);
        ensure!(panel_height == 0.0, "the panel is still there");
        ensure!(
            height == NOTE_HEIGHT && top + height <= call_top,
            "after the panel the note is at {top} and {height} high"
        );
        checkpoint("the yellow note stands over plate(); in line 12")?;

        check_high_note(view)
    }
}

/// A note too high for the room under its line, and the keys while a note
/// shows.
fn check_high_note(view: Weak<CodeEditorNote>) -> Result<()> {
    let (_, first_bottom) = line_edges(view, 0);
    from_main(move || {
        view.editor.show_note(CodePos::new(0, 3), note("a note 200 high", 200.0));
    });
    say(
        view,
        "a note 200 high has no room over line 1, it lies under that line",
    );
    let ((top, height), _) = floats(view);
    ensure!(
        (height - 200.0).abs() < f32::EPSILON && top >= first_bottom,
        "the high note is at {top} and {height} high, line 1 ends at {first_bottom}"
    );
    checkpoint("the high yellow note lies under line 1")?;

    let left = from_main(move || {
        view.editor.hide_note();
        view.editor.note().is_ok()
    });
    ensure!(!left, "the note is still there");

    click_at(view.editor, CodePos::new(CALL_LINE, 5));
    from_main(move || {
        view.editor
            .show_note(CodePos::new(CALL_LINE, 0), note("note: plate()", NOTE_HEIGHT));
    });
    inject_named_key(NamedKey::Escape);
    let state = from_main(move || (view.editor.note().is_ok(), view.editor.is_editing()));
    ensure!(
        state == (false, true),
        "after Escape the note shows {} and the editor has the keys {}",
        state.0,
        state.1
    );
    from_main(move || {
        view.editor
            .show_note(CodePos::new(CALL_LINE, 0), note("note: plate()", NOTE_HEIGHT));
    });
    inject_named_key(NamedKey::Escape);
    inject_named_key(NamedKey::Escape);
    let state = from_main(move || (view.editor.note().is_ok(), view.editor.is_editing()));
    ensure!(
        state == (false, false),
        "after 2 more Escape the note shows {} and the editor has the keys {}",
        state.0,
        state.1
    );
    say(
        view,
        "a press gave the editor the keys: Escape closed a note and left the keys, a second Escape on \
         a new note closed it, a third one ended the editing",
    );
    checkpoint("no note, the editor has no caret")?;
    Ok(())
}

/// Recorded with `--record-colors`.
const NOTE_ONLY: &str = r"
             592    4 - #ffffff
             220   92 - #383838
              32   96 - #000000
              68   96 - #ffffff
             116   96 - #525252
             152   96 - #000000
             180   96 - #ffffff
             240   96 - #525252
             260   96 - #272727
             288   96 - #525252
             352   96 - #050505
             384   96 - #000000
             444   96 - #000000
             444  100 - #181818
              80  204 - #6f7881
             112  204 - #6f7881
             168  204 - #747c86
             184  204 - #6f7881
             200  204 - #747c86
              76  224 - #282d33
             120  224 - #d4a72c
             144  224 - #d4a72c
             188  224 - #d4a72c
             240  224 - #d4a72c
             288  224 - #d4a72c
             320  228 - #fff8c5
             220  236 - #fff8c5
             164  240 - #fff8c5
             204  240 - #fff8c5
             244  240 - #040403
             260  240 - #fff8c5
             272  240 - #fff8c5
             108  248 - #fff8c5
             136  252 - #fff8c5
             280  252 - #fff8c5
             308  252 - #fff8c5
              72  264 - #cf222e
              72  268 - #cf232f
              80  268 - #cf222e
              92  268 - #d02632
             172  288 - #ffffff
              88  292 - #8250df
             124  292 - #a17ce7
             140  312 - #8453e0
             180  312 - #282d33
             592  324 - #ffffff
              28  328 - #bbc1c8
             292  332 - #ffffff
             124  336 - #a17ce7
             136  336 - #8251df
             168  336 - #8251df
             320  352 - #ffffff
             184  356 - #8453e0
             204  356 - #a17ce7
             284  356 - #5b5f63
              32  376 - #8d96a0
             448  432 - #ffffff
              72  444 - #8251df
             120  464 - #24292f
              56  468 - #24292f
             148  468 - #5b5f63
             172  468 - #ffffff
             364  592 - #ffffff
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const NOTE_AND_LIST: &str = r"
             252   84 - #000000
             120   88 - #313131
             276   88 - #ffffff
             352   88 - #ffffff
             436   88 - #191919
             488   88 - #ffffff
             556   88 - #000000
             200  100 - #1a1a1a
              28  104 - #000000
             100  104 - #2d2d2d
             156  104 - #000000
              80  204 - #6f7881
             112  204 - #6f7881
             168  204 - #747c86
             200  204 - #747c86
             148  224 - #d4a72c
             244  224 - #d4a72c
             316  224 - #d4a72c
             120  228 - #fff8c5
             176  228 - #fff8c5
             200  236 - #fff8c5
             220  236 - #fff8c5
             268  236 - #fff8c5
             164  240 - #fff8c5
             300  252 - #fff8c5
              72  264 - #cf222e
              72  268 - #cf232f
              80  268 - #cf222e
              92  268 - #d02632
             128  268 - #8251df
             140  268 - #8453e0
             528  288 - #d3d3d3
              88  292 - #8250df
             452  292 - #9facb6
             224  312 - #45494e
             204  316 - #ffffff
             452  316 - #a8aeb6
             528  324 - #d3d3d3
              28  328 - #bbc1c8
             452  340 - #a8aeb6
             528  344 - #d3d3d3
             112  352 - #acacac
             248  352 - #acacac
             332  352 - #acacac
             412  352 - #acacac
             196  356 - #6942b2
             284  356 - #484c4f
             316  356 - #cbcbcb
             160  360 - #e8e8e8
             216  360 - #e8e8e8
             272  360 - #e8e8e8
             280  360 - #e8e8e8
             368  360 - #e8e8e8
             484  360 - #e8e8e8
             520  360 - #e8e8e8
              32  376 - #8d96a0
              72  444 - #8251df
              56  468 - #24292f
             104  468 - #272c32
             148  468 - #5b5f63
             468  500 - #ffffff
             208  592 - #ffffff
             348  592 - #ffffff
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const PANEL_OVER: &str = r"
              60   84 - #353535
             260   84 - #2d2d2d
             312   84 - #3b3b3b
             412   84 - #454545
             568   84 - #000000
              40   88 - #191919
             100   88 - #272727
             140   88 - #191919
             204   88 - #181818
             388   88 - #ffffff
             400   88 - #111111
             484   88 - #010101
             512   88 - #ffffff
             564   88 - #000000
              80  100 - #282828
             156  104 - #0a0a0a
             200  104 - #000000
             232  104 - #ffffff
             292  104 - #ffffff
             348  104 - #ffffff
             376  104 - #000000
             428  104 - #383838
             456  104 - #ffffff
              80  204 - #6f7881
             112  204 - #6f7881
             168  204 - #747c86
             184  204 - #6f7881
             200  204 - #747c86
             592  216 - #ffffff
              76  224 - #282d33
             428  252 - #ffffff
              60  260 - #54aeff
              80  260 - #54aeff
             100  260 - #54aeff
             116  260 - #54aeff
             132  260 - #54aeff
             160  260 - #54aeff
             184  260 - #54aeff
             204  260 - #54aeff
             228  260 - #54aeff
             248  260 - #54aeff
              28  328 - #bbc1c8
             292  332 - #ffffff
             548  340 - #ffffff
             128  344 - #000000
             152  344 - #ddf4ff
             180  344 - #000000
             208  344 - #040505
             320  352 - #ffffff
             284  356 - #5b5f63
              32  376 - #8d96a0
              72  444 - #8251df
              84  444 - #8453e0
             420  444 - #ffffff
             172  464 - #ffffff
              56  468 - #24292f
             104  468 - #272c32
             148  468 - #5b5f63
             556  468 - #ffffff
             296  480 - #ffffff
               4  592 - #ffffff
             212  592 - #ffffff
             352  592 - #ffffff
             592  592 - #ffffff
";
