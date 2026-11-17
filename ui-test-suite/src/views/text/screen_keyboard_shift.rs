use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Label, ScreenKeyboard, Setup, TextAlignment, TextField, ViewData, ViewFrame, ViewTest, view},
    ui_test::{
        check_colors, checkpoint,
        system_input::{screen_keyboard, tap, wait_until},
    },
};

use super::keyboard_marker::KeyboardMarker;

const FIELD: (f32, f32) = (300.0, 925.0);
/// Above every keyboard, also after the screen moved up.
const OUTSIDE: (f32, f32) = (300.0, 420.0);
/// Where the keyboard of the phone of the simulator lane ends, the stand in
/// of the other platforms ends there too.
const KEYBOARD_TOP: f32 = 814.0;
const FIELD_BOTTOM: f32 = 950.0;

/// A field at the bottom of a screen with nothing to scroll. The keyboard
/// would cover it, so the whole screen moves up with the keyboard, and
/// comes back down with it.
#[view]
struct ScreenKeyboardShift {
    #[init]
    title:  Label,
    middle: Label,
    hint:   Label,
    field:  TextField,
    marker: KeyboardMarker,
}

impl Setup for ScreenKeyboardShift {
    fn setup(self: Weak<Self>) {
        let label = |label: Weak<Label>, text: &str, y: f32| {
            label.set_text(text).set_text_size(18);
            label.set_alignment(TextAlignment::Left);
            label.place().t(y).lr(20).h(26);
        };

        label(self.title, "top of the screen", 20.0);
        label(self.middle, "middle, it moves up with the field", 500.0);
        // From the bottom, so the field is under the keyboard on a screen
        // of any height, also when the view is shown outside a test.
        self.hint.set_text("the keyboard would cover this field").set_text_size(18);
        self.hint.set_alignment(TextAlignment::Left);
        self.hint.place().b(106).lr(20).h(26);

        self.field.set_text_size(24).set_placeholder("tap to type");
        self.field.place().b(50).lr(20).h(50);
    }
}

impl ViewTest for ScreenKeyboardShift {
    fn canvas() -> (u32, u32) {
        (600, 1000)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        tap(FIELD.0, FIELD.1)?;
        wait_until("the field is edited", move || view.field.is_editing())?;
        screen_keyboard(true, KEYBOARD_TOP)?;
        wait_until("the field is above the keyboard", move || {
            ScreenKeyboard::top().is_some_and(|top| view.field.absolute_frame().max_y() < top)
        })?;
        checkpoint("the keyboard is up, the screen moved up, the field is above the keyboard")?;
        from_main(move || {
            let top = ScreenKeyboard::top().unwrap_or_default();
            let bottom = view.field.absolute_frame().max_y();
            ensure!(
                top < FIELD_BOTTOM,
                "the keyboard at {top} never covered the field"
            );
            ensure!(
                (top - bottom - 8.0).abs() < 1.0,
                "the field ends at {bottom}, the keyboard starts at {top}"
            );
            let shift = ScreenKeyboard::shift();
            ensure!(
                (shift - (FIELD_BOTTOM + 8.0 - top)).abs() < 1.0,
                "the screen moved up by {shift} for a keyboard at {top}"
            );
            let title = view.title.absolute_frame().y();
            ensure!(
                (title - (20.0 - shift)).abs() < 1.0,
                "the title is at {title} with the screen moved by {shift}"
            );
            Ok(())
        })?;

        tap(OUTSIDE.0, OUTSIDE.1)?;
        wait_until("the field is not edited any more", move || {
            !view.field.is_editing()
        })?;
        screen_keyboard(false, KEYBOARD_TOP)?;
        wait_until("the screen is back down", move || {
            (view.field.absolute_frame().max_y() - FIELD_BOTTOM).abs() < 0.5
        })?;
        from_main(|| {
            let shift = ScreenKeyboard::shift();
            ensure!(shift == 0.0, "the screen is still moved up by {shift}");
            Ok(())
        })?;

        check_colors(FINAL)
    }
}

const FINAL: &str = r"
              56   32 - #597c95
              80   32 - #304350
              96   36 - #425b6e
             168   36 - #10171b
             592  180 - #597c95
             332  248 - #597c95
             220  508 - #273742
             104  512 - #374d5c
             216  512 - #172026
             292  512 - #597c95
              44  516 - #1b252d
             164  516 - #000001
             248  516 - #466175
             592  588 - #597c95
             252  876 - #000000
              44  880 - #000000
             168  880 - #24333d
             108  884 - #597c95
             200  884 - #597c95
             404  900 - #ffffff
             576  900 - #ffffff
             288  924 - #e5e5e5
             316  924 - #ffffff
             332  924 - #bcbcbc
             288  928 - #e5e5e5
             300  928 - #ffffff
             336  928 - #ffffff
             248  932 - #bcbcbc
             272  932 - #bcbcbc
             324  932 - #bcbcbc
             332  936 - #bcbcbc
             460  948 - #ffffff
";
