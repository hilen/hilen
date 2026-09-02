use parking_lot::Mutex;
use ui_proc::view;

use crate::{
    deps::{refs::Weak, vents::OnceEvent},
    gm::{
        color::{BLACK, CLEAR, Color},
        flat::Size,
    },
    ui::{Button, Container, Label, ModalView, Setup, UIColor, view::ViewData},
};

const ALERT_WIDTH: f32 = 270.0;
const PADDING: f32 = 20.0;
const BUTTON_HEIGHT: f32 = 44.0;
const MIN_TEXT_HEIGHT: f32 = 22.0;
const MAX_TEXT_HEIGHT: f32 = 400.0;

const BACKGROUND: Color = Color::hex("#f9f9f9");
const SEPARATOR: Color = Color::hex("#c6c6c8");
const ACTION_BLUE: Color = Color::hex("#007aff");
const MESSAGE_COLOR: Color = Color::hex("#1c1c1e");

#[allow(clippy::type_complexity)]
static LABEL_SETUP: Mutex<Option<Box<dyn FnOnce(Weak<Label>) + Send>>> = Mutex::new(None);

#[view]
pub struct Alert {
    event:     OnceEvent,
    #[init]
    label:     Label,
    separator: Container,
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
        self.set_corner_radius(14);
        self.set_color(BACKGROUND);

        self.label.set_text_size(15);
        self.label.set_text_color(MESSAGE_COLOR);
        self.label.set_multiline(true);
        self.label.place().lrt(PADDING).h(MIN_TEXT_HEIGHT);

        self.separator.set_color(SEPARATOR);
        self.separator.place().lr(0).b(BUTTON_HEIGHT).h(1);

        self.ok_button.set_text("OK").set_text_size(17).set_text_color(ACTION_BLUE);
        self.ok_button.set_color(CLEAR);
        self.ok_button.place().lrb(0).h(BUTTON_HEIGHT);

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
        (ALERT_WIDTH, 150.0).into()
    }

    fn modal_scrim_color() -> UIColor {
        BLACK.with_alpha(0.25).into()
    }

    fn setup_input(self: Weak<Self>, message: String) {
        self.label.set_text(message);

        // modal_size cannot see the message, so the alert resizes to its
        // text here, the way an iOS alert grows with its content.
        let text_height = self
            .label
            .size_for_width(ALERT_WIDTH - PADDING * 2.0)
            .height
            .clamp(MIN_TEXT_HEIGHT, MAX_TEXT_HEIGHT);

        self.label.place().clear().lrt(PADDING).h(text_height);
        self.place()
            .clear()
            .size(ALERT_WIDTH, PADDING + text_height + PADDING + BUTTON_HEIGHT)
            .center();
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
