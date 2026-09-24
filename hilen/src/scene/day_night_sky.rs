use crate::gm::volume::{Vec3, Vec4};

/// A sky the shader draws every frame, so a game can move it with its
/// clock: a blue day, an orange glow on the horizon around a low sun,
/// a sun disc, and a dark night with stars and a moon. It replaces the
/// cube `Sky` behind the scene. It lights nothing, the flat `ambient`
/// or a cube `Sky` still does, and the game moves `Sun` with it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DayNightSky {
    /// The direction from the camera to the sun. Its disc and glow show
    /// only while it is above the horizon.
    pub sun:       Vec3,
    /// The direction from the camera to the moon, shown only at night.
    pub moon:      Vec3,
    /// 0 is full day, 1 full night. The sky darkens and the stars and
    /// the moon fade in with it.
    pub night:     f32,
    /// How strong the orange glow is on the horizon around the sun, from
    /// 0 to 1. It fades out with `night` on its own.
    pub sunset:    f32,
    /// A phase in radians that makes the stars twinkle as it moves.
    pub twinkle:   f32,
    /// Picks the pattern of the stars, so each world has its own sky.
    pub star_seed: f32,
}

impl Default for DayNightSky {
    /// Noon, the sun straight up.
    fn default() -> Self {
        Self {
            sun:       Vec3::Y,
            moon:      Vec3::NEG_Y,
            night:     0.0,
            sunset:    0.0,
            twinkle:   0.0,
            star_seed: 0.0,
        }
    }
}

impl DayNightSky {
    /// The three vectors the sky shader reads, see `SceneView`. Zero
    /// when there is no such sky, then the shader draws the cube.
    pub(crate) fn shader_data(sky: Option<&Self>) -> [Vec4; 3] {
        let Some(sky) = sky else {
            return [Vec4::ZERO; 3];
        };
        [
            sky.sun.normalize_or(Vec3::Y).extend(1.0),
            sky.moon.normalize_or(Vec3::NEG_Y).extend(sky.night.clamp(0.0, 1.0)),
            Vec4::new(sky.sunset.clamp(0.0, 1.0), sky.twinkle, sky.star_seed, 0.0),
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // The sky shader tells the two skies apart by the fourth component of
    // the sun, so without this sky it must be zero.
    #[test]
    fn no_sky_is_all_zero() {
        assert_eq!(DayNightSky::shader_data(None), [Vec4::ZERO; 3]);
    }

    #[test]
    fn directions_are_unit_and_factors_clamped() {
        let sky = DayNightSky {
            sun: Vec3::new(0.0, 3.0, 4.0),
            night: 2.0,
            sunset: -1.0,
            ..DayNightSky::default()
        };
        let [sun, moon, params] = DayNightSky::shader_data(Some(&sky));
        assert!((sun.truncate().length() - 1.0).abs() < 1e-6);
        assert!((sun.w - 1.0).abs() < f32::EPSILON);
        assert!((moon.w - 1.0).abs() < f32::EPSILON);
        assert!(params.x.abs() < f32::EPSILON);
    }
}
