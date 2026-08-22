use bytemuck::{Pod, Zeroable};
use educe::Educe;

use crate::gm::flat::{Point, Size};

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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        // Web requirements
        assert_eq!(size_of::<SpriteView>() % 16, 0);
    }
}
