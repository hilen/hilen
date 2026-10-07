//! Where the screen keyboard of a phone is, and what the engine does about
//! it.
//!
//! The system tells the top edge the keyboard moves to and how long the move
//! takes. The engine moves its own top edge along that time, so everything
//! that follows it moves with the keyboard:
//!
//! - a view placed with `b_keyboard` keeps its bottom edge on top of the
//!   keyboard, a compose box,
//! - the edited text field stays in view with no code in the app. The nearest
//!   `ScrollView` around it scrolls as far as it can, and what is still covered
//!   moves the whole screen up.
//!
//! Lengths are in points of the window, the ones `absolute_frame` counts in.

use crate::{
    deps::refs::{Weak, main_lock::MainLock},
    gm::{Clock, LossyConvert},
    ui::{
        ScrollView, TextField, UIEvents, UIManager, View, ViewFrame, ViewSubviews, WeakView,
        layout::Placement,
    },
    window::request_frame,
};

/// The gap kept between the edited field and the keyboard.
const FIELD_GAP: f32 = 8.0;

/// What the system said about the keyboard, the payload of
/// `UIEvents::screen_keyboard`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenKeyboardMove {
    /// The top edge the keyboard moves to, `None` when it goes away.
    pub top:      Option<f32>,
    /// Seconds the move takes.
    pub duration: f32,
}

#[derive(Default)]
struct State {
    /// The top edge the move started from and the one it ends at.
    from:     f32,
    to:       f32,
    /// The keyboard is on the screen, or on its way there.
    up:       bool,
    started:  f64,
    duration: f64,
    /// How far the screen is moved up for the edited field.
    shift:    f32,
    /// The field the screen was last arranged for.
    arranged: WeakView,
    /// The screen was arranged for the place the keyboard stopped at.
    settled:  bool,
    /// The bottom edge the screen was last moved for, with the gap, where
    /// it is with no shift. The screen comes down by it when the field
    /// let the keys go and the keyboard leaves.
    bottom:   f32,
    /// What the system said last, not started yet. The next frame starts
    /// it. iOS says several things in a row for 1 move, a keyboard that
    /// comes up says it is gone in between, and only the last one counts.
    asked:    Option<ScreenKeyboardMove>,
}

static STATE: MainLock<State> = MainLock::new();

pub struct ScreenKeyboard;

impl ScreenKeyboard {
    /// This device types on a screen keyboard, an iPhone or an iPad.
    pub const fn on_this_device() -> bool {
        cfg!(all(ios, not(tvos)))
    }

    /// The top edge of the keyboard now, part way while it moves. `None`
    /// with no keyboard on the screen.
    pub fn top() -> Option<f32> {
        let state = STATE.get_mut();
        (state.up || Self::is_moving()).then(|| current_top(state))
    }

    /// The keyboard is on its way up or down.
    pub fn is_moving() -> bool {
        let state = STATE.get_mut();
        state.asked.is_some() || Clock::now_ms() < state.started + state.duration
    }

    /// How much of the bottom of `view` the keyboard covers now.
    pub fn cover(view: &(impl View + ?Sized)) -> f32 {
        match Self::top() {
            Some(top) => (view.absolute_frame().max_y() - top).max(0.0),
            None => 0.0,
        }
    }

    /// How far the screen is moved up for the edited field.
    pub fn shift() -> f32 {
        STATE.get_mut().shift
    }

    /// The system moves the keyboard to `top`, or away with `None`.
    pub(crate) fn moves_to(top: Option<f32>, duration: f32) {
        log::debug!("screen keyboard moves to {top:?} in {duration} s");
        STATE.get_mut().asked = Some(ScreenKeyboardMove { top, duration });
        request_frame();
    }

    /// Starts the move the system asked for last. The app hears of it
    /// here, on a frame, and not from inside the change of the selected
    /// view the system said it in.
    fn start_asked() {
        let Some(asked) = STATE.get_mut().asked.take() else {
            return;
        };

        let on_screen = Self::top();
        let state = STATE.get_mut();

        state.from = on_screen.unwrap_or_else(window_bottom);
        state.to = asked.top.unwrap_or_else(window_bottom);
        state.up = asked.top.is_some();
        state.started = Clock::now_ms();
        state.duration = f64::from(asked.duration.max(0.0)) * 1000.0;
        state.settled = false;

        UIEvents::screen_keyboard().trigger(asked);
    }

