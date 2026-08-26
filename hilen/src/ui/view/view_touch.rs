use std::ops::DerefMut;

use crate::{
    deps::refs::weak_from_ref,
    ui::{
        LongPress, Touch, TouchStack, UIManager, View, ViewTouchEvents, WeakView,
        view::{ViewFrame, view_data::ViewData},
    },
    window::MouseButton,
};

pub(crate) const NO_TOUCH_ID: usize = 0;

pub trait ViewTouch {
    fn is_selected(&self) -> bool;
    fn is_hovered(&self) -> bool;
    fn enable_touch(&self) -> &Self;
    fn enable_touch_low_priority(&self) -> &Self;
    fn enable_hover(&self) -> &Self;
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

    fn disable_touch(&self) {
        TouchStack::disable_for(self.weak_view());
    }

    fn touch(&self) -> &ViewTouchEvents {
        &self.__base_view().events.touch
    }
}

pub(crate) fn check_touch(mut view: WeakView, touch: &mut Touch) -> bool {
    if view.is_null() {
        return false;
    }

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
        if !touch.is_began() || !view.absolute_frame().contains(touch.position) {
            return false;
        }

        touch.position -= view.absolute_frame().origin;
        base_view.events.touch.secondary.trigger(*touch);
        return true;
    }

    if touch.is_moved() && base_view.__touch_id == touch.id {
        LongPress::moved(touch.id, touch.position);
        touch.position -= view.absolute_frame().origin;
        base_view.events.touch.all.trigger(*touch);
        base_view.events.touch.moved.trigger(*touch);
        return true;
    }

    if touch.is_moved() {
        return false;
    }

    if touch.is_ended() && base_view.__touch_id == touch.id {
        LongPress::cancel(touch.id);
        let inside = view.absolute_frame().contains(touch.position);

        touch.position -= view.absolute_frame().origin;
        base_view.__touch_id = NO_TOUCH_ID;
        base_view.events.touch.all.trigger(*touch);

        if inside && touch.is_ended() {
            base_view.events.touch.up_inside.trigger(*touch);
        }
        return true;
    }

    if view.absolute_frame().contains(touch.position) {
        touch.position -= view.absolute_frame().origin;
        if touch.is_began() {
            base_view.__touch_id = touch.id;
            LongPress::arm(
                weak_from_ref(view),
                touch.id,
                touch.position + view.absolute_frame().origin,
            );
            base_view.events.touch.began.trigger(*touch);
            UIManager::set_selected(weak_from_ref(view), true);
        }
        base_view.events.touch.all.trigger(*touch);
        return true;
    }

    if touch.is_began() {
        UIManager::unselect_view();
    }

    false
}
