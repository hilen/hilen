use log::warn;
use parking_lot::Mutex;
pub use winit::keyboard::{ModifiersState, NamedKey};

#[cfg(any(desktop, wasm))]
use crate::ui::Hover;
use crate::{
    deps::refs::Weak,
    gm::{color::Color, flat::Point},
    level::LevelManager,
    ui::{
        Container, Scrollable, Setup, Touch, TouchStack, UIEvents, UIManager, ViewData, ViewFrame,
        ViewSubviews, WeakView, check_touch,
    },
};

const LOG_TOUCHES: bool = false;
const DRAW_TOUCHES: bool = false;

static MODIFIERS: Mutex<ModifiersState> = Mutex::new(ModifiersState::empty());

pub(crate) struct Input;

impl Input {
    pub(crate) fn set_modifiers(modifiers: ModifiersState) {
        *MODIFIERS.lock() = modifiers;
    }

    /// The modifier keys held right now, so a key handler can tell Shift
    /// plus an arrow or Cmd plus a letter from the plain key.
    pub(crate) fn modifiers() -> ModifiersState {
        *MODIFIERS.lock()
    }

    /// Cmd on a Mac, Ctrl everywhere else, the key that turns a letter
    /// into a command like copy or select all.
    pub(crate) fn command_held() -> bool {
        let modifiers = Self::modifiers();
        modifiers.super_key() || modifiers.control_key()
    }

    pub(crate) fn on_char(ch: char) {
        UIManager::keymap().check(ch);
        UIEvents::keyboard_input().trigger(ch);
    }

    pub(crate) fn on_key(key: NamedKey) {
        UIManager::keymap().check(key);
        UIEvents::keyboard_key().trigger(key);
    }

    pub(crate) fn on_scroll(offset: Point) {
        UIEvents::on_scroll().trigger(offset);
        Self::check_wheel_scroll(offset);

        // Scroll moves content under a still cursor. Re-pick the hovered
        // view. Frames update on the next layout, so a scroll burst is
        // one event behind until the cursor moves again.
        #[cfg(any(desktop, wasm))]
        Hover::update(UIManager::cursor_position());
    }

    pub(crate) fn process_touch_event(mut touch: Touch) -> bool {
        UIEvents::on_debug_touch().trigger(touch);

        if UIManager::touch_disabled() && touch.is_began() {
            return false;
        }

        let original_pos = touch.position;

        touch.position *= 1.0 / UIManager::scale();

        UIManager::set_cursor_position(touch.position);
        UIEvents::on_touch().trigger(touch);

        #[cfg(any(desktop, wasm))]
        if touch.is_moved() {
            Hover::update(touch.position);
        }

        if LOG_TOUCHES && !touch.is_moved() {
            warn!("{touch:?}");
        }

        if (DRAW_TOUCHES || UIManager::draw_touches()) && !touch.is_moved() {
            let mut view = Container::new();
            view.set_z_position(0.1);
            view.set_size(5, 5).set_color(Color::random());
            view.set_center(touch.position);
            let mark = UIManager::root_view().add_subview_to_root(view);
            UIManager::add_touch_mark(mark);
        }

        Self::check_scroll_touches(touch);

        for view in TouchStack::touch_views() {
            if check_touch(view, &mut touch) {
                return true;
            }
        }

        if touch.is_began() && !LevelManager::no_level() {
            return LevelManager::level_weak().add_touch(original_pos);
        }

        false
    }
}

impl Input {
    fn check_scroll_touches(touch: Touch) {
        for mut scroll in TouchStack::scrolls() {
            if scroll.__process_scroll_touch(touch) {
                return;
            }
        }
    }

    /// Wheel scroll goes to a single scroll view of the top touch layer:
    /// the deepest visible one under the cursor.
    fn check_wheel_scroll(delta: Point) {
        let cursor = UIManager::cursor_position();

        let mut deepest: Option<(usize, Weak<dyn Scrollable>)> = None;

        for scroll in TouchStack::scrolls() {
            if scroll.is_null() || scroll.is_hidden_in_tree() || !scroll.absolute_frame().contains(cursor) {
                continue;
            }

            let depth = view_depth(scroll.weak_view());

            if deepest.as_ref().is_none_or(|(deepest_depth, _)| depth >= *deepest_depth) {
                deepest = Some((depth, scroll));
            }
        }

        if let Some((_, mut scroll)) = deepest {
            scroll.__process_wheel_scroll(delta);
        }
    }
}

fn view_depth(view: WeakView) -> usize {
    let mut depth = 0;
    let mut superview = *view.superview();

    while superview.is_ok() {
        depth += 1;
        superview = *superview.superview();
    }

    depth
}
