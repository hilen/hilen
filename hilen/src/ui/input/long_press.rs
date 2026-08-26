use std::sync::atomic::{AtomicU64, Ordering};

use crate::{
    deps::{hreads::after, refs::main_lock::MainLock},
    gm::flat::Point,
    ui::{NO_TOUCH_ID, Tooltip, Touch, TouchStack, ViewData, ViewFrame, WeakView, input::TouchEvent},
    window::MouseButton,
};

struct Pending {
    view:       WeakView,
    touch_id:   usize,
    origin:     Point,
    generation: u64,
    /// The view never captured the touch, it only has a tooltip to show.
    hold_only:  bool,
}

static PENDING: MainLock<Option<Pending>> = MainLock::new();
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// Turns a held touch into a `secondary` event on the view that captured
/// it. A touch screen has no right button, so this is how a phone opens
/// a context menu. The timer runs on every platform, a mouse held still
/// long enough counts too.
pub(crate) struct LongPress;

impl LongPress {
    pub(crate) const DURATION: f32 = 0.5;

    /// A finger never holds perfectly still. Anything under this many
    /// points of travel is still a hold, more is a drag.
    const SLOP: f32 = 10.0;

    /// `position` is the absolute touch position, before the view origin
    /// is subtracted.
    pub(crate) fn arm(view: WeakView, touch_id: usize, position: Point) {
        Self::arm_internal(view, touch_id, position, false);
    }

    /// A press that no touch view claimed. The topmost hover view with a
    /// tooltip under it shows that tooltip after the hold.
    pub(crate) fn arm_tooltip_hold(touch_id: usize, position: Point) {
        if PENDING.get_mut().is_some() {
            return;
        }

        let Some(view) = TouchStack::hover_views().find(|view| {
            view.is_ok() && !view.is_hidden_in_tree() && view.absolute_frame().contains(position)
        }) else {
            return;
        };

        if view.__base_view().tooltip.is_none() {
            return;
        }

        Self::arm_internal(view, touch_id, position, true);
    }

    fn arm_internal(view: WeakView, touch_id: usize, position: Point, hold_only: bool) {
        let generation = GENERATION.fetch_add(1, Ordering::Relaxed) + 1;

        *PENDING.get_mut() = Some(Pending {
            view,
            touch_id,
            origin: position,
            generation,
            hold_only,
        });

        after(Self::DURATION, move || Self::fire(generation));
    }

    pub(crate) fn moved(touch_id: usize, position: Point) {
        let Some(pending) = PENDING.get_mut() else {
            return;
        };

        if pending.touch_id == touch_id && (position - pending.origin).length() > Self::SLOP {
            *PENDING.get_mut() = None;
        }
    }

    pub(crate) fn cancel(touch_id: usize) {
        if PENDING.get_mut().as_ref().is_some_and(|pending| pending.touch_id == touch_id) {
            *PENDING.get_mut() = None;
        }
    }

    fn fire(generation: u64) {
        if PENDING
            .get_mut()
            .as_ref()
            .is_none_or(|pending| pending.generation != generation)
        {
            return;
        }

        let pending = PENDING.get_mut().take().unwrap();
        let view = pending.view;

        if view.is_null() || view.is_hidden_in_tree() {
            return;
        }

        if pending.hold_only {
            Tooltip::show(view, pending.origin);
            return;
        }

        let base = view.__base_view();

        // The capture moved on, the view was released or another touch
        // took it, so there is nothing being held any more.
        if base.__touch_id != pending.touch_id {
            return;
        }

        // The hold is consumed. Its release must not end as a tap.
        base.__touch_id = NO_TOUCH_ID;

        let touch = Touch {
            id:       pending.touch_id,
            position: pending.origin - view.absolute_frame().origin,
            event:    TouchEvent::Began,
            button:   MouseButton::Left,
        };

        // A view that hangs nothing on the hold shows its tooltip instead,
        // the touch screen stand in for hovering.
        if base.events.touch.secondary.has_subscribers() {
            base.events.touch.secondary.trigger(touch);
        } else if base.tooltip.is_some() {
            Tooltip::show(view, pending.origin);
        }
    }
}
