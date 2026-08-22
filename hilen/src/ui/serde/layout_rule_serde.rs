use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::ui::{
    LayoutRule, WeakView,
    layout::{Anchor, Placement, Tiling},
    serde::weak_serde::WeakRepr,
};

#[derive(Serialize, Deserialize)]
enum PlacementRepr {
    Side {
        side:   Anchor,
        offset: f32,
    },
    Anchor {
        side:   Anchor,
        offset: f32,
        view:   WeakRepr,
    },
    Relative {
        side:  Anchor,
        ratio: f32,
        view:  WeakRepr,
    },
    Same {
        side: Anchor,
        view: WeakRepr,
    },
    Between {
        a: WeakRepr,
        b: WeakRepr,
    },
    BetweenSuper {
        side: Anchor,
        view: WeakRepr,
    },
    Tiling(Tiling),
}

#[derive(Serialize, Deserialize)]
struct LayoutRuleRepr {
    placement: PlacementRepr,
    enabled:   bool,
}

impl From<&LayoutRule> for LayoutRuleRepr {
    fn from(rule: &LayoutRule) -> Self {
        let placement = match &rule.placement {
            Placement::Side { side, offset } => PlacementRepr::Side {
                side:   *side,
                offset: *offset,
            },
            Placement::Anchor { side, offset, view } => PlacementRepr::Anchor {
                side:   *side,
                offset: *offset,
                view:   (*view).into(),
            },
            Placement::Relative { side, ratio, view } => PlacementRepr::Relative {
                side:  *side,
                ratio: *ratio,
                view:  (*view).into(),
            },
            Placement::Same { side, view } => PlacementRepr::Same {
                side: *side,
                view: (*view).into(),
            },
            Placement::Between { a, b } => PlacementRepr::Between {
                a: (*a).into(),
                b: (*b).into(),
            },
            Placement::BetweenSuper { side, view } => PlacementRepr::BetweenSuper {
                side: *side,
                view: (*view).into(),
            },
            Placement::Tiling(tiling) => PlacementRepr::Tiling(tiling.clone()),
        };

        Self {
            placement,
            enabled: rule.enabled,
        }
    }
}

impl From<LayoutRuleRepr> for LayoutRule {
    fn from(repr: LayoutRuleRepr) -> Self {
        let placement = match repr.placement {
            PlacementRepr::Side { side, offset } => Placement::Side { side, offset },
            PlacementRepr::Anchor { side, offset, view } => Placement::Anchor {
                side,
                offset,
                view: WeakView::from(view),
            },
            PlacementRepr::Relative { side, ratio, view } => Placement::Relative {
                side,
                ratio,
                view: WeakView::from(view),
            },
            PlacementRepr::Same { side, view } => Placement::Same {
                side,
                view: WeakView::from(view),
            },
            PlacementRepr::Between { a, b } => Placement::Between {
                a: WeakView::from(a),
                b: WeakView::from(b),
            },
            PlacementRepr::BetweenSuper { side, view } => Placement::BetweenSuper {
                side,
                view: WeakView::from(view),
            },
            PlacementRepr::Tiling(tiling) => Placement::Tiling(tiling),
        };

        Self::from_placement(placement, repr.enabled)
    }
}

impl Serialize for LayoutRule {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        LayoutRuleRepr::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for LayoutRule {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        Ok(LayoutRuleRepr::deserialize(deserializer)?.into())
    }
}
