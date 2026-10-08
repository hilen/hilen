use std::ops::DerefMut;

use crate::{
    deps::refs::weak_from_ref,
    ui::{
        CursorIcon, LongPress, TextSelection, Touch, TouchStack, UIManager, View, ViewTouchEvents, WeakView,
        view::{ViewFrame, double_tap::wait_for_single_tap, view_data::ViewData},
    },
    window::MouseButton,
};

pub(crate) const NO_TOUCH_ID: usize = 0;

pub trait ViewTouch {
    fn is_selected(&self) -> bool;
    fn is_hovered(&self) -> bool;
    /// This view or a view inside it is hovered, see `hover_within`.
    fn is_hover_within(&self) -> bool;
    fn enable_touch(&self) -> &Self;
    fn enable_touch_low_priority(&self) -> &Self;
    fn enable_hover(&self) -> &Self;
    /// Makes this view take a pinch, see `touch().pinch`.
    fn enable_pinch(&self) -> &Self;

    /// The mouse cursor to show while this view is hovered, for example
    /// `CursorIcon::ColResize` on a panel drag handle. Turns hover on for
    /// the view. Only desktop and the browser have a cursor, everywhere
    /// else this is inert.
    fn set_hover_cursor(&self, icon: CursorIcon) -> &Self;

    /// On, a touch that begins on this view leaves the selected view as it
    /// is, like a button of a web page that prevents the default of its
    /// mouse down. A text field in edit keeps its caret, its keys and the
    /// screen keyboard of a phone, and this view still gets every touch
    /// event. For a send button next to a compose box. It counts only for
    /// the view that takes the touch, not for the views inside it.
    fn set_keeps_selection(&self, on: bool) -> &Self;

    fn disable_touch(&self);
    fn touch(&self) -> &ViewTouchEvents;
}

impl<T: ?Sized + View> ViewTouch for T {
    fn is_selected(&self) -> bool {
        self.__base_view().is_selected
    }

    fn is_hovered(&self) -> bool {
        self.__base_view().is_hovered
    }

    fn is_hover_within(&self) -> bool {
        self.__base_view().is_hover_within
    }

    fn enable_touch(&self) -> &Self {
        TouchStack::enable_for(self.weak_view());
        self
    }

    fn enable_touch_low_priority(&self) -> &Self {
        TouchStack::enable_for_low_priority(self.weak_view());
        self
    }

    fn enable_hover(&self) -> &Self {
        TouchStack::enable_hover(self.weak_view());
        self
    }

    fn enable_pinch(&self) -> &Self {
        TouchStack::enable_pinch(self.weak_view());
        self
    }

    fn set_hover_cursor(&self, icon: CursorIcon) -> &Self {
        self.__base_view().hover_cursor = Some(icon);
        self.enable_hover()
    }

    fn set_keeps_selection(&self, on: bool) -> &Self {
        self.__base_view().keeps_selection = on;
        self
    }

    fn disable_touch(&self) {
        TouchStack::disable_for(self.weak_view());
    }

    fn touch(&self) -> &ViewTouchEvents {
        &self.__base_view().events.touch
    }
}

/// The view that takes this press carries `set_keeps_selection`. Asked
/// before the press goes to the views: they are asked one by one, and each
/// one that is not under the press drops the selection before the one that
/// takes it is reached.
pub(crate) fn press_keeps_selection(touch: &Touch) -> bool {
    if !touch.is_began() || touch.button == MouseButton::Right {
        return false;
    }
    let point = touch.position;
    TouchStack::touch_views()
        .find(|view| {
            view.is_ok()
                && !view.is_hidden_in_tree()
                && view.contains_visible(point)
                && !TouchStack::covered(*view, point)
        })
        .is_some_and(|view| view.__base_view().keeps_selection)
}

/// `keeps_selection` is the answer of `press_keeps_selection` for this
/// touch.
pub(crate) fn check_touch(mut view: WeakView, touch: &mut Touch, keeps_selection: bool) -> bool {
    if view.is_null() {
        return false;
    }

    let weak = view;
    let view = view.deref_mut();
    let base_view = view.__base_view();

    if view.is_hidden_in_tree() {
        // A view hidden during an active touch must not keep the capture.
        // A stale capture eats hover moves and steals other views' ends.
        base_view.__touch_id = NO_TOUCH_ID;
        return false;
    }

    // A right press is its own event and never captures the view. Handled
    // first so its release cannot end a left capture that shares the id,
    // every mouse event is finger 1.
    if touch.button == MouseButton::Right {
        if !touch.is_began() || !view.contains_visible(touch.position) {
            return false;
        }

        // Before the event of the view, so a menu the view opens itself
        // is the one that stays.
        TextSelection::right_clicked(weak, touch.position);
        touch.position = base_view.local_point(touch.position);
        base_view.events.touch.secondary.trigger(*touch);
        return true;
    }

    if touch.is_moved() && base_view.__touch_id == touch.id {
        touch.position = base_view.local_point(touch.position);
        base_view.events.touch.all.trigger(*touch);
        base_view.events.touch.moved.trigger(*touch);
        return true;
    }

    if touch.is_moved() {
        return false;
    }

    if touch.is_ended() && base_view.__touch_id == touch.id {
        let inside = view.contains_visible(touch.position);
        let on_screen = touch.position;

        touch.position = base_view.local_point(touch.position);
        base_view.__touch_id = NO_TOUCH_ID;
        base_view.events.touch.all.trigger(*touch);

        if inside && touch.is_ended() {
            let first = base_view.taps.tap(on_screen);
            base_view.events.touch.up_inside.trigger(*touch);
            // The tap may have closed the screen this view was on.
            if weak.is_null() {
                return true;
            }
            match first {
                None => base_view.events.touch.double_tap.trigger(*touch),
                Some(first) if base_view.events.touch.single_tap.has_subscribers() => {
                    wait_for_single_tap(weak, first, *touch);
                }
                Some(_) => {}
            }
        }
        return true;
    }

    // Only a began touch may be claimed by position. An ended touch must
    // fall through to the view that captured it on began: a release over
    // some other view used to be eaten here, the captor kept its
    // __touch_id, and every later bare mouse move kept dragging it.
    if touch.is_began() && view.contains_visible(touch.position) {
        let on_screen = touch.position;
        touch.position = base_view.local_point(on_screen);
        base_view.__touch_id = touch.id;
        LongPress::arm(weak_from_ref(view), touch.id, on_screen);
        base_view.events.touch.began.trigger(*touch);
        TextSelection::pressed(weak, on_screen, touch.id);
        if !keeps_selection {
            UIManager::set_selected(weak_from_ref(view), true);
        }
        base_view.events.touch.all.trigger(*touch);
        return true;
    }

    if touch.is_began() && !keeps_selection {
        UIManager::unselect_view();
    }

    false
}
