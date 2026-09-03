use bytemuck::{Pod, Zeroable};
use educe::Educe;

#[cfg(feature = "level")]
use crate::gm::flat::{Point, Size};

#[cfg(feature = "level")]
#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod, PartialEq, Educe)]
#[educe(Default)]
pub struct SpriteView {
    pub camera_pos:      Point,
    #[educe(Default = (1000, 1000).into())]
    pub resolution:      Size,
    pub camera_rotation: f32,
    #[educe(Default = 1.0)]
    pub scale:           f32,
    #[allow(clippy::pub_underscore_fields)]
    pub _padding:        u64,
}

#[cfg(all(test, feature = "level"))]
mod test {
    use super::*;

    #[test]
    fn test() {
        // Web requirements
        assert_eq!(size_of::<SpriteView>() % 16, 0);
    }
}

/// What every mesh of a frame shares. The vectors carry `xyz`, the
/// fourth component is padding unless named. `sun_dir` is the direction
/// the light travels, unit length, its `w` the world size of one shadow
/// map texel. `sun_color` and `ambient` are linear light, the sun's
/// times its intensity, and `sun_color.w` is 1 when the sun casts
/// shadows, then `sun_view_proj` maps the world into the shadow map.
/// `ambient.w` is 1 when a sky is bound, then `irradiance` holds its
/// nine spherical harmonics and the flat ambient is not used.
/// `viewport` is the width, the height, the start and the size of the
/// depth band, what the fragment stage needs to rebuild a world
/// position from its own coordinates.
#[cfg(feature = "scene")]
#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod, PartialEq, Educe)]
#[educe(Default)]
pub struct SceneView {
    #[educe(Default = crate::gm::volume::Mat4::IDENTITY)]
    pub view_proj:     crate::gm::volume::Mat4,
    #[educe(Default = crate::gm::volume::Mat4::IDENTITY)]
    pub inv_view_proj: crate::gm::volume::Mat4,
    #[educe(Default = crate::gm::volume::Mat4::IDENTITY)]
    pub sun_view_proj: crate::gm::volume::Mat4,
    pub camera_pos:    crate::gm::volume::Vec4,
    #[educe(Default = crate::gm::volume::Vec4::NEG_Y)]
    pub sun_dir:       crate::gm::volume::Vec4,
    pub sun_color:     crate::gm::volume::Vec4,
    pub ambient:       crate::gm::volume::Vec4,
    #[educe(Default = crate::gm::volume::Vec4::new(1.0, 1.0, 0.0, 1.0))]
    pub viewport:      crate::gm::volume::Vec4,
    pub irradiance:    [crate::gm::volume::Vec4; 9],
}

#[cfg(all(test, feature = "scene"))]
mod scene_test {
    use super::*;

    #[test]
    fn scene_view_is_a_uniform() {
        assert_eq!(size_of::<SceneView>() % 16, 0);
        assert_eq!(size_of::<SceneView>(), 416);
    }
}
