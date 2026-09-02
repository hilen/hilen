use plat::Platform;
use ui_proc::view;

#[cfg(any(desktop, wasm))]
use crate::ui::Hover;
use crate::{
    self as hilen,
    deps::{refs::Weak, vents::Event},
    gm::color::{CLEAR, LIGHT_BLUE},
    ui::{
        Container, CursorIcon, Setup, Touch, TouchStack, UIColor, UIManager, View, ViewData, ViewFrame,
        ViewTouch,
    },
};

/// The visible divider line.
const LINE_WIDTH: f32 = 2.0;

/// The grab zone around the line. A finger needs more than a cursor.
fn grab_width() -> f32 {
    if Platform::MOBILE { 24.0 } else { 10.0 }
}

/// The invisible strip a divider is dragged by. Wider than the line it
/// draws, always on top of the panel content it overlaps, and it keeps
/// the resize cursor and its highlight for the whole drag.
#[view]
pub(crate) struct SplitHandle {
    dragging: bool,

    idle_color:   UIColor,
    active_color: UIColor,

    #[init]
    line: Container,
}

impl Setup for SplitHandle {
    fn setup(mut self: Weak<Self>) {
        self.idle_color = CLEAR.into();
        self.active_color = LIGHT_BLUE.into();

        // The grab zone overlaps both panels, whose content registers
        // touch and hover later. The high priority tiers keep the divider
        // winning inside its strip regardless of registration order.
        TouchStack::enable_for_high_priority(self.weak_view());
        TouchStack::enable_hover_high_priority(self.weak_view());
        self.__base_view().hover_cursor = Some(CursorIcon::ColResize);

        self.line.place().center_x().w(LINE_WIDTH).tb(0);

        self.touch().hovered.val(self, move |_| self.apply_color());
        self.apply_color();
    }
}

impl SplitHandle {
    fn set_dragging(mut self: Weak<Self>, dragging: bool) {
        self.dragging = dragging;
        self.apply_color();
    }

    fn apply_color(self: Weak<Self>) {
        let color = if self.dragging || self.is_hovered() {
            self.active_color
        } else {
            self.idle_color
        };
        self.line.set_color(color);
    }
}

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

    drag_start_x:     f32,
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

        // A later sibling draws behind an earlier sibling's children, so
        // without the bump the divider lines would hide under panel
        // content. Same move as the scroll bar.
        self.left_handle.bump_z_position(UIManager::subview_z_offset() * 10.0);
        self.right_handle.bump_z_position(UIManager::subview_z_offset() * 10.0);

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
        for mut handle in [self.left_handle, self.right_handle] {
            handle.idle_color = idle;
            handle.active_color = active;
            handle.apply_color();
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
        handle.touch().began.sub(move || {
            self.drag_start_x = UIManager::cursor_position().x;
            self.drag_start_width = if left { self.left_width } else { self.right_width };
            handle.set_dragging(true);
            #[cfg(any(desktop, wasm))]
            Hover::lock(handle.weak_view());
        });

        // Deltas from the drag start, not from the handle, so a clamped
        // drag never jumps when the cursor comes back.
        handle.touch().moved.sub(move || {
            let dx = UIManager::cursor_position().x - self.drag_start_x;
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

        // The release can land far off the handle after a clamped drag,
        // so it rides the `all` event, not `up_inside`.
        handle.touch().all.val(move |touch: Touch| {
            if !touch.is_ended() {
                return;
            }
            handle.set_dragging(false);
            #[cfg(any(desktop, wasm))]
            Hover::unlock();
            self.resized.trigger(());
        });
    }
}
