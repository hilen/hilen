use std::mem::take;

use plat::Platform;

use crate::{
    deps::refs::{Weak, main_lock::MainLock},
    gm::flat::Point,
    ui::{Touch, TouchStack, UIManager, View, ViewData, ViewFrame, WeakView, input::TouchEvent},
    window::MouseButton,
};

/// How far a finger moves before the swipe or the views take it.
const SLOP: f32 = 10.0;
/// A last step of at least this many points finishes the swipe also before
/// half the width, a fling.
const FLING: f32 = 8.0;

/// Where a released swipe goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideSwipeEnd {
    /// The finger went left, past half the width of the view or with a fling.
    Left,
    /// The same to the right.
    Right,
    /// Neither, the view goes back to its place.
    Back,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SideSwipeEvent {
    /// The finger is this many points right of where it came down, less than
    /// 0 to the left.
    Moved(f32),
    /// The finger is up, the last `Moved` was its last place.
    Ended(SideSwipeEnd),
}

type Callback = Box<dyn FnMut(SideSwipeEvent) + Send>;

struct Entry {
    view: WeakView,
    on:   Option<Callback>,
}

#[derive(Default)]
enum Phase {
    #[default]
    Idle,
    /// A finger came down on the view. It is a swipe once it moves sideways.
    Watching {
        id:    usize,
        start: Point,
        view:  WeakView,
    },
    /// The view gets the offset of the finger.
    Dragging {
        id:     usize,
        start:  f32,
        offset: f32,
        step:   f32,
        view:   WeakView,
    },
}

static ENTRIES: MainLock<Vec<Entry>> = MainLock::new();
static PHASE: MainLock<Phase> = MainLock::new();

/// A swipe left or right over any view, also over a scroll view that moves
/// up and down, like the pages of a native iOS app. A finger that moves 10
/// points sideways, more sideways than down, is a swipe: the views and the
/// scroll that caught it let it go, so the release is no tap and no scroll.
/// A move of 10 points down first stays a scroll.
///
/// The engine moves nothing. The view gets `SideSwipeEvent::Moved` with the
/// offset of the finger while it moves and moves its own content, then
/// `Ended` with where it went. A release past half the width of the view
/// goes there, and so does a last step of 8 points or more, a fling.
///
/// A finger that comes down on a view that moves sideways by itself, a
/// scroll view with room to the side or a `CanvasView`, starts no swipe.
/// It works where a finger drags the content, see
/// `UIManager::drag_scrolling`, so on a phone and a tablet, never on a TV
/// and not over an open modal. The swipe back from the left edge,
/// `SwipeBack`, is asked first.
pub struct SideSwipe;

impl SideSwipe {
    pub fn set<T: View + ?Sized>(view: Weak<T>, on: impl FnMut(SideSwipeEvent) + Send + 'static) {
        let view = view.weak_view();
        let entries = ENTRIES.get_mut();
        entries.retain(|entry| entry.view.is_ok() && entry.view.raw() != view.raw());
        entries.push(Entry {
            view,
            on: Some(Box::new(on)),
        });
    }

    /// A new test, or a touch that the swipe back took.
    pub(crate) fn reset() {
        *PHASE.get_mut() = Phase::Idle;
    }

    /// True when the touch belongs to a swipe and no view may see it.
    /// `touch` is in screen points.
    pub(crate) fn swallow(touch: &Touch) -> bool {
        if touch.is_began() && !matches!(*PHASE.get_mut(), Phase::Dragging { .. }) {
            *PHASE.get_mut() = match Self::view_at(touch) {
                Some(view) => Phase::Watching {
                    id: touch.id,
                    start: touch.position,
                    view,
                },
                None => Phase::Idle,
            };
            return false;
        }
        match *PHASE.get_mut() {
            Phase::Idle => false,
            Phase::Watching { id, start, view } => {
                if touch.id != id {
                    return false;
                }
                Self::watch(touch, start, view);
                matches!(*PHASE.get_mut(), Phase::Dragging { .. })
            }
            Phase::Dragging { id, .. } => {
                // A second finger reaches no view while the first swipes.
                if touch.id == id {
                    Self::drag(touch);
                }
                true
            }
        }
    }

