// For educe generated Debug impl
#![allow(clippy::used_underscore_binding)]

use educe::Educe;

use crate::{
    gm::ToF32,
    ui::{
        WeakView,
        layout::{Anchor, Tiling},
    },
};

#[derive(Clone, Educe)]
#[educe(Debug)]
pub enum Placement {
    Side {
        side:   Anchor,
        offset: f32,
    },
    Anchor {
        side:   Anchor,
        offset: f32,
        #[educe(Debug(ignore))]
        view:   WeakView,
    },
    Relative {
        side:  Anchor,
        ratio: f32,
        #[educe(Debug(ignore))]
        view:  WeakView,
    },
    Same {
        side: Anchor,
        #[educe(Debug(ignore))]
        view: WeakView,
    },
    Between {
        #[educe(Debug(ignore))]
        a: WeakView,
        #[educe(Debug(ignore))]
        b: WeakView,
    },
    BetweenSuper {
        side: Anchor,
        #[educe(Debug(ignore))]
        view: WeakView,
    },
    Tiling(Tiling),
}

impl Placement {
    fn validate(&self) {
        let (name, side, valid) = match self {
            Self::Side { side, .. } => (
                "Side",
                *side,
                matches!(
                    side,
                    Anchor::Top
                        | Anchor::Bot
                        | Anchor::Left
                        | Anchor::Right
                        | Anchor::Width
                        | Anchor::Height
                        | Anchor::MaxWidth
                        | Anchor::MaxHeight
                        | Anchor::MinWidth
                        | Anchor::MinHeight
                        | Anchor::CenterX
                        | Anchor::CenterY
                        | Anchor::Center
                ),
            ),
            Self::Anchor { side, .. } => (
                "Anchor",
                *side,
                matches!(
                    side,
                    Anchor::Top
                        | Anchor::Bot
                        | Anchor::Left
                        | Anchor::Right
                        | Anchor::Width
                        | Anchor::Height
                        | Anchor::X
                        | Anchor::Y
                ),
            ),
            Self::Relative { side, .. } => (
                "Relative",
                *side,
                matches!(side, Anchor::Width | Anchor::Height | Anchor::X | Anchor::Y),
            ),
            Self::Same { side, .. } => (
                "Same",
                *side,
                matches!(
                    side,
                    Anchor::Width
                        | Anchor::Height
                        | Anchor::CenterX
                        | Anchor::CenterY
                        | Anchor::X
                        | Anchor::Y
                ),
            ),
            Self::BetweenSuper { side, .. } => (
                "BetweenSuper",
                *side,
                matches!(side, Anchor::Top | Anchor::Bot | Anchor::Left | Anchor::Right),
            ),
            Self::Between { .. } | Self::Tiling(_) => return,
        };

        assert!(valid, "{name} placement does not support {side:?}");
    }
}

