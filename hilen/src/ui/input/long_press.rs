use std::sync::atomic::{AtomicU64, Ordering};

use crate::{
    deps::{hreads::after, refs::main_lock::MainLock},
    gm::flat::Point,
    ui::{NO_TOUCH_ID, Touch, ViewData, ViewFrame, WeakView, input::TouchEvent},
    window::MouseButton,
};

struct Pending {
    view:       WeakView,
    touch_id:   usize,
    origin:     Point,
    generation: u64,
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
        let generation = GENERATION.fetch_add(1, Ordering::Relaxed) + 1;

        *PENDING.get_mut() = Some(Pending {
            view,
            touch_id,
            origin: position,
            generation,
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

        base.events.touch.secondary.trigger(touch);
    }
}
