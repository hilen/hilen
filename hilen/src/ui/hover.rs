#[cfg(any(desktop, wasm))]
use crate::gm::flat::Point;
#[cfg(any(desktop, wasm))]
use crate::ui::{
    TouchStack,
    view::{ViewData, ViewFrame},
};
use crate::{
    deps::refs::main_lock::MainLock,
    ui::{CursorIcon, Tooltip, WeakView},
};

static HOVERED: MainLock<WeakView> = MainLock::new();
static CURSOR: MainLock<CursorIcon> = MainLock::new();

/// Tracks the single topmost hovered view. Runs only on mouse move,
/// scroll and cursor leave. Nothing here runs per frame.
///
/// Everything that picks a view under the cursor is compiled only where a
/// cursor exists, which is desktop and the browser. A touch screen has no
/// pointer, so on iOS and Android there is nothing to track.
pub struct Hover;

impl Hover {
    #[cfg(any(desktop, wasm))]
    pub(crate) fn update(cursor: Point) {
        Self::set_hovered(Self::view_under(cursor));
    }

    /// The cursor left the window. The hovered view gets an exit.
    pub fn clear() {
        Self::set_hovered(WeakView::default());
    }

    #[cfg(any(desktop, wasm))]
    fn view_under(cursor: Point) -> WeakView {
        TouchStack::hover_views()
            .find(|view| view.is_ok() && !view.is_hidden_in_tree() && view.absolute_frame().contains(cursor))
            .unwrap_or_default()
    }

    fn set_hovered(new: WeakView) {
        let old = *HOVERED;
        let old_alive = old.is_ok();

        // Raw equality alone is not enough. A freed view and its
        // replacement can share an address, and the new view still
        // needs its enter event.
        if old_alive && old.raw() == new.raw() {
            return;
        }

        if !old_alive && !new.is_ok() {
            return;
        }

        *HOVERED.get_mut() = new;

        if old_alive {
            let base = old.__base_view();
            base.is_hovered = false;
            base.events.touch.hovered.trigger(false);
        }

        if new.is_ok() {
            let base = new.__base_view();
            base.is_hovered = true;
            base.events.touch.hovered.trigger(true);
        }

        let cursor = if new.is_ok() {
            new.__base_view().hover_cursor.unwrap_or_default()
        } else {
            CursorIcon::default()
        };
        Self::apply_cursor(cursor);

        Tooltip::hover_changed(new);
    }

    /// The cursor icon hover currently asks for, `Default` off any view
    /// with a custom one. Windowless runs track it too, so tests can
    /// assert it.
    pub fn cursor() -> CursorIcon {
        *CURSOR
    }

    fn apply_cursor(cursor: CursorIcon) {
        if *CURSOR == cursor {
            return;
        }
        *CURSOR.get_mut() = cursor;

        #[cfg(any(desktop, wasm))]
        if let Some(window) = crate::window::Window::winit_window() {
            window.set_cursor(cursor);
        }
    }
}
