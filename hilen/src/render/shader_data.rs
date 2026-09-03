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

/// Depth slices of the camera's view that get their own shadow map, see
/// `scene::shadow`. The mesh shader gets the same number prepended.
#[cfg(feature = "scene")]
pub(crate) const SHADOW_CASCADES: usize = 3;

/// The texel sizes of the cascades travel in one `vec4`.
#[cfg(feature = "scene")]
const _: () = assert!(SHADOW_CASCADES <= 4);

/// What every mesh of a frame shares. The vectors carry `xyz`, the
/// fourth component is padding unless named. `camera_pos.w` is the
/// world height of the view one unit from the camera, twice the tangent
/// of half the field of view, what a pixel's size on a surface comes
/// from. `sun_dir` is the direction the light travels, unit length. `sun_color`
/// and `ambient` are linear light, the sun's times its intensity, and
/// `sun_color.w` is 1 when the sun casts shadows, then `sun_view_proj` maps the
/// world into each cascade's layer of the shadow map, `sun_texel` holds the
/// world size of one texel of each and `sun_depth` one over the world length
/// of each map's depth range, so a texel becomes a depth bias. `ambient.w` is
/// 1 when a sky is bound, then `irradiance` holds its nine spherical
/// harmonics and the flat ambient is not used. `viewport` is the width, the
/// height, the start and the size of the depth band, what the fragment stage
/// needs to rebuild a world position from its own coordinates. `fog_color.w` is
/// 1 when the scene has fog, then `fog_range` holds where it starts, one
/// over the length of its fade, see `Fog::range`, and how far up the sky
/// it reaches, see `Fog::height`.
#[cfg(feature = "scene")]
#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod, PartialEq, Educe)]
#[educe(Default)]
pub struct SceneView {
    #[educe(Default = crate::gm::volume::Mat4::IDENTITY)]
    pub view_proj:     crate::gm::volume::Mat4,
    #[educe(Default = crate::gm::volume::Mat4::IDENTITY)]
    pub inv_view_proj: crate::gm::volume::Mat4,
    #[educe(Default = [crate::gm::volume::Mat4::IDENTITY; SHADOW_CASCADES])]
    pub sun_view_proj: [crate::gm::volume::Mat4; SHADOW_CASCADES],
    pub camera_pos:    crate::gm::volume::Vec4,
    #[educe(Default = crate::gm::volume::Vec4::NEG_Y)]
    pub sun_dir:       crate::gm::volume::Vec4,
    pub sun_color:     crate::gm::volume::Vec4,
    pub ambient:       crate::gm::volume::Vec4,
    #[educe(Default = crate::gm::volume::Vec4::new(1.0, 1.0, 0.0, 1.0))]
    pub viewport:      crate::gm::volume::Vec4,
    pub sun_texel:     crate::gm::volume::Vec4,
    pub sun_depth:     crate::gm::volume::Vec4,
    pub fog_color:     crate::gm::volume::Vec4,
    pub fog_range:     crate::gm::volume::Vec4,
    pub irradiance:    [crate::gm::volume::Vec4; 9],
}

#[cfg(all(test, feature = "scene"))]
mod scene_test {
    use super::*;

    #[test]
    fn scene_view_is_a_uniform() {
        assert_eq!(size_of::<SceneView>() % 16, 0);
        assert_eq!(size_of::<SceneView>(), 608);
    }
}
