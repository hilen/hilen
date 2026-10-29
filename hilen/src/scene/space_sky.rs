use crate::{gm::volume::Quat, scene::Atmosphere};

/// The sky of a world in space, drawn in place of `sky` and
/// `day_night_sky`: the air of a planet seen from its ground or from
/// above, or the black sky of a moon with no air, with stars that turn
/// with the planet and the sun's disc. With air it lights the scene too,
/// see `Atmosphere`, so the sun reddens as it sets, far land fades into
/// the blue and a night is dark. The shadows of the `Occluder`s fall on
/// the air, so an eclipse darkens the sky.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpaceSky {
    pub atmosphere:      Option<Atmosphere>,
    /// The turn from the stars' own axes to the world. A game turns it
    /// with its planet, so the stars wheel round the sky in a night.
    pub stars:           Quat,
    /// How bright the brightest star is, 0 for none. A bright sky hides
    /// them, as the day does.
    pub star_brightness: f32,
    /// Picks the pattern of the stars.
    pub star_seed:       f32,
    /// Draws the sun's disc, `Sun::angular_radius` wide.
    pub sun_disc:        bool,
    /// How bright the glare round the sun is, 0 for none. The real sun is
    /// a small disc, the eye and a camera see it big for the glare: a
    /// bright core that falls with the square of the angle and a wide dim
    /// halo. The share of the disc the occluders hide dims it, so a sun
    /// behind a planet has none. It lights the sky only, not the nodes.
    pub sun_glare:       f32,
    /// How much brighter the light of the air is than the physics gives,
    /// 1 for none: the sky, the haze over far land and the sky's light in
    /// the shade. A game whose ground colors are picked for a sun of 1
    /// lifts its sky to a day as bright as Earth's with about 5, the
    /// ground keeps its colors. The night stays dark, the air sends almost
    /// no light then.
    pub air_brightness:  f32,
}

impl Default for SpaceSky {
    /// No air, stars and the sun.
    fn default() -> Self {
        Self {
            atmosphere:      None,
            stars:           Quat::IDENTITY,
            star_brightness: 1.0,
            star_seed:       0.0,
            sun_disc:        true,
            sun_glare:       0.0,
            air_brightness:  1.0,
        }
    }
}
