use crate::{
    deps::refs::main_lock::MainLock,
    gm::flat::Point,
    ui::{NO_TOUCH_ID, Touch, TouchStack, ViewSubviews, WeakView, input::TouchEvent},
    window::MouseButton,
};

/// One step of a pinch, what a view gets from `touch().pinch` after
/// `enable_pinch`. 2 fingers on a touch screen make it, and so does a
/// pinch on a trackpad.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Pinch {
    /// How much the distance between the fingers grew since the last
    /// step, 1 is no change. Multiply a zoom by it.
    pub scale:  f32,
    /// The middle between the fingers, or the cursor for a trackpad, in
    /// the points of the view.
    pub center: Point,
    /// How far the middle moved since the last step, in the points of the
    /// view. Always 0 for a trackpad.
    pub shift:  Point,
}

/// The view a running pinch belongs to. A pinch keeps the view it began
/// over, the middle of 2 fingers can leave it.
static TARGET: MainLock<WeakView> = MainLock::new();

/// Every finger on the screen, by touch id, in screen points.
static FINGERS: MainLock<Vec<(usize, Point)>> = MainLock::new();

/// The 2 touch ids of a finger pinch in flight.
static PAIR: MainLock<Option<(usize, usize)>> = MainLock::new();

pub(crate) struct PinchInput;

impl PinchInput {
    /// A pinch begins at `center`, in screen points. The view under it
    /// gets every step until `end`.
    pub(crate) fn begin(center: Point) {
        *TARGET.get_mut() = TouchStack::pinch_view_at(center).unwrap_or_default();
    }

    pub(crate) fn end() {
        *TARGET.get_mut() = WeakView::default();
    }

    /// One step, `center` and `shift` in screen points. Without a `begin`
    /// the view under `center` gets it.
    pub(crate) fn step(scale: f32, center: Point, shift: Point) {
        let mut target = *TARGET;
        if !target.is_ok() {
            target = TouchStack::pinch_view_at(center).unwrap_or_default();
        }
        if !target.is_ok() {
            return;
        }

        let base = target.__base_view();
        base.events.touch.pinch.trigger(Pinch {
            scale,
            center: base.local_point(center),
            shift: shift / base.tree_scale,
        });
    }

    /// Follows the fingers. True when the touch belongs to a pinch in
    /// flight and no view may see it. `touch` is in screen points.
    pub(crate) fn swallow(touch: &Touch) -> bool {
        if touch.button != MouseButton::Left {
            return false;
        }
        let Some((first, second)) = *PAIR else {
            Self::track(touch);
            return false;
        };
        if touch.id != first && touch.id != second {
            Self::track(touch);
            return false;
        }

        match touch.event {
            TouchEvent::Began => (),
            TouchEvent::Moved => {
                let before = Self::span(first, second);
                Self::track(touch);
                let after = Self::span(first, second);
                if let (Some((old_middle, old_length)), Some((middle, length))) = (before, after)
                    && old_length > 0.0
                {
                    Self::step(length / old_length, middle, middle - old_middle);
                }
            }
            TouchEvent::Ended => {
                Self::track(touch);
                *PAIR.get_mut() = None;
                Self::end();
            }
        }
        true
    }

    /// A finger went down and the views had their chance to take it. With
    /// 2 fingers down over a view that takes a pinch, the pinch begins and
    /// both fingers leave the views that captured them, so the release of
    /// a pinch is no tap.
    pub(crate) fn finger_down(touch: &Touch) {
        if touch.button != MouseButton::Left || PAIR.is_some() || FINGERS.len() != 2 {
            return;
        }
        let (first, second) = (FINGERS[0].0, FINGERS[1].0);
        let Some((middle, _)) = Self::span(first, second) else {
            return;
        };
        let Some(target) = TouchStack::pinch_view_at(middle) else {
            return;
        };

        // A finger held by a view outside the target is busy there, 2
        // thumbs on 2 buttons over a map are no pinch.
        for id in [first, second] {
            if let Some(holder) = holder_of(id)
                && !is_inside(holder, target)
            {
                return;
            }
        }

        TouchStack::cancel_touch(first);
        TouchStack::cancel_touch(second);
        *PAIR.get_mut() = Some((first, second));
        *TARGET.get_mut() = target;
    }

    /// A new test starts with no finger down.
    pub(crate) fn reset() {
        FINGERS.get_mut().clear();
        *PAIR.get_mut() = None;
        Self::end();
    }

    fn track(touch: &Touch) {
        let fingers = FINGERS.get_mut();
        match touch.event {
            TouchEvent::Began => {
                fingers.retain(|(id, _)| *id != touch.id);
                fingers.push((touch.id, touch.position));
            }
            // A mouse moves with no button down, that is no finger.
            TouchEvent::Moved => {
                if let Some(finger) = fingers.iter_mut().find(|(id, _)| *id == touch.id) {
                    finger.1 = touch.position;
                }
            }
            TouchEvent::Ended => fingers.retain(|(id, _)| *id != touch.id),
        }
    }

    /// The middle between 2 fingers and the distance between them.
    fn span(first: usize, second: usize) -> Option<(Point, f32)> {
        let find =
            |wanted: usize| FINGERS.iter().find(|(id, _)| *id == wanted).map(|(_, position)| *position);
        let (a, b) = (find(first)?, find(second)?);
        Some(((a + b) / 2.0, (a - b).length()))
    }
}

/// The view that captured touch `id` on its began.
fn holder_of(id: usize) -> Option<WeakView> {
    debug_assert_ne!(id, NO_TOUCH_ID);
    TouchStack::touch_views().find(|view| view.is_ok() && view.__base_view().__touch_id == id)
}

fn is_inside(view: WeakView, outer: WeakView) -> bool {
    let mut current = view;
    while current.is_ok() {
        if current.raw() == outer.raw() {
            return true;
        }
        current = *current.superview();
    }
    false
}
