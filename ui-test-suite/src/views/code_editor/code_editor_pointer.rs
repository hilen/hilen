use std::{thread::sleep, time::Duration};

use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        CodeEditor, CodePos, CodePress, Color, Container, Label, Modifiers, Point, Setup, UIColor, ViewData,
        ViewFrame, ViewTest, WHITE, view,
    },
    ui_test::{checkpoint, inject_modifiers, inject_touches, system_input::wait_until},
};

use super::common::{SCAD, click_at, color_all, screen_point, status_style};

const RESTED: UIColor = UIColor::Plain(Color::hex("#cfe3ff"));
const POINTER: Color = Color::hex("#e5484d");
const POINTER_SIZE: f32 = 8.0;

/// The line with `cube(size);`, `cube` starts at byte 8.
const CUBE_LINE: usize = 5;
const CUBE: usize = 8;
/// The line with `plate();`.
const CALL_LINE: usize = 11;

/// What the pointer does over a code editor. It rests on a name and the
/// editor tells where, shown as a blue ground behind that char. It moves
/// off and the editor tells that too. A short stop on the way tells
/// nothing. A press tells its place, and whether Cmd was held. The red
/// square is where the test put the pointer.
#[view]
struct CodeEditorPointer {
    /// The last `pointer_rested`, and how often it came with a place and
    /// with none.
    rested: Option<CodePos>,
    told:   usize,
    left:   usize,
    press:  Option<CodePress>,

    #[init]
    editor:  CodeEditor,
    status:  Label,
    pointer: Container,
}

impl Setup for CodeEditorPointer {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        self.editor.place().t(10).lr(10).h(380);
        self.editor.set_text(SCAD);
        color_all(self.editor);
        self.editor.pointer_rested.val(move |at| {
            let mut editor = self.editor;
            editor.clear_background_runs();
            match at {
                Some(at) => {
                    self.told += 1;
                    editor.set_background_runs(at.line, [(at.byte..at.byte + 1, RESTED)]);
                }
                None => self.left += 1,
            }
            self.rested = at;
        });
        self.editor.pressed.val(move |press| self.press = Some(press));

        status_style(self.status);
        self.status.place().t(400).lr(10).b(10);
        self.pointer.set_color(POINTER);
        self.pointer.set_hidden(true);
    }
}

/// Moves the pointer to a point of the window, with no button down, and
/// puts the red square there.
fn move_to(view: Weak<CodeEditorPointer>, at: (f32, f32)) {
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

/// The middle of the char that starts at `pos`.
fn char_point(view: Weak<CodeEditorPointer>, pos: CodePos) -> (f32, f32) {
    from_main(move || {
        let from = screen_point(view.editor, pos);
        let to = screen_point(view.editor, CodePos::new(pos.line, pos.byte + 1));
        (f32::midpoint(from.0, to.0) - 1.0, from.1)
    })
}

fn say(view: Weak<CodeEditorPointer>, text: &'static str) {
    from_main(move || {
        view.status.set_text(text);
    });
}

impl ViewTest for CodeEditorPointer {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let on_cube = char_point(view, CodePos::new(CUBE_LINE, CUBE));
        // Past the end of the line, which is 19 bytes long.
        let beside = from_main(move || {
            let end = screen_point(view.editor, CodePos::new(CUBE_LINE, 19));
            (end.0 + 60.0, end.1)
        });

        move_to(view, on_cube);
        ensure!(
            from_main(move || view.rested.is_none() && view.told == 0),
            "the rest was told at once"
        );
        wait_until("the pointer rests on cube", move || view.told == 1)?;
        let rested = from_main(move || view.rested);
        ensure!(
            rested == Some(CodePos::new(CUBE_LINE, CUBE)),
            "the pointer rests at {rested:?}"
        );
        say(
            view,
            "the pointer stood on the c of cube for a moment, the editor told its place, the c is on blue",
        );
        wait_for_next_frame();
        checkpoint("the pointer rests on the c of cube, the c is on blue")?;

        move_to(view, beside);
        let (rested, left) = from_main(move || (view.rested, view.left));
        ensure!(
            rested.is_none() && left == 1,
            "off the code: {rested:?}, told {left} times"
        );
        say(
            view,
            "the pointer moved past the end of the line, the editor told that it left, the blue is gone",
        );
        wait_for_next_frame();
        checkpoint("the pointer is beside the code, no char is on blue")?;

        // A short stop on a char on the way to another place tells nothing.
        move_to(view, on_cube);
        move_to(view, beside);
        let quiet = from_main(move || {
            let local = |at: (f32, f32)| {
                let frame = view.editor.absolute_frame();
                Point::new(at.0 - frame.x(), at.1 - frame.y())
            };
            (
                view.editor.text_at(local(on_cube)),
                view.editor.text_at(local(beside)),
                view.editor.pos_at(local(beside)),
                view.editor.text_at(Point::new(2.0, local(on_cube).y)),
            )
        });
        ensure!(
            quiet.0 == Some(CodePos::new(CUBE_LINE, CUBE)),
            "the char under the pointer is {:?}",
            quiet.0
        );
        ensure!(
            quiet.1.is_none(),
            "a place beside the code has the char {:?}",
            quiet.1
        );
        ensure!(
            quiet.2 == CodePos::new(CUBE_LINE, 19),
            "the nearest place is {:?}",
            quiet.2
        );
        ensure!(quiet.3.is_none(), "the gutter has the char {:?}", quiet.3);
        // Longer than the wait of the editor.
        sleep(Duration::from_millis(700));
        let (told, left) = from_main(move || (view.told, view.left));
        ensure!(
            (told, left) == (1, 1),
            "a short stop was told: {told} rests, {left} leaves"
        );

        check_presses(view)?;
        Ok(())
    }
}

/// A press tells its place, and with Cmd held it says so.
fn check_presses(view: Weak<CodeEditorPointer>) -> Result<()> {
    click_at(view.editor, CodePos::new(CALL_LINE, 2));
    let press = from_main(move || view.press);
    ensure!(
        press
            == Some(CodePress {
                pos:     CodePos::new(CALL_LINE, 2),
                command: false,
            }),
        "the press is {press:?}"
    );

    inject_modifiers(Modifiers::META);
    click_at(view.editor, CodePos::new(CUBE_LINE, CUBE + 2));
    inject_modifiers(Modifiers::empty());
    let (press, cursor) = from_main(move || (view.press, view.editor.cursor()));
    ensure!(
        press
            == Some(CodePress {
                pos:     CodePos::new(CUBE_LINE, CUBE + 2),
                command: true,
            }),
        "the press with Cmd is {press:?}"
    );
    ensure!(
        cursor == CodePos::new(CUBE_LINE, CUBE + 2),
        "the caret is at {cursor:?}"
    );
    say(
        view,
        "a press in plate told its place, then a press with Cmd in cube told its place and the Cmd, \
         the caret is in cube",
    );
    wait_for_next_frame();
    checkpoint("a press with Cmd in cube, the caret is in cube")?;
    Ok(())
}
