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
        }
    }
}
