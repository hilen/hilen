use std::{thread::sleep, time::Duration};

use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        CLEAR, CodeEditor, CodePos, Color, Container, Label, ScrollView, Setup, TextAlignment, UIColor,
        ViewData, ViewFrame, ViewSubviews, ViewTest, WHITE, view,
    },
    ui_test::{checkpoint, inject_scroll, inject_touches, system_input::wait_until},
};

use super::common::{SCAD, color_all, screen_point, status_style};

const NOTE: UIColor = UIColor::Plain(Color::hex("#fff8c5"));
const NOTE_BORDER: UIColor = UIColor::Plain(Color::hex("#d4a72c"));
const POINTER: Color = Color::hex("#e5484d");
const POINTER_SIZE: f32 = 8.0;
/// Nearer than the note, which floats over the whole window like a toast.
const POINTER_Z: f32 = 0.2;

/// The line with `cube(size);`, `cube` starts at byte 8.
const CUBE_LINE: usize = 5;
const CUBE: usize = 8;
/// The line with `size = [40, 30, 10];`, 20 bytes long.
const SIZE_LINE: usize = 1;

const NOTE_WIDTH: f32 = 260.0;
const NOTE_HEIGHT: f32 = 110.0;
/// The text of the note has more lines than the note shows.
const TEXT_LINES: usize = 20;
const TEXT_LINE_HEIGHT: usize = 20;

/// A panel that an app shows for a rest of the pointer stays while the
/// pointer is on it. The pointer rests on a name and a yellow note with a
/// long text opens under it, lower than the editor. The pointer goes down
/// onto the note and the note stays. The wheel there moves the text of the
/// note and not the code. The pointer leaves the note and the note closes.
/// Then the pointer rests again and moves away beside the code: the note
/// stays for a moment and closes. Last the wheel over the code moves the
/// code. The red square is where the test put the pointer.
#[view]
struct CodeEditorPanelRest {
    /// How often `pointer_rested` came with a place and with none.
    told:    usize,
    left:    usize,
    note:    Weak<LongNote>,
    pointer: Weak<Container>,

    #[init]
    editor: CodeEditor,
    status: Label,
}

/// What an app shows for a long text: a scroll view with the text in it.
#[view]
struct LongNote {
    #[init]
    scroll: ScrollView,
}

impl Setup for LongNote {
    fn setup(self: Weak<Self>) {
        self.set_size(NOTE_WIDTH, NOTE_HEIGHT);
        self.set_color(NOTE).set_corner_radius(6);
        self.set_border_color(NOTE_BORDER).set_border_width(1);
        self.scroll.place().back();
        let text = self.scroll.add_view::<Label>();
        let lines: Vec<String> = (1..=TEXT_LINES)
            .map(|line| format!("line {line} of the text of the note"))
            .collect();
        text.set_text(lines.join("\n"))
            .set_multiline(true)
            .set_text_size(14)
            .set_line_height(TEXT_LINE_HEIGHT)
            .set_alignment(TextAlignment::Left);
        text.set_color(CLEAR);
        text.place().t(0).lr(8).h(TEXT_LINES * TEXT_LINE_HEIGHT);
    }
}

impl Setup for CodeEditorPanelRest {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        // Lower than its 13 lines, so the wheel can move the code.
        self.editor.place().t(10).lr(10).h(180);
        self.editor.set_text(SCAD);
        color_all(self.editor);
        self.editor.pointer_rested.val(move |at| {
            if let Some(at) = at {
                self.told += 1;
                let note = LongNote::new();
                self.note = note.weak();
                self.editor.show_panel(at, note);
            } else {
                self.left += 1;
                self.editor.hide_panel();
            }
        });

        status_style(self.status);
        self.status.place().t(400).lr(10).b(10);
        let pointer = Container::new();
        pointer.set_z_position(POINTER_Z);
        pointer.set_color(POINTER);
        pointer.set_hidden(true);
        self.pointer = self.add_subview(pointer);
    }
}

/// Moves the pointer to a point of the window, with no button down, and
/// puts the red square there.
fn move_to(view: Weak<CodeEditorPanelRest>, at: (f32, f32)) {
    from_main(move || {
        view.pointer.set_hidden(false);
        view.pointer.set_frame((
            at.0 - POINTER_SIZE / 2.0,
            at.1 + POINTER_SIZE,
            POINTER_SIZE,
            POINTER_SIZE,
        ));
    });
    inject_touches(format!("{} {} m", at.0, at.1));
    wait_for_next_frame();
}

/// Moves the pointer from `from` to `to` in small steps, the way a hand
/// does, so it crosses what lies between them.
fn glide(view: Weak<CodeEditorPanelRest>, from: (f32, f32), to: (f32, f32)) {
    const STEPS: u8 = 8;
    for step in 1..=STEPS {
        let part = f32::from(step) / f32::from(STEPS);
        move_to(
            view,
            (from.0 + (to.0 - from.0) * part, from.1 + (to.1 - from.1) * part),
        );
    }
}

fn say(view: Weak<CodeEditorPanelRest>, text: &'static str) {
    from_main(move || {
        view.status.set_text(text);
    });
}