impl PartialEq for Placement {
    fn eq(&self, other: &Self) -> bool {
        fn same_view(a: &WeakView, b: &WeakView) -> bool {
            a.raw() == b.raw()
        }

        match (self, other) {
            (
                Self::Side { side, offset },
                Self::Side {
                    side: o_side,
                    offset: o_offset,
                },
            ) => side == o_side && offset == o_offset,
            (
                Self::Anchor { side, offset, view },
                Self::Anchor {
                    side: o_side,
                    offset: o_offset,
                    view: o_view,
                },
            ) => side == o_side && offset == o_offset && same_view(view, o_view),
            (
                Self::Relative { side, ratio, view },
                Self::Relative {
                    side: o_side,
                    ratio: o_ratio,
                    view: o_view,
                },
            ) => side == o_side && ratio == o_ratio && same_view(view, o_view),
            (
                Self::Same { side, view },
                Self::Same {
                    side: o_side,
                    view: o_view,
                },
            )
            | (
                Self::BetweenSuper { side, view },
                Self::BetweenSuper {
                    side: o_side,
                    view: o_view,
                },
            ) => side == o_side && same_view(view, o_view),
            (Self::Between { a, b }, Self::Between { a: o_a, b: o_b }) => {
                same_view(a, o_a) && same_view(b, o_b)
            }
            (Self::Tiling(tiling), Self::Tiling(o_tiling)) => tiling == o_tiling,
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutRule {
    pub placement: Placement,
    pub enabled:   bool,
}

impl LayoutRule {
    fn new(placement: Placement) -> Self {
        placement.validate();
        Self {
            placement,
            enabled: true,
        }
    }

    pub(crate) fn from_placement(placement: Placement, enabled: bool) -> Self {
        placement.validate();
        Self { placement, enabled }
    }

    pub(crate) fn make(side: Anchor, offset: impl ToF32) -> Self {
        Self::new(Placement::Side {
            side,
            offset: offset.to_f32(),
        })
    }

    pub fn anchor(side: Anchor, offset: impl ToF32, view: WeakView) -> Self {
        Self::new(Placement::Anchor {
            side,
            offset: offset.to_f32(),
            view,
        })
    }

    pub(crate) fn relative(side: Anchor, ratio: impl ToF32, view: WeakView) -> Self {
        Self::new(Placement::Relative {
            side,
            ratio: ratio.to_f32(),
            view,
        })
    }

    pub fn same(side: Anchor, view: WeakView) -> Self {
        Self::new(Placement::Same { side, view })
    }

    pub fn between(a: WeakView, b: WeakView) -> Self {
        Self::new(Placement::Between { a, b })
    }

    pub fn between_super(view: WeakView, side: Anchor) -> Self {
        Self::new(Placement::BetweenSuper { side, view })
    }
}

impl LayoutRule {
    pub(crate) fn side(&self) -> Option<Anchor> {
        match &self.placement {
            Placement::Side { side, .. }
            | Placement::Anchor { side, .. }
            | Placement::Relative { side, .. }
            | Placement::Same { side, .. }
            | Placement::BetweenSuper { side, .. } => Some(*side),
            Placement::Between { .. } | Placement::Tiling(_) => None,
        }
    }

    pub fn offset(&self) -> f32 {
        match &self.placement {
            Placement::Side { offset, .. } | Placement::Anchor { offset, .. } => *offset,
            Placement::Relative { ratio, .. } => *ratio,
            _ => 0.0,
        }
    }

    pub fn set_offset(&mut self, offset: impl ToF32) {
        let value = offset.to_f32();
        match &mut self.placement {
            Placement::Side { offset, .. } | Placement::Anchor { offset, .. } => *offset = value,
            Placement::Relative { ratio, .. } => *ratio = value,
            _ => {}
        }
    }

    pub fn width(&self) -> bool {
        self.side().is_some_and(Anchor::is_width)
    }

    pub fn height(&self) -> bool {
        self.side().is_some_and(Anchor::is_height)
    }
}

impl From<Anchor> for LayoutRule {
    fn from(anchor: Anchor) -> Self {
        Self::make(anchor, 0)
    }
}

impl From<Tiling> for LayoutRule {
    fn from(tiling: Tiling) -> Self {
        Self::new(Placement::Tiling(tiling))
    }
}

#[cfg(test)]
mod tests {
    use super::{Anchor, LayoutRule};
    use crate::deps::refs::Weak;

    #[test]
    #[should_panic(expected = "Anchor placement does not support Center")]
    fn invalid_anchor_panics_at_creation() {
        LayoutRule::anchor(Anchor::Center, 0, Weak::default());
    }

    #[test]
    #[should_panic(expected = "Same placement does not support Top")]
    fn invalid_same_panics_at_creation() {
        LayoutRule::same(Anchor::Top, Weak::default());
    }

    #[test]
    #[should_panic(expected = "BetweenSuper placement does not support Center")]
    fn invalid_between_super_panics_at_creation() {
        LayoutRule::between_super(Weak::default(), Anchor::Center);
    }
}
