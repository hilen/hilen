use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum Tiling {
    Background,

    Horizontally,
    Vertically,

    LeftHalf,
    RightHalf,

    Distribute(Vec<f32>),

    Wrap,
}