/// The note on screen, its place and size and how far its text moved.
fn note_of(view: Weak<CodeEditorPanelRest>) -> Option<(f32, f32, f32, f32, f32)> {
    from_main(move || {
        let panel = view.editor.panel();
        (panel.is_ok() && view.note.is_ok()).then(|| {
            let frame = panel.absolute_frame();
            (
                frame.x(),
                frame.y(),
                frame.size.width,
                frame.size.height,
                view.note.scroll.get_scroll_content_offset(),
            )
        })
    })
}

impl ViewTest for CodeEditorPanelRest {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let cube = CodePos::new(CUBE_LINE, CUBE);
        let on_cube = from_main(move || {
            let from = screen_point(view.editor, cube);
            let to = screen_point(view.editor, CodePos::new(CUBE_LINE, CUBE + 1));
            (f32::midpoint(from.0, to.0) - 1.0, from.1)
        });
        let cube_at = move || from_main(move || screen_point(view.editor, cube));
        let code_before = cube_at();

        move_to(view, on_cube);
        wait_until("the pointer rests on cube", move || view.told == 1)?;
        wait_for_next_frame();
        let Some((x, y, width, height, _)) = note_of(view) else {
            anyhow::bail!("no note shows after the rest");
        };
        ensure!(
            (width, height) == (NOTE_WIDTH, NOTE_HEIGHT),
            "the note is {width} by {height}"
        );
        let editor_bottom = from_main(move || {
            let frame = view.editor.absolute_frame();
            frame.y() + frame.size.height
        });
        ensure!(
            y + height > editor_bottom,
            "the note ends at {}, inside the editor that ends at {editor_bottom}",
            y + height
        );
        say(
            view,
            "the pointer stood on the c of cube, a yellow note with a long text opened under it, the note is \
             lower than the editor",
        );
        wait_for_next_frame();
        checkpoint("the pointer rests on cube, a yellow note with a long text is under it")?;

        // Down onto the lower part of the note, which is outside the editor.
        let on_note = (x + 60.0, y + height - 20.0);
        glide(view, on_cube, on_note);
        // Longer than the wait before a rest ends.
        sleep(Duration::from_millis(700));
        let left = from_main(move || view.left);
        ensure!(
            left == 0 && note_of(view).is_some(),
            "the rest ended {left} times while the pointer is on the note"
        );
        say(
            view,
            "the pointer went down onto the note and stood there, the note stays",
        );
        wait_for_next_frame();
        checkpoint("the pointer is on the note, the note stays")?;

        for _ in 0..3 {
            inject_scroll(-40);
        }
        wait_for_next_frame();
        let moved = note_of(view).map(|note| note.4);
        ensure!(
            moved.is_some_and(|offset| offset < 0.0),
            "the wheel on the note left its text at {moved:?}"
        );
        let code_now = cube_at();
        ensure!(
            code_now == code_before,
            "the wheel on the note moved the code from {code_before:?} to {code_now:?}"
        );
        say(
            view,
            "the wheel on the note moved its text up, the first lines are gone, the code did not move",
        );
        wait_for_next_frame();
        checkpoint("the text of the note moved up, the code did not move")?;

        let beside = (x + width + 60.0, y + height - 20.0);
        move_to(view, beside);
        wait_until("the rest ends after the pointer left the note", move || {
            view.left == 1
        })?;
        ensure!(note_of(view).is_none(), "the note still shows");
        say(view, "the pointer left the note, the note is gone");
        wait_for_next_frame();
        checkpoint("the pointer is beside the note, the note is gone")?;

        move_to(view, on_cube);
        check_rest_away(view, on_cube, code_before)?;
        checkpoint("no note, the code moved up")?;
        Ok(())
    }
}

/// A rest that ends away from the note closes it a moment later, and the wheel
/// on code with no char moves the code.
fn check_rest_away(
    view: Weak<CodeEditorPanelRest>,
    on_cube: (f32, f32),
    code_before: (f32, f32),
) -> Result<()> {
    wait_until("the pointer rests on cube again", move || view.told == 2)?;
    // Past the end of the line, not onto the note.
    let away = (on_cube.0 + 300.0, on_cube.1);
    move_to(view, away);
    let left = from_main(move || view.left);
    ensure!(
        left == 1 && note_of(view).is_some(),
        "the rest ended at once, {left} times, with a note on screen"
    );
    wait_until("the rest ends after the pointer left the name", move || {
        view.left == 2
    })?;
    ensure!(note_of(view).is_none(), "the note still shows");
    let told = from_main(move || view.told);
    ensure!(told == 2, "{told} rests were told");

    // On the editor with no char under it, so nothing rests.
    let on_code = from_main(move || {
        let end = screen_point(view.editor, CodePos::new(SIZE_LINE, 20));
        (end.0 + 120.0, end.1)
    });
    move_to(view, on_code);
    for _ in 0..3 {
        inject_scroll(-40);
    }
    wait_for_next_frame();
    let code_now = from_main(move || screen_point(view.editor, CodePos::new(CUBE_LINE, CUBE)));
    ensure!(
        code_now.1 < code_before.1,
        "the wheel on the code left cube at {code_now:?}"
    );
    say(
        view,
        "the pointer rested on cube again and moved away beside the line: the note stayed a moment and \
         closed. Then the wheel on the code moved the code up",
    );
    wait_for_next_frame();
    Ok(())
}
