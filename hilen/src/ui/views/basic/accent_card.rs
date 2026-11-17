use ui_proc::view;

use crate::{
    deps::refs::{Weak, weak_from_ref},
    gm::ToF32,
    ui::{Container, Setup, UIColor, view::ViewData},
};

const BAR_WIDTH: f32 = 4.0;
const RADIUS: f32 = 8.0;
/// The space between the bar and the content an owner places after it.
const CONTENT_GAP: f32 = 8.0;

/// The edge of an `AccentCard` its bar stands at.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BarSide {
    #[default]
    Left,
    Right,
}

/// A rounded block with a bar of color along 1 side, like an event in a
/// calendar or a quote. The bar lies on the edge over the whole height and
/// the card cuts it, and everything else inside, to its round outline, the
/// way `overflow: hidden` does on the web. So the outer side of the bar
/// follows the corners and its inner side is straight, and the card keeps
/// 1 clean outline. The fill and the corner radius of the card are the
/// usual `set_color` and `set_corner_radius`. A 1 point border, with
/// `set_border_color` and `set_border_width`, draws under the bar and
/// outlines the rest. The owner adds its content with `add_view` and
/// starts it at `content_left()`.
#[view]
pub struct AccentCard {
    #[educe(Default = BAR_WIDTH)]
    bar_width: f32,
    side:      BarSide,

    #[init]
    bar: Container,
}

impl AccentCard {
    /// The color of the bar. A `DynamicColor` follows the theme.
    pub fn set_bar_color(&self, color: impl Into<UIColor>) -> &Self {
        self.bar.set_color(color);
        self
    }

    /// How thick the bar is, 4 points by default.
    pub fn set_bar_width(&self, width: impl ToF32) -> &Self {
        weak_from_ref(self).bar_width = width.to_f32().max(0.0);
        self.place_bar();
        self
    }

    pub fn set_bar_side(&self, side: BarSide) -> &Self {
        weak_from_ref(self).side = side;
        self.place_bar();
        self
    }

    /// Where the content of a card with its bar on the left can start, the
    /// bar and a gap after it. A card with the bar on the right ends its
    /// content at this distance from the right edge.
    pub fn content_left(&self) -> f32 {
        self.bar_width + CONTENT_GAP
    }

    fn place_bar(&self) {
        let placer = self.bar.place().clear().tb(0).w(self.bar_width);
        match self.side {
            BarSide::Left => placer.l(0),
            BarSide::Right => placer.r(0),
        };
        self.bar.set_hidden(self.bar_width <= 0.0);
    }
}

impl Setup for AccentCard {
    fn setup(self: Weak<Self>) {
        self.set_corner_radius(RADIUS);
        self.place_bar();
    }

    fn clips_to_bounds(&self) -> bool {
        true
    }
}
