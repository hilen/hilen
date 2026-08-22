use serde::{Deserialize, Serialize};

#[derive(Debug, Default, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub enum Anchor {
    Top,
    Bot,

    Left,
    Right,

    Width,
    Height,

    MaxWidth,
    MaxHeight,

    MinWidth,
    MinHeight,

    CenterX,
    CenterY,

    Center,

    X,
    Y,

    #[default]
    None,
}

impl Anchor {
    pub(crate) fn is_width(self) -> bool {
        matches!(self, Self::Width)
    }

    pub(crate) fn is_height(self) -> bool {
        matches!(self, Self::Height)
    }

    pub(crate) fn is_left(self) -> bool {
        matches!(self, Self::Left)
    }

    pub(crate) fn is_top(self) -> bool {
        matches!(self, Self::Top)
    }
}
