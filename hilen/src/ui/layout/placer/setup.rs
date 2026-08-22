use std::{ops::Deref, sync::Arc};

use parking_lot::Mutex;

use super::Placer;
use crate::{
    gm::{ToF32, flat::Rect},
    ui::{
        View, WeakView,
        layout::{Anchor, Tiling, layout_rule::LayoutRule},
        view::{ViewFrame, ViewSubviews},
    },
};

impl Placer {
    fn add_size_rule(&self, rule: LayoutRule) {
        if rule.width() {
            self.has().width = true;
            self.fit_text.borrow_mut().width = false;
            self.rules().retain(|r| !r.width());
        } else {
            self.has().height = true;
            self.fit_text.borrow_mut().height = false;
            self.rules().retain(|r| !r.height());
        }
        self.rules().insert(0, rule);
    }

    fn assert_not_self(&self, view: WeakView) {
        assert_ne!(
            view.raw(),
            self.view.weak_view().raw(),
            "Trying to anchor View to itself"
        );
    }

    pub fn size(&self, width: impl ToF32, height: impl ToF32) -> &Self {
        self.view.weak_view().set_size(width, height);
        self.w(width).h(height)
    }

    pub fn same_size(&self, view: impl Deref<Target = impl View> + Copy) -> &Self {
        self.relative(Anchor::Width, view, 1).relative(Anchor::Height, view, 1)
    }

    pub fn same_x(&self, view: impl Deref<Target = impl View>) -> &Self {
        self.anchor(Anchor::X, view, 1)
    }

    pub fn same_y(&self, view: impl Deref<Target = impl View>) -> &Self {
        self.anchor(Anchor::Y, view, 1)
    }

    pub fn same_width(&self, view: impl Deref<Target = impl View>) -> &Self {
        self.anchor(Anchor::Width, view, 1)
    }

    pub fn same_height(&self, view: impl Deref<Target = impl View>) -> &Self {
        self.anchor(Anchor::Height, view, 1)
    }

    pub fn relative_width(&self, view: impl Deref<Target = impl View>, multiplier: impl ToF32) -> &Self {
        self.relative(Anchor::Width, view, multiplier)
    }

    pub fn relative_height(&self, view: impl Deref<Target = impl View>, multiplier: impl ToF32) -> &Self {
        self.relative(Anchor::Height, view, multiplier)
    }

    pub fn relative_size(
        &self,
        view: impl Deref<Target = impl View> + Copy,
        multiplier: impl ToF32,
    ) -> &Self {
        self.relative(Anchor::Width, view, multiplier)
            .relative(Anchor::Height, view, multiplier)
    }

    pub fn relative_x(&self, multiplier: impl ToF32) -> &Self {
        self.relative(Anchor::X, self.view.superview().deref(), multiplier)
    }

    pub fn relative_y(&self, multiplier: impl ToF32) -> &Self {
        self.relative(Anchor::Y, self.view.superview().deref(), multiplier)
    }

    pub fn same<const S: usize>(
        &self,
        anchors: [Anchor; S],
        view: impl Deref<Target = impl View> + Copy,
    ) -> &Self {
        for anchor in anchors {
            let rule = LayoutRule::same(anchor, view.weak_view());

            if anchor.is_width() || anchor.is_height() {
                self.add_size_rule(rule);
            } else {
                self.rules().push(rule);
            }
        }
        self
    }

    pub(crate) fn fit_text_width(&self) -> &Self {
        self.has().width = true;
        self.rules().retain(|r| !r.width());
        self.fit_text.borrow_mut().width = true;
        self
    }

    pub fn fit_text_height(&self) -> &Self {
        self.has().height = true;
        self.rules().retain(|r| !r.height());
        self.fit_text.borrow_mut().height = true;
        self
    }

    pub fn fit_text(&self) -> &Self {
        self.fit_text_width().fit_text_height()
    }

    pub fn w(&self, w: impl ToF32) -> &Self {
        self.add_size_rule(LayoutRule::make(Anchor::Width, w));
        self
    }

    pub fn h(&self, h: impl ToF32) -> &Self {
        self.add_size_rule(LayoutRule::make(Anchor::Height, h));
        self
    }

    pub fn center(&self) -> &Self {
        self.rules().push(Anchor::Center.into());
        self
    }

    pub fn center_x(&self) -> &Self {
        self.rules().push(Anchor::CenterX.into());
        self
    }

    pub fn center_y(&self) -> &Self {
        self.rules().push(Anchor::CenterY.into());
        self
    }

    pub fn center_y_offset(&self, offset: impl ToF32) -> &Self {
        self.rules().push(LayoutRule::make(Anchor::CenterY, offset));
        self
    }

    pub fn back(&self) -> &Self {
        self.rules().push(Tiling::Background.into());
        self
    }

    pub fn left_half(&self) -> &Self {
        self.rules().push(Tiling::LeftHalf.into());
        self
    }

