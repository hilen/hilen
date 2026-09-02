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

/// What every mesh of a frame shares. `light_dir` is the direction the
/// light travels, unit length.
#[cfg(feature = "scene")]
#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod, PartialEq, Educe)]
#[educe(Default)]
pub struct SceneView {
    #[educe(Default = crate::gm::volume::Mat4::IDENTITY)]
    pub view_proj: crate::gm::volume::Mat4,
    #[educe(Default = crate::gm::volume::Vec3::NEG_Y)]
    pub light_dir: crate::gm::volume::Vec3,
    pub ambient:   f32,
}

#[cfg(all(test, feature = "scene"))]
mod scene_test {
    use super::*;

    #[test]
    fn scene_view_is_a_uniform() {
        assert_eq!(size_of::<SceneView>() % 16, 0);
        assert_eq!(size_of::<SceneView>(), 80);
    }
}
