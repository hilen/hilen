use hilen::{
    refs::Weak,
    ui::{Font, ImageView, Label, Setup, Shadow, TextAlignment, ViewData, ViewTouch, view},
};

use crate::interface::palette::{ACCENT, BORDER, SURFACE, TEXT, TEXT_DIM};

/// A rounded, shadowed tile with an icon, a title and a subtitle. It
/// lifts its border to the accent color on hover and runs an action on
/// tap. The home dashboard is a grid of these.
#[view]
pub struct Card {
    #[init]
    icon:     ImageView,
    title:    Label,
    subtitle: Label,
}

impl Card {
    pub fn set_title(&self, text: &str) -> &Self {
        self.title.set_text(text);
        self
    }

    pub fn set_subtitle(&self, text: &str) -> &Self {
        self.subtitle.set_text(text);
        self
    }

    pub fn set_icon(&self, name: &str) -> &Self {
        self.icon.set_image(name);
        self
    }

    pub fn set_title_font(&self, font: Weak<Font>) -> &Self {
        self.title.set_font(font);
        self
    }

    pub fn on_tap(self: Weak<Self>, action: impl FnMut() + Send + 'static) -> Weak<Self> {
        self.enable_touch();
        self.touch().up_inside.sub(self, action);
        self
    }
}

impl Setup for Card {
    fn setup(self: Weak<Self>) {
        self.set_color(SURFACE)
            .set_corner_radius(16)
            .set_border_width(1)
            .set_border_color(BORDER)
            .set_shadow(Shadow::default());

        self.icon.place().t(16).size(42, 42).center_x();

        self.title
            .set_text_color(TEXT)
            .set_text_size(19)
            .set_alignment(TextAlignment::Center);
        self.title.place().t(64).lr(6).h(30);

        self.subtitle
            .set_text_color(TEXT_DIM)
            .set_text_size(12)
            .set_alignment(TextAlignment::Center);
        self.subtitle.place().t(96).lr(6).h(18);

        self.enable_hover();
        self.touch().hovered.val(self, move |hovered| {
            if hovered {
                self.set_border_color(ACCENT).set_border_width(2);
            } else {
                self.set_border_color(BORDER).set_border_width(1);
            }
        });
    }
}