    /// Every frame, before the layout.
    pub(crate) fn update() {
        Self::start_asked();

        let moving = Self::is_moving();
        let state = STATE.get_mut();

        if !state.up && !moving {
            if state.shift != 0.0 {
                set_shift(state, 0.0);
            }
            return;
        }

        let field = edited_field();

        // A keyboard that stands still arranges a field once. After that
        // the user may scroll it out of view.
        if !moving && state.settled && field.raw() == state.arranged.raw() {
            return;
        }
        state.settled = !moving;
        state.arranged = field;

        // The last frame of the move still draws the keyboard part way.
        request_frame();

        let top = current_top(state);

        let covered = match field.get() {
            Some(field) if follows_keyboard(field) => {
                state.bottom = 0.0;
                0.0
            }
            Some(field) => {
                // The frame is the one of the last layout, with the shift
                // in it.
                let bottom = field.absolute_frame().max_y() + state.shift + FIELD_GAP;
                let scrolled = scroll_up(field, (bottom - top).max(0.0));
                state.bottom = bottom - scrolled;
                state.bottom - top
            }
            None => state.bottom - top,
        };

        set_shift(state, covered.max(0.0));
    }

    /// A test that failed with the keyboard up must not leave it there.
    pub(crate) fn reset() {
        let state = STATE.get_mut();
        *state = State::default();
        UIManager::root_view().set_keyboard_shift(0.0);
    }
}

fn window_bottom() -> f32 {
    UIManager::window_resolution().height / UIManager::scale()
}

fn current_top(state: &State) -> f32 {
    if state.duration <= 0.0 {
        return state.to;
    }
    let passed: f32 = ((Clock::now_ms() - state.started) / state.duration).lossy_convert();
    state.from + (state.to - state.from) * ease(passed.clamp(0.0, 1.0))
}

/// Slow at both ends, close to the curve iOS moves its keyboard on.
fn ease(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn set_shift(state: &mut State, shift: f32) {
    if (state.shift - shift).abs() < f32::EPSILON {
        return;
    }
    state.shift = shift;
    UIManager::root_view().set_keyboard_shift(shift);
}

/// The text field that has the keys now.
fn edited_field() -> WeakView {
    let selected = UIManager::selected_view();
    let editing =
        selected.is_ok() && selected.downcast_view::<TextField>().is_some_and(|field| field.is_editing());
    if editing { selected } else { Weak::default() }
}

/// `view` sits in a view that rides on the keyboard, so it is in view
/// already. Moving the screen for it too would move it twice.
fn follows_keyboard(view: &dyn View) -> bool {
    let mut above = view.weak_view();
    while above.is_ok() {
        // The root has no placer of its own.
        let placer = &above.__base_view().placer;
        let rides = placer.is_ok()
            && placer
                .get_rules()
                .iter()
                .any(|rule| matches!(rule.placement, Placement::AboveKeyboard { .. }));
        if rides {
            return true;
        }
        above = *above.superview();
    }
    false
}

/// Scrolls the nearest scroll view around `view` by at most `distance` and
/// returns how far it went.
fn scroll_up(view: &dyn View, distance: f32) -> f32 {
    if distance <= 0.0 {
        return 0.0;
    }
    let mut above = *view.superview();
    while above.is_ok() {
        if let Some(mut scroll) = above.downcast_view::<ScrollView>() {
            return scroll.scroll_up_by(distance);
        }
        above = *above.superview();
    }
    0.0
}

#[cfg(test)]
mod tests {
    use super::ease;

    #[test]
    fn ease_starts_at_0_ends_at_1_and_is_half_way_in_the_middle() {
        assert!(ease(0.0).abs() < f32::EPSILON);
        assert!((ease(1.0) - 1.0).abs() < f32::EPSILON);
        assert!((ease(0.5) - 0.5).abs() < f32::EPSILON);
        assert!(ease(0.25) < 0.25);
        assert!(ease(0.75) > 0.75);
    }
}