    pub fn right_half(&self) -> &Self {
        self.rules().push(Tiling::RightHalf.into());
        self
    }

    pub fn all_ver(&self) -> &Self {
        self.all_tiling_rules().push(Tiling::Vertically.into());
        self
    }

    pub fn all_hor(&self) -> &Self {
        self.all_tiling_rules().push(Tiling::Horizontally.into());
        self
    }

    /// Subviews flow left to right in declaration order and wrap to the
    /// next row when the width runs out. Children keep their own sizes,
    /// the container height is set to fit all rows.
    pub fn all_wrap(&self) -> &Self {
        self.has().height = true;
        self.all_tiling_rules().push(Tiling::Wrap.into());
        self
    }

    pub fn distribute_ratio<const LEN: usize>(&self, ratios: [impl ToF32; LEN]) -> &Self {
        let ratios: Vec<_> = ratios.iter().map(|ratio| ratio.to_f32()).collect();
        assert!(!ratios.is_empty(), "Distribute ratios cannot be empty");
        assert!(
            ratios.iter().all(|ratio| ratio.is_finite() && *ratio > 0.0),
            "Distribute ratios must be positive finite numbers"
        );
        self.all_tiling_rules().push(Tiling::Distribute(ratios).into());
        self
    }

    pub fn all(&self, margin: impl ToF32) -> &Self {
        *self.all_margin.borrow_mut() = margin.to_f32();
        self
    }
}

impl Placer {
    pub fn max_width(&self, w: impl ToF32) -> &Self {
        self.rules().push(LayoutRule::make(Anchor::MaxWidth, w));
        self
    }

    pub fn max_height(&self, h: impl ToF32) -> &Self {
        self.rules().push(LayoutRule::make(Anchor::MaxHeight, h));
        self
    }

    pub fn min_width(&self, w: impl ToF32) -> &Self {
        self.rules().push(LayoutRule::make(Anchor::MinWidth, w));
        self
    }

    pub fn min_height(&self, w: impl ToF32) -> &Self {
        self.rules().push(LayoutRule::make(Anchor::MinHeight, w));
        self
    }
}

impl Placer {
    pub fn anchor(
        &self,
        side: Anchor,
        view: impl Deref<Target = impl View + ?Sized>,
        offset: impl ToF32,
    ) -> &Self {
        self.assert_not_self(view.weak_view());

        let rule = LayoutRule::anchor(side, offset, view.weak_view());

        if side.is_width() || side.is_height() {
            self.add_size_rule(rule);
        } else {
            self.rules().push(rule);
        }
        self
    }

    pub fn relative(
        &self,
        side: Anchor,
        view: impl Deref<Target = impl View + ?Sized>,
        ratio: impl ToF32,
    ) -> &Self {
        self.assert_not_self(view.weak_view());

        let rule = LayoutRule::relative(side, ratio, view.weak_view());

        if side.is_width() || side.is_height() {
            self.add_size_rule(rule);
        } else {
            self.rules().push(rule);
        }
        self
    }
}

impl Placer {
    pub fn dump_rules(&self) {
        let rules = format!("{:?}", self.get_rules());
        let tiling_rules = format!("{:?}", self.get_tiling_rules());
        println!("Rules: {rules}\nAll tiling rules: {tiling_rules}");
    }
}

impl Placer {
    pub fn above(&self, view: impl Deref<Target = impl View> + Copy, offset: impl ToF32) -> &Self {
        self.same([Anchor::Width, Anchor::Height, Anchor::X], view)
            .anchor(Anchor::Bot, view, offset)
    }

    pub fn below(&self, view: impl Deref<Target = impl View> + Copy, offset: impl ToF32) -> &Self {
        self.same([Anchor::Width, Anchor::Height, Anchor::X], view)
            .anchor(Anchor::Top, view, offset)
    }

    pub fn at_right(&self, view: impl Deref<Target = impl View> + Copy, offset: impl ToF32) -> &Self {
        self.same([Anchor::Width, Anchor::Height, Anchor::CenterY], view)
            .anchor(Anchor::Left, view, offset)
    }

    pub fn between(
        &self,
        view_a: impl Deref<Target = impl View> + Copy,
        view_b: impl Deref<Target = impl View> + Copy,
    ) -> &Self {
        self.rules().push(LayoutRule::between(view_a.weak_view(), view_b.weak_view()));
        self
    }

    pub fn between_super(&self, view: impl Deref<Target = impl View> + Copy, anchor: Anchor) -> &Self {
        self.rules().push(LayoutRule::between_super(view.weak_view(), anchor));
        self
    }

    pub fn custom(&self, custom: impl FnMut(&mut Rect) + Send + 'static) -> &Self {
        *self.custom.borrow_mut() = Some(Arc::new(Mutex::new(custom)));
        self
    }
}
