use ui_proc::view;

use super::split_handle::{SplitHandle, grab_width};
use crate::{
    self as hilen,
    deps::{refs::Weak, vents::Event},
    ui::{Container, Setup, UIColor, ViewData, ViewFrame, view::depth_order},
};

/// Three panels side by side: a left and a right panel with draggable
/// dividers and the center taking the rest. Put content into `left`,
/// `center` and `right`, they fill their panels with `place().back()`.
///
/// Dragging respects the minimum widths, keeps following the cursor when
/// it outruns the divider, and releases cleanly wherever the cursor ends
/// up. `resized` fires once per finished drag, for persisting widths.
#[view]
pub struct SplitView {
    left_width:  f32,
    right_width: f32,

    min_left_width:   f32,
    min_center_width: f32,
    min_right_width:  f32,

    left_hidden:  bool,
    right_hidden: bool,

    drag_start_width: f32,

    /// A divider drag ended. Read the widths and persist them.
    pub resized: Event,

    #[init]
    pub left:   Container,
    pub center: Container,
    pub right:  Container,

    left_handle:  SplitHandle,
    right_handle: SplitHandle,
}

impl Setup for SplitView {
    fn setup(mut self: Weak<Self>) {
        self.left_width = 240.0;
        self.right_width = 240.0;
        self.min_left_width = 100.0;
        self.min_center_width = 100.0;
        self.min_right_width = 100.0;

        // In front of the panels, which are added after the handles.
        depth_order::set_raise(&*self.left_handle, 1);
        depth_order::set_raise(&*self.right_handle, 1);

        self.setup_handle(self.left_handle, true);
        self.setup_handle(self.right_handle, false);

        self.size_changed().sub(move || self.layout_panels());
        self.layout_panels();
    }
}

impl SplitView {
    pub fn left_width(&self) -> f32 {
        self.left_width
    }

    pub fn right_width(&self) -> f32 {
        self.right_width
    }

    pub fn set_left_width(mut self: Weak<Self>, width: f32) -> Weak<Self> {
        self.left_width = width.max(self.min_left_width);
        self.layout_panels();
        self
    }

    pub fn set_right_width(mut self: Weak<Self>, width: f32) -> Weak<Self> {
        self.right_width = width.max(self.min_right_width);
        self.layout_panels();
        self
    }

    pub fn set_min_widths(mut self: Weak<Self>, left: f32, center: f32, right: f32) -> Weak<Self> {
        self.min_left_width = left;
        self.min_center_width = center;
        self.min_right_width = right;
        self.layout_panels();
        self
    }

    pub fn left_hidden(&self) -> bool {
        self.left_hidden
    }

    pub fn right_hidden(&self) -> bool {
        self.right_hidden
    }

    pub fn set_left_hidden(mut self: Weak<Self>, hidden: bool) -> Weak<Self> {
        self.left_hidden = hidden;
        self.layout_panels();
        self
    }

    pub fn set_right_hidden(mut self: Weak<Self>, hidden: bool) -> Weak<Self> {
        self.right_hidden = hidden;
        self.layout_panels();
        self
    }

    /// The divider line colors: `idle` normally, `active` while hovered
    /// or dragged.
    pub fn set_divider_colors(
        self: Weak<Self>,
        idle: impl Into<UIColor>,
        active: impl Into<UIColor>,
    ) -> Weak<Self> {
        let idle = idle.into();
        let active = active.into();
        for handle in [self.left_handle, self.right_handle] {
            handle.set_colors(idle, active);
        }
        self
    }

    /// A drag may grow a panel only until the center would shrink below
    /// its minimum, so the panels never overlap.
    fn max_left_width(&self, width: f32) -> f32 {
        let other = if self.right_hidden { 0.0 } else { self.right_width };
        self.min_left_width.max(width - other - self.min_center_width)
    }

    fn max_right_width(&self, width: f32) -> f32 {
        let other = if self.left_hidden { 0.0 } else { self.left_width };
        self.min_right_width.max(width - other - self.min_center_width)
    }

    fn layout_panels(self: Weak<Self>) {
        let width = self.width();
        let height = self.height();
        let grab = grab_width();

        let left_width = if self.left_hidden {
            0.0
        } else {
            self.left_width.min(self.max_left_width(width))
        };
        let right_width = if self.right_hidden {
            0.0
        } else {
            self.right_width.min(self.max_right_width(width))
        };

        self.left.set_hidden(self.left_hidden);
        self.left.set_frame((0, 0, left_width, height));
        self.left_handle.set_hidden(self.left_hidden);
        self.left_handle.set_frame((left_width - grab / 2.0, 0, grab, height));

        self.right.set_hidden(self.right_hidden);
        self.right.set_frame((width - right_width, 0, right_width, height));
        self.right_handle.set_hidden(self.right_hidden);
        self.right_handle.set_frame((width - right_width - grab / 2.0, 0, grab, height));

        let center_width = (width - left_width - right_width).max(0.0);
        self.center.set_frame((left_width, 0, center_width, height));
    }

    fn setup_handle(mut self: Weak<Self>, handle: Weak<SplitHandle>, left: bool) {
        handle.drag_began.sub(move || {
            self.drag_start_width = if left { self.left_width } else { self.right_width };
        });

        handle.dragged.val(move |dx| {
            let width = self.width();
            if left {
                self.left_width = (self.drag_start_width + dx)
                    .max(self.min_left_width)
                    .min(self.max_left_width(width));
            } else {
                self.right_width = (self.drag_start_width - dx)
                    .max(self.min_right_width)
                    .min(self.max_right_width(width));
            }
            self.layout_panels();
        });

        handle.drag_ended.sub(move || self.resized.trigger(()));
    }
}
