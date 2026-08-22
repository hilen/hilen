use crate::gm::{
    color::{BLACK, Color},
    flat::Point,
};

/// A drop shadow drawn under the view's rounded rect shape. Desktop
/// look reference is a CSS box shadow: `offset` shifts the shadow,
/// `radius` is the blur distance across the edge.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Shadow {
    pub offset: Point,
    pub radius: f32,
    pub color:  Color,
}

impl Default for Shadow {
    fn default() -> Self {
        Self {
            offset: Point::new(0.0, 3.0),
            radius: 12.0,
            color:  BLACK.with_alpha(0.5),
        }
    }
}
