#[cfg(any(desktop, wasm))]
use crate::gm::flat::Point;
#[cfg(any(desktop, wasm))]
use crate::ui::{
    TouchStack,
    view::{ViewData, ViewFrame},
};
use crate::{deps::refs::main_lock::MainLock, ui::WeakView};

static HOVERED: MainLock<WeakView> = MainLock::new();

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
    }
}
