use ui_proc::view;

use super::split_handle::{SplitHandle, grab_width};
use crate::{
    self as hilen,
    deps::{refs::Weak, vents::Event},
    ui::{Container, Setup, UIColor, ViewData, ViewFrame, view::depth_order},
};

/// Two panels, one above the other: a bottom panel with a draggable
/// divider over it, and the top panel taking the rest. Put content into
/// `top` and `bottom`, they fill their panels with `place().back()`.
///
/// The twin of `SplitView` for a panel under the content, like a terminal
/// under an editor. Dragging respects the minimum heights, keeps
/// following the cursor when it outruns the divider, and releases cleanly
/// wherever the cursor ends up. `resized` fires once per finished drag,
/// for persisting the height.
#[view]
pub struct VSplitView {
    bottom_height: f32,

    min_top_height:    f32,
    min_bottom_height: f32,

    bottom_hidden: bool,

    drag_start_height: f32,

    /// A divider drag ended. Read the height and persist it.
    pub resized: Event,

    #[init]
    pub top:    Container,
    pub bottom: Container,

    handle: SplitHandle,
}

impl Setup for VSplitView {
    fn setup(mut self: Weak<Self>) {
        self.bottom_height = 240.0;
        self.min_top_height = 100.0;
        self.min_bottom_height = 100.0;

        // In front of the panels, whose content overlaps its grab zone.
        depth_order::set_raise(&*self.handle, 1);
        self.handle.lay_flat();

        // From the height on screen, not from the wanted one. A view too
        // short for the wanted height shows less, and the divider has to
        // move with the first pixel of the drag.
        self.handle
            .drag_began
            .sub(move || self.drag_start_height = self.bottom.height());

        // Up is a smaller y, and up makes the bottom panel taller.
        self.handle.dragged.val(move |dy| {
            let height = self.height();
            self.bottom_height = (self.drag_start_height - dy)
                .max(self.min_bottom_height)
                .min(self.max_bottom_height(height));
            self.layout_panels();
        });

        self.handle.drag_ended.sub(move || self.resized.trigger(()));

        self.size_changed().sub(move || self.layout_panels());
        self.layout_panels();
    }
}

impl VSplitView {
    /// The height the bottom panel wants. A view too short for it shows
    /// less and keeps this for when it grows.
    pub fn bottom_height(&self) -> f32 {
        self.bottom_height
    }

    pub fn set_bottom_height(mut self: Weak<Self>, height: f32) -> Weak<Self> {
        self.bottom_height = height.max(self.min_bottom_height);
        self.layout_panels();
        self
    }

    pub fn set_min_heights(mut self: Weak<Self>, top: f32, bottom: f32) -> Weak<Self> {
        self.min_top_height = top;
        self.min_bottom_height = bottom;
        self.layout_panels();
        self
    }

    pub fn bottom_hidden(&self) -> bool {
        self.bottom_hidden
    }

    /// A hidden bottom panel gives its room to the top one and keeps its
    /// height for when it comes back.
    pub fn set_bottom_hidden(mut self: Weak<Self>, hidden: bool) -> Weak<Self> {
        self.bottom_hidden = hidden;
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
        self.handle.set_colors(idle.into(), active.into());
        self
    }

    /// The bottom panel may grow only until the top one would shrink
    /// below its minimum.
    fn max_bottom_height(&self, height: f32) -> f32 {
        self.min_bottom_height.max(height - self.min_top_height)
    }

    fn layout_panels(self: Weak<Self>) {
        let width = self.width();
        let height = self.height();
        let grab = grab_width();

        let bottom_height = if self.bottom_hidden {
            0.0
        } else {
            self.bottom_height.min(self.max_bottom_height(height))
        };
        let top_height = (height - bottom_height).max(0.0);

        self.top.set_frame((0, 0, width, top_height));

        self.bottom.set_hidden(self.bottom_hidden);
        self.bottom.set_frame((0, top_height, width, bottom_height));
        self.handle.set_hidden(self.bottom_hidden);
        self.handle.set_frame((0, top_height - grab / 2.0, width, grab));
    }
}
