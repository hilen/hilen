use crate::{
    self as hilen,
    deps::{refs::Weak, vents::Event},
    ui::{
        ImageView, Setup, Shadow, TouchStack, UIColor, UIImages, View, ViewCallbacks, ViewData, ViewTouch,
        view,
    },
};

pub(super) const BUTTON_SIZE: f32 = 32.0;
const ICON_SIZE: f32 = 18.0;

/// The round button of a `LogView` that jumps to the end of the log.
#[view]
pub(super) struct LogEndButton {
    icon_color: UIColor,

    pub tapped: Event,

    #[init]
    icon: ImageView,
}

impl LogEndButton {
    pub(super) fn set_look(mut self: Weak<Self>, background: UIColor, icon: UIColor) {
        self.set_color(background);
        self.icon_color = icon;
        self.tint();
    }

    fn tint(&self) {
        self.icon.set_image(UIImages::arrow_down(self.icon_color.resolve()));
    }
}

impl ViewCallbacks for LogEndButton {
    /// The icon is an SVG with the color written in, a theme change has
    /// to tint it again.
    fn theme_changed(&mut self) {
        self.tint();
    }
}

impl Setup for LogEndButton {
    fn setup(self: Weak<Self>) {
        self.set_corner_radius(BUTTON_SIZE / 2.0).set_shadow(Shadow::default());
        self.icon.place().center().size(ICON_SIZE, ICON_SIZE);

        // In front of the rows of the log, a row that is made later would
        // take the touch of a plain touch view under it.
        TouchStack::enable_for_high_priority(self.weak_view());
        self.touch().up_inside.sub(self, move || self.tapped.trigger(()));
    }
}
