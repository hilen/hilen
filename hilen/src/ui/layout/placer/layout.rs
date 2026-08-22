use std::ops::Deref;

use super::Placer;
use crate::{
    deps::refs::{Own, ToRglica},
    gm::{
        LossyConvert,
        axis::Axis,
        flat::{Rect, Size},
    },
    ui::{
        Label, View, ViewSubviews, WeakView,
        layout::{
            Anchor, Tiling,
            layout_rule::{LayoutRule, Placement},
        },
        view::{ViewData, ViewFrame, ViewLayout},
    },
};

type RMut<'a> = &'a mut Rect;

impl Placer {
    pub(crate) fn layout(&mut self) {
        if self.view.is_null() {
            return;
        }

        let this = self.to_rglica();

        let has_left = self.has_left();
        let has_top = self.has_top();

        let old_frame = *self.view.frame();
        let mut new_frame = old_frame;

        for rule in this.rules().iter().filter(|r| r.enabled) {
            self.apply_rule(&mut new_frame, rule, has_left, has_top);
        }

        for rule in this.all_tiling_rules().iter().filter(|r| r.enabled) {
            let Placement::Tiling(tiling) = &rule.placement else {
                unreachable!("Only tiling rules are allowed in all_tiling_rules")
            };
            self.tiling_layout(&mut new_frame, tiling);
        }

        if self.fit_text_layout(&mut new_frame) {
            // Position rules run again so centers and edges see the fitted
            // size. The has flags are set, so no rule resizes here.
            for rule in this.rules().iter().filter(|r| r.enabled) {
                self.apply_rule(&mut new_frame, rule, has_left, has_top);
            }
        }

        if let Some(custom) = self.custom.borrow().as_ref() {
            custom.lock()(&mut new_frame);
        }

        if new_frame != old_frame {
            self.view.set_frame(new_frame);
        }
    }

    fn apply_rule(&mut self, frame: RMut, rule: &LayoutRule, has_left: bool, has_top: bool) {
        match &rule.placement {
            Placement::Side { side, offset } => self.simple_layout(frame, *side, *offset),
            Placement::Anchor { side, offset, view } => {
                anchor_layout(frame, *side, *offset, *view, has_left, has_top);
            }
            Placement::Relative { side, ratio, view } => {
                relative_layout(frame, *side, *ratio, *view);
            }
            Placement::Same { side, view } => same_layout(frame, *side, *view),
            Placement::Between { a, b } => between_2_layout(frame, *a, *b),
            Placement::BetweenSuper { side, view } => {
                self.between_super_layout(frame, *side, *view);
            }
            Placement::Tiling(tiling) => self.tiling_layout(frame, tiling),
        }
    }

    fn fit_text_layout(&self, frame: RMut) -> bool {
        let fit = *self.fit_text.borrow();

        if !fit.width && !fit.height {
            return false;
        }

        let Some(label) = self.view.as_any().downcast_ref::<Label>() else {
            return false;
        };

        let size = label.size_for_width(frame.width());
        let mut changed = false;

        if fit.width && (frame.width() - size.width).abs() > f32::EPSILON {
            frame.size.width = size.width;
            changed = true;
        }

        if fit.height && (frame.height() - size.height).abs() > f32::EPSILON {
            frame.size.height = size.height;
            changed = true;
        }

        changed
    }

