use super::look::{ACCENT, BORDER, DIM_TEXT, FAINT_TEXT, HOVER, ON_ACCENT, PANEL, RADIUS, TEXT, TEXT_SIZE};
use crate::{
    self as hilen,
    deps::{refs::Weak, vents::Event},
    gm::color::CLEAR,
    ui::{Button, Container, Label, Setup, TextAlignment, ViewData, view},
};

pub(super) const CHOOSE_BAR_HEIGHT: f32 = 48.0;

const PAD: f32 = 10.0;
const BUTTON_WIDTH: f32 = 88.0;
const BUTTON_HEIGHT: f32 = 30.0;

/// The bar under the list while the browser picks something: what would
/// be picked, Cancel and Choose.
#[view]
pub(super) struct ChooseBar {
    pub cancelled: Event,
    pub chosen:    Event,

    #[init]
    line:              Container,
    pub(super) picked: Label,
    pub(super) cancel: Button,
    pub(super) choose: Button,
}

impl ChooseBar {
    /// The text of what Choose would give, empty while there is nothing,
    /// and Choose is off then.
    pub(super) fn set_picked(mut self: Weak<Self>, text: &str) {
        self.picked.set_text(text);
        self.choose.set_enabled(!text.is_empty());
    }

    pub(super) fn set_choose_title(&self, title: &str) {
        self.choose.set_text(title);
    }
}

impl Setup for ChooseBar {
    fn setup(mut self: Weak<Self>) {
        self.set_color(PANEL);

        self.line.set_color(BORDER);
        self.line.place().t(0).lr(0).h(1);

        // The end of a long path says more than its start.
        self.picked
            .set_color(CLEAR)
            .set_text_size(TEXT_SIZE)
            .set_text_color(DIM_TEXT)
            .set_alignment(TextAlignment::Left)
            .set_ellipsize_head(true);
        self.picked.place().tb(0).l(PAD + 4.0).r(PAD * 3.0 + BUTTON_WIDTH * 2.0);

        self.choose
            .set_text("Choose")
            .set_text_size(TEXT_SIZE)
            .set_text_color(ON_ACCENT);
        self.choose.set_color(ACCENT).set_corner_radius(RADIUS);
        self.choose.set_disabled_color(HOVER).set_disabled_text_color(FAINT_TEXT);
        self.choose.place().r(PAD).center_y().size(BUTTON_WIDTH, BUTTON_HEIGHT);
        self.choose.on_tap(move || self.chosen.trigger(()));
        self.choose.set_enabled(false);

        self.cancel.set_text("Cancel").set_text_size(TEXT_SIZE).set_text_color(TEXT);
        self.cancel.set_color(HOVER).set_corner_radius(RADIUS);
        self.cancel
            .place()
            .r(PAD * 2.0 + BUTTON_WIDTH)
            .center_y()
            .size(BUTTON_WIDTH, BUTTON_HEIGHT);
        self.cancel.on_tap(move || self.cancelled.trigger(()));
    }
}
