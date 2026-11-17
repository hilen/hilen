use super::look::{ACCENT, DIM_TEXT, FAINT_TEXT, HOVER, Icon, RADIUS};
use crate::{
    self as hilen,
    deps::{refs::Weak, vents::Event},
    gm::color::CLEAR,
    ui::{ImageView, Setup, UIColor, ViewCallbacks, ViewData, ViewTouch, view},
};

const ICON_SIZE: f32 = 16.0;

/// A small square button with one icon: back, forward, the view switch.
/// `lit` is the look of a switch that is on.
#[view]
pub(super) struct IconButton {
    icon: Option<Icon>,

    #[educe(Default = true)]
    enabled: bool,
    lit:     bool,

    pub tapped: Event,

    #[init]
    image: ImageView,
}

impl IconButton {
    pub(super) fn set_icon(mut self: Weak<Self>, icon: Icon) {
        self.icon = Some(icon);
        self.refresh();
    }

    pub(super) fn set_enabled(mut self: Weak<Self>, enabled: bool) {
        self.enabled = enabled;
        self.refresh();
    }

    pub(super) fn set_lit(mut self: Weak<Self>, lit: bool) {
        self.lit = lit;
        self.refresh();
    }

    fn refresh(&self) {
        let tint = if !self.enabled {
            FAINT_TEXT
        } else if self.lit {
            ACCENT
        } else {
            DIM_TEXT
        };
        if let Some(icon) = self.icon {
            self.image.set_image(icon.image(tint.resolve()));
        }

        let background: UIColor = if self.enabled && self.is_hovered() {
            HOVER.into()
        } else {
            CLEAR.into()
        };
        self.set_color(background);
    }
}

impl ViewCallbacks for IconButton {
    /// The icon is an SVG with the color written in, a theme change has
    /// to tint it again.
    fn theme_changed(&mut self) {
        self.refresh();
    }
}

impl Setup for IconButton {
    fn setup(self: Weak<Self>) {
        self.set_color(CLEAR).set_corner_radius(RADIUS);
        self.image.place().center().size(ICON_SIZE, ICON_SIZE);

        self.enable_touch();
        self.enable_hover();
        self.touch().hovered.sub(self, move || self.refresh());
        self.touch().up_inside.sub(self, move || {
            if self.enabled {
                self.tapped.trigger(());
            }
        });
    }
}