    /// The last view under the finger that asked for a swipe.
    fn view_at(touch: &Touch) -> Option<WeakView> {
        if touch.button != MouseButton::Left
            || Platform::TVOS
            || !UIManager::drag_scrolling()
            || UIManager::touch_disabled()
            || TouchStack::overlay_count() > 0
        {
            return None;
        }
        let entries = ENTRIES.get_mut();
        entries.retain(|entry| entry.view.is_ok());
        let view = entries
            .iter()
            .rev()
            .find(|entry| !entry.view.is_hidden_in_tree() && entry.view.contains_visible(touch.position))?
            .view;
        let pans = TouchStack::scrolls().any(|scroll| {
            scroll.is_ok()
                && scroll.__moves_sideways()
                && !scroll.is_hidden_in_tree()
                && scroll.contains_visible(touch.position)
        });
        (!pans).then_some(view)
    }

    fn watch(touch: &Touch, start: Point, view: WeakView) {
        let moved = touch.position - start;
        match touch.event {
            TouchEvent::Moved if moved.x.abs() >= SLOP && moved.x.abs() > moved.y.abs() => {
                Self::begin(touch, start, view);
            }
            TouchEvent::Moved if moved.y.abs() < SLOP => (),
            _ => *PHASE.get_mut() = Phase::Idle,
        }
    }

    fn begin(touch: &Touch, start: Point, view: WeakView) {
        // The views that caught the finger let it go, so the release is no
        // tap and no scroll.
        TouchStack::cancel_touch(touch.id);
        TouchStack::release_scrolls(touch.id);
        log::debug!("side swipe began on {}", view.label());
        *PHASE.get_mut() = Phase::Dragging {
            id: touch.id,
            start: start.x,
            offset: 0.0,
            step: 0.0,
            view,
        };
        Self::drag(touch);
    }

    fn drag(touch: &Touch) {
        let Phase::Dragging {
            start,
            offset,
            step,
            view,
            ..
        } = PHASE.get_mut()
        else {
            return;
        };
        let now = touch.position.x - *start;
        // The release comes where the last move was, the step of a fling is
        // the one before it.
        if touch.is_moved() {
            *step = now - *offset;
            *offset = now;
        }
        let view = *view;
        if touch.is_ended() {
            Self::release(view);
        } else {
            Self::send(view, SideSwipeEvent::Moved(now));
        }
    }

    fn release(view: WeakView) {
        let Phase::Dragging { offset, step, .. } = take(PHASE.get_mut()) else {
            return;
        };
        let half = if view.is_ok() { view.width() / 2.0 } else { 0.0 };
        let end = if offset < 0.0 && (-offset > half || step <= -FLING) {
            SideSwipeEnd::Left
        } else if offset > 0.0 && (offset > half || step >= FLING) {
            SideSwipeEnd::Right
        } else {
            SideSwipeEnd::Back
        };
        log::debug!("side swipe released at {offset} of {half} half width, step {step}, {end:?}");
        Self::send(view, SideSwipeEvent::Ended(end));
    }

    fn send(view: WeakView, event: SideSwipeEvent) {
        if !view.is_ok() {
            return;
        }
        // The callback can set the swipe of another view, so the list is
        // not held while it runs.
        let Some(mut on) = ENTRIES
            .get_mut()
            .iter_mut()
            .find(|entry| entry.view.raw() == view.raw())
            .and_then(|entry| entry.on.take())
        else {
            return;
        };
        on(event);
        if let Some(entry) = ENTRIES
            .get_mut()
            .iter_mut()
            .find(|entry| entry.view.raw() == view.raw() && entry.on.is_none())
        {
            entry.on = Some(on);
        }
    }
}
