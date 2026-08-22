use parking_lot::Mutex;
use ui_proc::view;

// use crate::deps::vents::OnceEvent;
use crate::ui::{Button, Label, ModalView, Setup, view::ViewData};
use crate::{
    deps::{refs::Weak, vents::OnceEvent},
    gm::{
        color::{BLACK, BLUE, GRAY, WHITE},
        flat::Size,
    },
};
#[allow(clippy::type_complexity)]
static LABEL_SETUP: Mutex<Option<Box<dyn FnOnce(Weak<Label>) + Send>>> = Mutex::new(None);

#[view]
pub struct Alert {
    event:     OnceEvent,
    #[init]
    label:     Label,
    ok_button: Button,
}

impl Alert {
    pub fn with_label(label_setup: impl FnOnce(Weak<Label>) + Send + 'static) -> DummyAlert {
        LABEL_SETUP.lock().replace(Box::new(label_setup));
        DummyAlert
    }

    pub fn show(message: impl ToString) {
        Self::show_modally_with_input(message.to_string(), |()| {});
    }

    pub fn show_callback(message: impl ToString, callback: impl FnOnce() + Send + 'static) {
        Self::show_modally_with_input(message.to_string(), move |()| callback());
    }
}

impl Setup for Alert {
    fn setup(self: Weak<Self>) {
        self.set_corner_radius(10).set_border_color(BLACK);
        self.set_color(WHITE);

        self.label.place().lrt(10).h(140);
        self.label.set_text_size(28);
        self.label.set_multiline(true);

        self.ok_button.place().h(28).lrb(-1);
        self.ok_button.set_text("OK").set_border_color(GRAY).set_text_color(BLUE);

        self.ok_button.on_tap(move || self.hide_modal(()));

        if let Some(setup) = LABEL_SETUP.lock().take() {
            setup(self.label);
        }
    }
}

impl ModalView<String> for Alert {
    fn modal_event(&self) -> &OnceEvent<()> {
        &self.event
    }

    fn modal_size() -> Size {
        (280, 200).into()
    }

    fn setup_input(self: Weak<Self>, message: String) {
        self.label.set_text(message);
    }
}

pub struct DummyAlert;

impl DummyAlert {
    pub fn show(&self, message: impl ToString) {
        Alert::show_modally_with_input(message.to_string(), |()| {});
    }

    pub fn show_callback(&self, message: impl ToString, callback: impl FnOnce() + Send + 'static) {
        Alert::show_modally_with_input(message.to_string(), move |()| callback());
    }
}
