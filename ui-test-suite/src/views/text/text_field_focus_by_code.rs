use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Label, ScreenKeyboard, Setup, TextAlignment, TextField, UIManager, ViewData, ViewTest, view},
    ui_test::{
        check_colors, checkpoint,
        system_input::{system_input, system_request, tap, wait_until},
    },
};

const FIELD: (f32, f32) = (540.0, 115.0);
const OUTSIDE: (f32, f32) = (300.0, 400.0);

/// A focus set by code. On a desktop it starts the editing. On a phone it
/// does nothing, there it would bring the screen keyboard up with no word
/// from the user. A tap opens the keyboard there, and so does
/// `focus_with_keyboard`.
#[view]
struct TextFieldFocusByCode {
    #[init]
    title: Label,
    field: TextField,
    state: Label,
}

impl Setup for TextFieldFocusByCode {
    fn setup(self: Weak<Self>) {
        let label = |label: Weak<Label>, y: f32| {
            label.set_text_size(18);
            label.set_alignment(TextAlignment::Left);
            label.place().t(y).lr(20).h(26);
        };

        label(self.title, 60.0);
        self.title.set_text("a field that code puts the focus on");

        self.field.set_text_size(24).set_text("Riga");
        self.field.place().t(90).lr(20).h(50);

        label(self.state, 160.0);
        self.state.set_text("nothing yet");
    }
}

/// The keyboard is up or down. Only a phone can tell.
fn keyboard(up: bool) -> Result<()> {
    if system_input() {
        system_request(if up { "keyboard 1" } else { "keyboard 0" })?;
    }
    Ok(())
}

fn note(view: Weak<TextFieldFocusByCode>, text: &'static str) {
    from_main(move || {
        view.state.set_text(text);
    });
}

impl ViewTest for TextFieldFocusByCode {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        from_main(move || view.field.focus());
        if ScreenKeyboard::on_this_device() {
            note(view, "focus by code, a phone opens no keyboard");
            keyboard(false)?;
            from_main(move || {
                ensure!(!view.field.is_editing(), "a focus by code started the editing");
                Ok(())
            })?;
        } else {
            note(view, "focus by code, a desktop edits");
            from_main(move || {
                ensure!(
                    view.field.is_editing(),
                    "a focus by code did not start the editing"
                );
                Ok(())
            })?;
        }
        checkpoint("after a focus by code")?;
        from_main(UIManager::unselect_view);

        tap(FIELD.0, FIELD.1)?;
        wait_until("a tap starts the editing", move || view.field.is_editing())?;
        note(view, "a tap edits, a phone opens the keyboard");
        keyboard(true)?;
        checkpoint("after a tap on the field")?;
        tap(OUTSIDE.0, OUTSIDE.1)?;
        wait_until("a tap outside ends the editing", move || !view.field.is_editing())?;
        keyboard(false)?;

        from_main(move || view.field.focus_with_keyboard());
        wait_until("focus with keyboard starts the editing", move || {
            view.field.is_editing()
        })?;
        note(view, "focus with keyboard edits everywhere");
        keyboard(true)?;
        checkpoint("after focus with keyboard")?;
        from_main(UIManager::unselect_view);
        keyboard(false)?;
        note(view, "done, nothing is edited");

        check_colors(FINAL)
    }
}

const FINAL: &str = r"
              52   72 - #080b0d
              92   72 - #435e71
             116   72 - #141b21
             180   72 - #000000
             212   72 - #597c95
             228   72 - #597c95
             292   72 - #597c95
             152   76 - #000000
             240   76 - #0d1215
             272   76 - #1c262e
             388   92 - #ffffff
             576   92 - #ffffff
             280  112 - #5c5c5c
             284  112 - #ffffff
             288  116 - #ffffff
             304  116 - #ffffff
             280  120 - #5c5c5c
             316  120 - #ffffff
             448  136 - #ffffff
             520  136 - #ffffff
             112  168 - #0c1114
              40  172 - #597c95
             192  172 - #49667a
              64  176 - #000000
              92  176 - #223039
             132  176 - #000001
             172  176 - #597c95
             188  176 - #000000
             592  340 - #597c95
             300  472 - #597c95
               4  592 - #597c95
             592  592 - #597c95
";
