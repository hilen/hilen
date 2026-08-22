use bytemuck::{Pod, Zeroable};

use crate::gm::ToF32;

/// Corner radii of a rounded rect, one value per corner.
#[repr(C)]
#[derive(Debug, Copy, Clone, Default, PartialEq, Zeroable, Pod)]
pub struct CornerRadii {
    pub top_left:     f32,
    pub top_right:    f32,
    pub bottom_left:  f32,
    pub bottom_right: f32,
}

impl CornerRadii {
    pub fn all(radius: impl ToF32) -> Self {
        let radius = radius.to_f32();
        Self {
            top_left:     radius,
            top_right:    radius,
            bottom_left:  radius,
            bottom_right: radius,
        }
    }

    pub fn top(radius: impl ToF32) -> Self {
        let radius = radius.to_f32();
        Self {
            top_left: radius,
            top_right: radius,
            ..Self::default()
        }
    }

    pub fn bottom(radius: impl ToF32) -> Self {
        let radius = radius.to_f32();
        Self {
            bottom_left: radius,
            bottom_right: radius,
            ..Self::default()
        }
    }
}
