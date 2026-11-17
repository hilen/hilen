use ui_proc::view;

use crate::{
    self as hilen,
    deps::{refs::Weak, vents::Event},
    gm::color::BLACK,
    image::Image,
    ui::{ImageView, Setup, ViewData, ViewTouch},
};

const ICON: f32 = 24.0;

/// A round dark button with a white icon. It keeps its fill over any
/// picture, a bright one too, where a clear button would vanish.
#[view]
pub(super) struct ViewerButton {
    pub(super) tapped: Event,

    #[init]
    icon: ImageView,
}

impl ViewerButton {
    pub(super) fn set_icon(&self, icon: Weak<Image>) {
        self.icon.set_image(icon);
    }

    fn paint(self: Weak<Self>, hovered: bool) {
        let alpha = if hovered { 0.8 } else { 0.55 };
        self.set_color(BLACK.with_alpha(alpha));
    }
}

impl Setup for ViewerButton {
    fn setup(self: Weak<Self>) {
        self.icon.place().center().size(ICON, ICON);
        self.paint(false);
        self.enable_touch();
        self.enable_hover();
        self.touch().hovered.val(self, move |hovered| self.paint(hovered));
        self.touch().up_inside.sub(self, move || self.tapped.trigger(()));
    }
}
