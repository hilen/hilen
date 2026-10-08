use std::{thread::sleep, time::Duration};

use anyhow::{Result, bail, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Container, Label, Setup, TextAlignment, TextField, UIManager, ViewData, ViewFrame, ViewTest, view},
    ui_test::{
        checkpoint,
        system_input::{screen_ink, system_request, tap, type_text, wait_until},
    },
    window::Window,
};

const NOTES: (f32, f32, f32, f32) = (20.0, 90.0, 560.0, 150.0);
const OUTSIDE: (f32, f32) = (300.0, 540.0);

const HINT: &str = "what happened, write it here";
const TYPED: &str = "Hithere";

/// The bar under the notes, `x y width height` of the room it grows in.
const BAR: (f32, f32, f32, f32) = (20.0, 300.0, 560.0, 30.0);
const BAR_STEP: f32 = 40.0;

/// The hint is many times wider than the typed word. Ink wider than this
/// in the field after typing is the hint still on screen.
const TYPED_MAX_WIDTH: f32 = 160.0;

/// Long enough for a frame that was asked for to reach the screen.
const SETTLE: Duration = Duration::from_millis(600);

/// A letter typed on the screen keyboard of an iPhone must bring a frame.
/// The system field draws the typed text itself, but the hint of the empty
/// field and every view that follows `changed` are drawn by the engine, and
/// a key of the screen keyboard is no event of the engine window. With no
/// frame the hint stays under the typed text and the bar does not grow.
#[view]
struct ScreenKeyboardTypedFrames {
    #[init]
    notes_title:   Label,
    notes:         TextField,
    bar_title:     Label,
    bar:           Container,
    outside_title: Label,
}

impl Setup for ScreenKeyboardTypedFrames {
    fn setup(self: Weak<Self>) {
        let title = |label: Weak<Label>, text: &str, y: f32| {
            label.set_text(text).set_text_size(18);
            label.set_alignment(TextAlignment::Left);
            label.place().t(y).lr(20).h(26);
        };

        title(self.notes_title, "notes with a gray hint, centered", 60.0);
        self.notes
            .set_multiline(true)
            .set_placeholder(HINT)
            .set_placeholder_color((140, 140, 140))
            .set_text_color((0, 0, 0))
            .set_text_size(24);
        self.notes.place().t(NOTES.1).l(NOTES.0).size(NOTES.2, NOTES.3);

        title(self.bar_title, "bar, 40 points for every typed character", 270.0);
        self.bar.set_color((30, 60, 160));
        self.bar.set_frame((BAR.0, BAR.1, 0.0, BAR.3));
        self.notes.changed.val(move |text| {
            let width: f32 = text.chars().map(|_| BAR_STEP).sum();
            self.bar.set_frame((BAR.0, BAR.1, width, BAR.3));
        });

        title(self.outside_title, "a tap down here ends the editing", 527.0);
    }
}

/// The width of what is drawn in an area, like `screen_ink`, but with no
/// call into the main thread. Such a call wakes the loop and the engine
/// draws a frame, which is the very thing this test must not cause.
/// `ratio` is canvas points to screen points, taken before the typing.
fn quiet_ink_width(ratio: f32, (x, y, width, height): (f32, f32, f32, f32)) -> Result<f32> {
    let reply = system_request(&format!(
        "ink {} {} {} {}",
        x * ratio,
        y * ratio,
        width * ratio,
        height * ratio
    ))?;
    let parts: Vec<&str> = reply.split(' ').collect();
    let &[left, _, right, ..] = parts.as_slice() else {
        bail!("Unexpected ink reply `{reply}`");
    };
    Ok((right.parse::<f32>()? - left.parse::<f32>()?) / UIManager::scale())
}

impl ViewTest for ScreenKeyboardTypedFrames {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        // Inside the field, so the first pixel is its background.
        let notes = (NOTES.0 + 4.0, NOTES.1 + 4.0, NOTES.2 - 8.0, NOTES.3 - 8.0);

        let hint = screen_ink(notes.0, notes.1, notes.2, notes.3)?;
        ensure!(
            hint.right - hint.left > TYPED_MAX_WIDTH,
            "the hint is not on screen before the tap: {hint:?}"
        );

        tap(NOTES.0 + NOTES.2 / 2.0, NOTES.1 + NOTES.3 - 30.0)?;
        wait_until("notes is edited", move || view.notes.is_editing())?;
        system_request("keyboard 1")?;
        checkpoint("notes is edited and empty, the hint is still there")?;

        let ratio = UIManager::scale() / from_main(UIManager::display_scale);

        // From here to the last read of the screen nothing may call into
        // the main thread, no `wait_until`, no `screen_ink`, no `from_main`.
        // Each of them wakes the loop and draws the frame the typed letter
        // has to bring by itself. The frame counter and the helper are read
        // from this thread.
        //
        // The letters go one by one, each after the engine came to rest.
        // The keyboard comes up with a move the engine draws frames for,
        // and a letter typed while those run gets its frame from them.
        sleep(SETTLE);
        let mut no_frame = String::new();
        for letter in TYPED.chars() {
            let before = Window::render_frame();
            type_text(&letter.to_string())?;
            sleep(SETTLE);
            if Window::render_frame() == before {
                no_frame.push(letter);
            }
        }
        let bar = quiet_ink_width(ratio, BAR)?;
        let ink = quiet_ink_width(ratio, notes)?;

        ensure!(
            no_frame.is_empty(),
            "no frame was drawn after these typed letters of `{TYPED}`: `{no_frame}`"
        );
        let wanted: f32 = TYPED.chars().map(|_| BAR_STEP).sum();
        ensure!(
            bar > wanted - BAR_STEP / 2.0,
            "the bar did not follow the typed text, it is {bar} wide and not {wanted}"
        );
        ensure!(
            ink < TYPED_MAX_WIDTH,
            "the hint stayed under the typed text, the ink is {ink} wide"
        );

        wait_until("notes holds the typed word", move || view.notes.text() == TYPED)?;
        checkpoint("only Hithere in the notes, the hint is gone, the bar is 280 wide")?;

        tap(OUTSIDE.0, OUTSIDE.1)?;
        wait_until("notes is not edited any more", move || !view.notes.is_editing())?;
        system_request("keyboard 0")?;
        checkpoint("only Hithere in the notes after the editing")?;

        let after = screen_ink(notes.0, notes.1, notes.2, notes.3)?;
        ensure!(
            after.right - after.left < TYPED_MAX_WIDTH,
            "the hint came back after the editing: {after:?}"
        );
        Ok(())
    }
}
