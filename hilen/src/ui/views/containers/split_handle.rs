use plat::Platform;
use ui_proc::view;

#[cfg(any(desktop, wasm))]
use crate::ui::Hover;
use crate::{
    self as hilen,
    deps::{refs::Weak, vents::Event},
    gm::color::{CLEAR, LIGHT_BLUE},
    ui::{Container, CursorIcon, Setup, Touch, TouchStack, UIColor, UIManager, View, ViewData, ViewTouch},
};

/// The visible divider line.
const LINE_WIDTH: f32 = 2.0;

/// The grab zone around the line. A finger needs more than a cursor.
pub(crate) fn grab_width() -> f32 {
    if Platform::MOBILE { 24.0 } else { 10.0 }
}

/// The invisible strip a divider is dragged by. Wider than the line it
/// draws, always on top of the panel content it overlaps, and it keeps
/// the resize cursor and its highlight for the whole drag.
///
/// It stands upright and is dragged left and right, until `lay_flat`
/// makes it a divider that is dragged up and down.
#[view]
pub(crate) struct SplitHandle {
    dragging:   bool,
    flat:       bool,
    /// Where the cursor was on the drag axis when the drag began.
    drag_start: f32,

    idle_color:   UIColor,
    active_color: UIColor,

    /// A touch took the handle.
    pub drag_began: Event,
    /// The cursor moved in a drag. Carries its way from the start of the
    /// drag, not from the handle, so a clamped drag never jumps when the
    /// cursor comes back.
    pub dragged:    Event<f32>,
    /// The touch let go, wherever the cursor is by then.
    pub drag_ended: Event,

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
        self.enable_touch_high_priority();
        TouchStack::enable_hover_high_priority(self.weak_view());
        self.__base_view().hover_cursor = Some(CursorIcon::ColResize);

        self.line.place().center_x().w(LINE_WIDTH).tb(0);

        self.touch().hovered.val(self, move |_| self.apply_color());
        self.apply_color();

        self.touch().began.sub(move || {
            self.drag_start = self.cursor();
            self.set_dragging(true);
            #[cfg(any(desktop, wasm))]
            Hover::lock(self.weak_view());
            self.drag_began.trigger(());
        });

        self.touch()
            .moved
            .sub(move || self.dragged.trigger(self.cursor() - self.drag_start));

        // The release can land far off the handle after a clamped drag,
        // so it rides the `all` event, not `up_inside`.
        self.touch().all.val(move |touch: Touch| {
            if !touch.is_ended() {
                return;
            }
            self.set_dragging(false);
            #[cfg(any(desktop, wasm))]
            Hover::unlock();
            self.drag_ended.trigger(());
        });
    }
}

impl SplitHandle {
    /// The line lies flat and the drag goes up and down.
    pub(crate) fn lay_flat(mut self: Weak<Self>) {
        self.flat = true;
        self.__base_view().hover_cursor = Some(CursorIcon::RowResize);
        self.line.place().clear().center_y().h(LINE_WIDTH).lr(0);
    }

    /// The line colors: `idle` normally, `active` while hovered or
    /// dragged.
    pub(crate) fn set_colors(mut self: Weak<Self>, idle: UIColor, active: UIColor) {
        self.idle_color = idle;
        self.active_color = active;
        self.apply_color();
    }

    /// The cursor on the axis the handle is dragged along.
    fn cursor(&self) -> f32 {
        let cursor = UIManager::cursor_position();
        if self.flat { cursor.y } else { cursor.x }
    }

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
