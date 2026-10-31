use anyhow::{Result, ensure};
use hilen::{
    OnceEvent,
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        BLACK, Button, Focus, Label, ModalView, NamedKey, Setup, Size, TextDropDown, UIColor, ViewData,
        ViewFrame, ViewTest, WHITE, view,
    },
    ui_test::{checkpoint, inject_named_key, inject_touches},
};

/// A form with a drop down, its cancel is what Escape ends it with. The
/// shape of a search panel over a player.
#[view]
struct EscapeListForm {
    event: OnceEvent<Option<String>>,

    #[init]
    title: Label,
    drop:  TextDropDown,
    close: Button,
}

impl Setup for EscapeListForm {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE).set_corner_radius(12);
        self.title.set_text("Escape closes the list first").set_text_size(18);
        self.title.place().lrt(10).h(30);

        self.drop.set_values(vec!["One", "Two", "Three"]);
        self.drop.set_text_size(16);
        self.drop
            .set_color(WHITE)
            .set_corner_radius(8)
            .set_border_width(1)
            .set_border_color(BLACK.with_alpha(0.12));
        self.drop.place().t(50).l(20).size(200, 40);

        self.close.set_text("Close").set_text_size(16);
        self.close.set_color("#cfd8e3").set_corner_radius(8);
        self.close.place().br(20).size(100, 40);
        self.close.on_tap(move || self.hide_modal(None));
    }
}

impl ModalView<(), Option<String>> for EscapeListForm {
    fn modal_event(&self) -> &OnceEvent<Option<String>> {
        &self.event
    }

    fn modal_size() -> Size {
        (340, 300).into()
    }

    fn modal_scrim_color() -> UIColor {
        BLACK.with_alpha(0.3).into()
    }

    fn modal_cancel(self: Weak<Self>) -> Option<Option<String>> {
        Some(None)
    }
}

/// Escape with the list of a drop down open closes only the list, with the
/// keys of a remote and with a pointer. The next Escape closes the modal.
#[view]
struct DropDownEscape {
    #[init]
    page: Label,
}

impl Setup for DropDownEscape {
    fn setup(self: Weak<Self>) {
        self.page.set_text("the page behind the form").set_text_size(20);
        self.page.set_color(WHITE);
        self.page.place().back();
    }
}

fn press(key: NamedKey) {
    inject_named_key(key);
    wait_for_next_frame();
    wait_for_next_frame();
}

fn open_form() -> Weak<EscapeListForm> {
    let form = from_main(EscapeListForm::prepare_modally);
    wait_for_next_frame();
    form
}

fn list_open(form: Weak<EscapeListForm>) -> bool {
    from_main(move || form.drop.is_opened())
}

fn ring_on_box(form: Weak<EscapeListForm>) -> bool {
    from_main(move || {
        let ring = Focus::focused();
        ring.is_ok() && ring.absolute_frame().center() == form.drop.absolute_frame().center()
    })
}

impl ViewTest for DropDownEscape {
    fn perform_test(_: Weak<Self>) -> Result<()> {
        let form = open_form();
        press(NamedKey::ArrowRight);
        ensure!(
            ring_on_box(form),
            "the first key did not show the ring on the box"
        );
        press(NamedKey::Enter);
        ensure!(list_open(form), "Enter did not open the list");
        press(NamedKey::ArrowDown);
        press(NamedKey::ArrowDown);
        ensure!(!ring_on_box(form), "the arrows did not walk into the list");
        checkpoint("the list is open with the ring on a row")?;

        press(NamedKey::Escape);
        ensure!(
            from_main(move || form.is_ok()),
            "Escape closed the form with the list"
        );
        ensure!(!list_open(form), "Escape left the list open");
        ensure!(ring_on_box(form), "the ring did not go back to the box");
        checkpoint("Escape closed only the list, the ring is back on the box")?;

        press(NamedKey::Escape);
        ensure!(
            from_main(move || form.is_null()),
            "the second Escape left the form open"
        );
        checkpoint("the second Escape closed the form")?;

        let form = open_form();
        let center = from_main(move || form.drop.absolute_frame().center());
        let (x, y) = (center.x, center.y);
        inject_touches(format!("{x} {y} m\n{x} {y} b\n{x} {y} e"));
        wait_for_next_frame();
        ensure!(list_open(form), "the tap did not open the list");
        checkpoint("a pointer opened the list, no ring shows")?;

        press(NamedKey::Escape);
        ensure!(
            from_main(move || form.is_ok()),
            "Escape closed the form after a tap"
        );
        ensure!(!list_open(form), "Escape left the list open after a tap");
        ensure!(from_main(|| Focus::focused().is_null()), "Escape showed a ring");
        checkpoint("Escape closed only the list, still no ring")?;

        press(NamedKey::Escape);
        ensure!(from_main(move || form.is_null()), "the form stayed open");

        Ok(())
    }
}