    fn simple_layout(&mut self, frame: RMut, side: Anchor, offset: f32) {
        let has = *self.has();
        let s_content = self.s_content.deref();

        match side {
            Anchor::Top => {
                if !has.height {
                    frame.size.height = frame.max_y() - offset;
                }

                frame.origin.y = offset;
            }
            Anchor::Bot => {
                if has.height {
                    frame.origin.y = s_content.height - frame.height() - offset;
                } else {
                    frame.size.height = frame.height() + s_content.height - frame.max_y() - offset;
                }
            }
            Anchor::Left => {
                if !has.width {
                    frame.size.width = frame.max_x() - offset;
                }

                frame.origin.x = offset;
            }
            Anchor::Right => {
                if has.width {
                    frame.origin.x = s_content.width - frame.width() - offset;
                } else {
                    frame.size.width = s_content.width - frame.origin.x - offset;
                }
            }
            Anchor::Width => frame.size.width = offset,
            Anchor::Height => frame.size.height = offset,
            Anchor::CenterX => frame.origin.x = s_content.width / 2.0 - frame.width() / 2.0,
            Anchor::CenterY => frame.origin.y = s_content.height / 2.0 - frame.height() / 2.0 + offset,
            Anchor::Center => {
                frame.origin.x = s_content.width / 2.0 - frame.width() / 2.0;
                frame.origin.y = s_content.height / 2.0 - frame.height() / 2.0;
            }
            Anchor::MaxWidth => {
                if frame.size.width > offset {
                    frame.size.width = offset;
                }
            }
            Anchor::MaxHeight => {
                if frame.size.height > offset {
                    frame.size.height = offset;
                }
            }
            Anchor::MinWidth => {
                if frame.size.width < offset {
                    frame.size.width = offset;
                }
            }
            Anchor::MinHeight => {
                if frame.size.height < offset {
                    frame.size.height = offset;
                }
            }
            Anchor::X | Anchor::Y | Anchor::None => {
                unimplemented!("Simple layout for {side:?} is not supported")
            }
        }
    }

    fn tiling_layout(&mut self, frame: RMut, tiling: &Tiling) {
        let s_content = *self.s_content.deref();

        match tiling {
            Tiling::Background => *frame = s_content.into(),
            Tiling::Horizontally => place_horizontally(self.view.subviews(), *self.all_margin.borrow()),
            Tiling::Vertically => place_vertically(self.view.subviews(), *self.all_margin.borrow()),
            Tiling::LeftHalf => *frame = (0, 0, s_content.width / 2.0, s_content.height).into(),
            Tiling::RightHalf => {
                *frame = (s_content.width / 2.0, 0, s_content.width / 2.0, s_content.height).into();
            }
            Tiling::Distribute(ratio) => distribute_with_ratio(frame.size, self.view.subviews(), ratio),
            Tiling::Wrap => self.wrap_layout(frame),
        }
    }

    /// Rows of subviews in declaration order. A child keeps its own size,
    /// only its origin is set. Wraps before a child that would cross the
    /// container width, each row is as tall as its tallest child. Hidden
    /// children take no space. Sets the container height to the content.
    fn wrap_layout(&mut self, frame: RMut) {
        let margin = *self.all_margin.borrow();
        let width = frame.width();

        let mut x = 0.0;
        let mut y = 0.0;
        let mut row_height: f32 = 0.0;

        for view in self.view.subviews() {
            let mut view = view.weak();

            if view.is_hidden() {
                continue;
            }

            // The parent lays out before its children, so the child's size
            // rules have not run yet this frame. Running its layout here
            // lets the wrap see the current size instead of the last frame.
            // The traversal runs it again later, which is a no-op.
            view.layout();

            let size = view.frame().size;

            if x > 0.0 && x + size.width > width {
                x = 0.0;
                y += row_height + margin;
                row_height = 0.0;
            }

            view.set_position((x, y));

            x += size.width + margin;
            row_height = row_height.max(size.height);
        }

        frame.size.height = y + row_height;
    }

    fn between_super_layout(&mut self, frame: RMut, side: Anchor, view: WeakView) {
        let f = view.frame();
        let cen = f.center();

        match side {
            Anchor::Top => frame.set_center((cen.x, f.y() / 2.0)),
            Anchor::Bot => frame.set_center((
                cen.x,
                self.s_content.height - (self.s_content.height - f.max_y()) / 2.0,
            )),
            Anchor::Left => frame.set_center((f.x() / 2.0, cen.y)),
            Anchor::Right => frame.set_center((
                self.s_content.width - (self.s_content.width - f.max_x()) / 2.0,
                cen.y,
            )),
            _ => unimplemented!("Between super layout for {side:?} is not supported"),
        }
    }

    fn has_left(&self) -> bool {
        self.rules.borrow().iter().any(|rule| rule.side().is_some_and(Anchor::is_left))
    }

    fn has_top(&self) -> bool {
        self.rules.borrow().iter().any(|rule| rule.side().is_some_and(Anchor::is_top))
    }
}

fn place_vertically(views: &[Own<dyn View>], margin: f32) {
    distribute::<{ Axis::Y }>(views, margin);
}

fn place_horizontally(views: &[Own<dyn View>], margin: f32) {
    distribute::<{ Axis::X }>(views, margin);
}

