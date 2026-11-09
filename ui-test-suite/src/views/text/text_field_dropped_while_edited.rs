use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Label, Setup, TextAlignment, TextField, ViewData, ViewSubviews, ViewTest, view},
    ui_test::system_input::{system_input, system_request, tap, type_text, wait_until},
};

const FIELD_TOP: f32 = 90.0;
const FIELD_HEIGHT: f32 = 50.0;
const FIELD: (f32, f32) = (300.0, FIELD_TOP + FIELD_HEIGHT / 2.0);

/// A field removed while it is edited, the way a page is left in the middle
/// of typing. On a phone the system text field that stood in for it stayed
/// on the screen with its keyboard, and a field of the next page took no
/// keys. Here the keyboard has to go down with the field, and a new field
/// in its place is typed in as usual.
#[view]
struct TextFieldDroppedWhileEdited {
    first: Weak<TextField>,

    #[init]
    title: Label,
}

impl Setup for TextFieldDroppedWhileEdited {
    fn setup(mut self: Weak<Self>) {
        self.title.set_text("a field removed while it is edited").set_text_size(18);
        self.title.set_alignment(TextAlignment::Left);
        self.title.place().t(60).lr(20).h(26);
        // Made here and not a field of the view, so its removal drops it, the
        // way a page that goes away drops its fields.
        let first = self.add_view::<TextField>();
        place_field(first);
        self.first = first;
    }
}

fn place_field(field: Weak<TextField>) {
    field.set_text_size(24);
    field.place().t(FIELD_TOP).lr(20).h(FIELD_HEIGHT);
}

impl ViewTest for TextFieldDroppedWhileEdited {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        tap(FIELD.0, FIELD.1)?;
        wait_until("the first field is edited", move || view.first.is_editing())?;
        if system_input() {
            system_request("keyboard 1")?;
        }

        from_main(move || {
            let mut first = view.first;
            first.remove_from_superview();
        });
        if system_input() {
            system_request("keyboard 0")?;
        }

        let second = from_main(move || {
            let second = view.add_view::<TextField>();
            place_field(second);
            second
        });
        tap(FIELD.0, FIELD.1)?;
        wait_until("the new field is edited", move || second.is_editing())?;
        type_text("ok")?;
        wait_until("the new field holds the typed text", move || {
            second.text().eq_ignore_ascii_case("ok")
        })?;
        from_main(move || {
            ensure!(
                second.is_editing(),
                "the new field stopped editing while it was typed in"
            );
            Ok(())
        })?;

        tap(300.0, 400.0)?;
        wait_until("the new field is not edited any more", move || {
            !second.is_editing()
        })?;
        if system_input() {
            system_request("keyboard 0")?;
        }
        Ok(())
    }
}
