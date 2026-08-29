use hilen::{
    refs::{Weak, manage::DataManager},
    ui::{Container, Font, Label, Setup, TextAlignment, ViewData, view},
};

use crate::interface::palette::{ACCENT_END, ACCENT_START, TEXT};

/// The first screen, a big title with the accent bar under it. The
/// sidebar is the menu, so nothing is repeated here.
#[view]
pub struct Landing {
    #[init]
    title: Label,
    bar:   Container,
}

impl Setup for Landing {
    fn setup(self: Weak<Self>) {
        self.title
            .set_text("Hilen")
            .set_text_color(TEXT)
            .set_text_size(64)
            .set_font(Font::get("RussoOne-Regular.ttf"))
            .set_alignment(TextAlignment::Left);
        self.title.place().l(32).t(40).r(32).h(76);

        self.bar.set_gradient(ACCENT_START, ACCENT_END).set_corner_radius(3);
        self.bar.place().l(34).t(122).size(96, 6);
    }
}