fn distribute<const AXIS: Axis>(views: &[Own<dyn View>], margin: f32) {
    let Some(last) = views.last() else {
        return;
    };

    let super_frame = *last.superview().frame();

    if views.len() == 1 {
        let back = super_frame.with_zero_origin();
        last.set_frame(back);
        return;
    }

    let all_margins = margin * (views.len() - 1).lossy_convert();

    let left_length = super_frame.length::<AXIS>() - all_margins;

    let length = left_length / views.len().lossy_convert();
    let other_length = super_frame.other_length::<AXIS>();

    let mut last_pos: f32 = 0.0;

    for view in views {
        let mut frame = *view.frame();

        frame.set_position::<AXIS>(last_pos);
        frame.set_other_position::<AXIS>(0);
        frame.set_length::<AXIS>(length);
        frame.set_other_length::<AXIS>(other_length);

        view.set_frame(frame);

        last_pos += length + margin;
    }
}

fn distribute_with_ratio(size: Size, views: &[Own<dyn View>], ratios: &[f32]) {
    assert_eq!(
        views.len(),
        ratios.len(),
        "Distribute ratio count must match the number of subviews"
    );

    let total_ratio = 1.0 / ratios.iter().sum::<f32>();

    for i in 0..ratios.len() {
        let is_first = i == 0;
        let x_pos = if is_first { 0.0 } else { views[i - 1].max_x() };
        views[i].set_frame((x_pos, 0, ratios[i] * size.width * total_ratio, size.height));
    }
}

fn relative_layout(frame: RMut, side: Anchor, ratio: f32, view: WeakView) {
    let a_frame = view.frame();

    match side {
        Anchor::Width => frame.size.width = a_frame.size.width * ratio,
        Anchor::Height => frame.size.height = a_frame.size.height * ratio,
        Anchor::X => frame.origin.x = a_frame.width() * ratio,
        Anchor::Y => frame.origin.y = a_frame.height() * ratio,
        _ => unimplemented!("Relative layout for {side:?} is not supported"),
    }
}

fn anchor_layout(frame: RMut, side: Anchor, offset: f32, view: WeakView, has_left: bool, has_top: bool) {
    let a_frame = view.frame();

    match side {
        Anchor::Top => frame.origin.y = a_frame.max_y() + offset,
        Anchor::Bot => {
            if has_top {
                let max_y = frame.max_y();
                let desired_max_y = a_frame.y() - offset;
                let diff = desired_max_y - max_y;
                frame.size.height += diff;
            } else {
                frame.origin.y = a_frame.y() - offset - frame.height();
            }
        }
        Anchor::Left => frame.origin.x = a_frame.max_x() + offset,
        Anchor::Right => {
            if has_left {
                let max_x = frame.max_x();
                let desired_max_x = a_frame.x() - offset;
                let diff = desired_max_x - max_x;
                frame.size.width += diff;
            } else {
                frame.origin.x = a_frame.x() - offset - frame.width();
            }
        }
        Anchor::X => frame.origin.x = a_frame.x(),
        Anchor::Y => frame.origin.y = a_frame.y(),
        Anchor::Width => frame.size.width = a_frame.width(),
        Anchor::Height => frame.size.height = a_frame.height(),
        _ => unimplemented!("Anchor layout for: {side:?} is not supported"),
    }
}

fn between_2_layout(frame: RMut, a: WeakView, b: WeakView) {
    let center = a.frame().center().middle(b.frame().center());
    frame.set_center(center);
}

fn same_layout(frame: RMut, side: Anchor, view: WeakView) {
    let a_frame = view.frame();

    match side {
        Anchor::Width => frame.size.width = a_frame.size.width,
        Anchor::Height => frame.size.height = a_frame.size.height,
        Anchor::X => frame.origin.x = a_frame.x(),
        Anchor::Y => frame.origin.y = a_frame.y(),
        Anchor::CenterX => {
            let mut frame_center = frame.center();
            let a_center = a_frame.center();
            frame_center.x = a_center.x;
            frame.set_center(frame_center);
        }
        Anchor::CenterY => {
            let mut frame_center = frame.center();
            let a_center = a_frame.center();
            frame_center.y = a_center.y;
            frame.set_center(frame_center);
        }
        _ => unimplemented!("Same layout for {side:?} is not supported"),
    }
}
