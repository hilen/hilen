use super::look::{ACCENT, BACKGROUND, BORDER, FIELD, HOVER, ON_ACCENT, RADIUS, TEXT, TEXT_SIZE};
use crate::{
    self as hilen,
    deps::{refs::Weak, vents::OnceEvent},
    gm::{
        color::{BLACK, CLEAR},
        flat::Size,
    },
    ui::{Button, Label, ModalView, Setup, TextAlignment, TextField, UIColor, ViewData, view},
};

const PAD: f32 = 16.0;
const BUTTON_WIDTH: f32 = 88.0;
const BUTTON_HEIGHT: f32 = 30.0;

/// What a [`NamePrompt`] opens with.
pub(super) struct NameRequest {
    pub title:   String,
    /// The name the field starts with.
    pub name:    String,
    /// The title of the confirm button.
    pub confirm: String,
}

/// A small dialog that asks for one name, for a new folder and a rename.
/// It gives the trimmed name, or `None` on Cancel, Escape and an empty
/// field.
#[view]
pub(super) struct NamePrompt {
    event: OnceEvent<Option<String>>,

    #[init]
    title:              Label,
    pub(super) field:   TextField,
    cancel:             Button,
    pub(super) confirm: Button,
}

impl NamePrompt {
    fn answer(self: Weak<Self>) {
        let name = self.field.text().trim().to_string();
        self.hide_modal((!name.is_empty()).then_some(name));
    }
}

impl ModalView<NameRequest, Option<String>> for NamePrompt {
    fn modal_event(&self) -> &OnceEvent<Option<String>> {
        &self.event
    }

    fn modal_size() -> Size {
        (340, 148).into()
    }

    fn modal_scrim_color() -> UIColor {
        BLACK.with_alpha(0.25).into()
    }

    fn modal_cancel(self: Weak<Self>) -> Option<Option<String>> {
        Some(None)
    }

    fn setup_input(self: Weak<Self>, request: NameRequest) {
        self.title.set_text(request.title);
        self.confirm.set_text(request.confirm);
        self.field.set_text(request.name);
        self.field.focus_with_keyboard();
    }
}

impl Setup for NamePrompt {
    fn setup(self: Weak<Self>) {
        self.set_color(BACKGROUND)
            .set_corner_radius(RADIUS + 4.0)
            .set_border_width(1)
            .set_border_color(BORDER);

        self.title
            .set_color(CLEAR)
            .set_text_size(TEXT_SIZE + 1.0)
            .set_text_color(TEXT)
            .set_alignment(TextAlignment::Left);
        self.title.place().t(PAD - 4.0).lr(PAD).h(24);

        self.field
            .set_text_size(TEXT_SIZE)
            .set_text_color(TEXT)
            .set_selected_color(FIELD);
        let mut field = self.field;
        field.set_alignment(TextAlignment::Left);
        self.field
            .set_color(FIELD)
            .set_corner_radius(RADIUS)
            .set_border_width(1)
            .set_border_color(BORDER);
        self.field.place().t(PAD + 28.0).lr(PAD).h(30);
        self.field.submitted.sub(move || self.answer());

        self.confirm.set_text_size(TEXT_SIZE).set_text_color(ON_ACCENT);
        self.confirm.set_color(ACCENT).set_corner_radius(RADIUS);
        self.confirm.place().r(PAD).b(PAD).size(BUTTON_WIDTH, BUTTON_HEIGHT);
        self.confirm.on_tap(move || self.answer());

        self.cancel.set_text("Cancel").set_text_size(TEXT_SIZE).set_text_color(TEXT);
        self.cancel.set_color(HOVER).set_corner_radius(RADIUS);
        self.cancel
            .place()
            .r(PAD * 1.5 + BUTTON_WIDTH)
            .b(PAD)
            .size(BUTTON_WIDTH, BUTTON_HEIGHT);
        self.cancel.on_tap(move || self.hide_modal(None));
    }
}
