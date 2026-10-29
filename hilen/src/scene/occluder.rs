use crate::gm::{color::Color, volume::Vec3};

/// The most occluders a scene draws with, see `SceneBase::occluders`.
pub(crate) const MAX_OCCLUDERS: usize = 4;

/// A ball that hides the sun and the directional lights from what lies
/// behind it: a planet, so its night side is dark and a mountain keeps
/// the sun longer than the valley under it, or a moon, whose shadow
/// crosses the planet in an eclipse. Each pixel takes the share of the
/// light's disc the ball covers, so the edge of a shadow is as soft as
/// the light is wide, see `Sun::angular_radius`. It casts no shadow map,
/// the shadows of trees and hills stay the sun's maps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Occluder {
    pub center:   Vec3,
    pub radius:   f32,
    /// The light that still gets through at full cover, black for none.
    /// A planet with air bends a little red light into its shadow, the
    /// copper moon of a lunar eclipse.
    pub transmit: Color,
}

impl Occluder {
    pub fn new(center: impl Into<Vec3>, radius: f32) -> Self {
        Self {
            center: center.into(),
            radius,
            transmit: Color::rgb(0.0, 0.0, 0.0),
        }
    }

    pub fn transmit(mut self, transmit: Color) -> Self {
        self.transmit = transmit;
        self
    }
}
