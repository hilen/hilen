use log::warn;
use parking_lot::Mutex;
use winit::event::TouchPhase;
pub use winit::{
    keyboard::{ModifiersState, NamedKey},
    window::CursorIcon,
};

#[cfg(feature = "level")]
use crate::level::LevelManager;
#[cfg(feature = "scene")]
use crate::scene::SceneManager;
#[cfg(any(desktop, wasm))]
use crate::ui::Hover;
use crate::{
    deps::refs::Weak,
    gm::{color::Color, flat::Point},
    ui::{
        Container, Cursor, Focus, LongPress, Mouse, PinchInput, Scrollable, Setup, TextField, TextSelection,
        Tooltip, Touch, TouchStack, UIEvents, UIManager, ViewData, ViewFrame, check_touch, depth_key,
        press_keeps_selection,
    },
    window::frame_control,
};

const LOG_TOUCHES: bool = false;
const DRAW_TOUCHES: bool = false;

/// How much a pinch in a browser grows the content per pixel of its wheel
/// turn, the step browsers themselves zoom a page by.
#[cfg(wasm)]
const BROWSER_PINCH_STEP: f32 = 0.01;

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
        frame_control::input_arrived();
        UIManager::keymap().check(ch);
        TextSelection::on_char(ch);
        UIEvents::keyboard_input().trigger(ch);
    }

    pub(crate) fn on_key(key: NamedKey) {
        frame_control::input_arrived();
        // A game's Escape gives the mouse back and nothing else sees it.
        if key == NamedKey::Escape && Cursor::captured() {
            Cursor::release();
            return;
        }
        if Focus::on_key(key) {
            return;
        }
        // Before the keymap, a key the owner of the edited field took
        // must not also run a binding of the screen, like its Escape.
        if TextField::offer_key(key) {
            return;
        }
        UIManager::keymap().check(key);
        UIEvents::keyboard_key().trigger(key);
    }

    /// A pinch on a trackpad. `growth` is the share the content grew by
    /// since the last event, what macOS calls the magnification. The view
    /// under the cursor at the start keeps the pinch until it ends.
    pub(crate) fn on_trackpad_pinch(growth: f32, phase: TouchPhase) {
        match phase {
            TouchPhase::Started => PinchInput::begin(UIManager::cursor_position()),
            TouchPhase::Moved => (),
            TouchPhase::Ended | TouchPhase::Cancelled => {
                PinchInput::end();
                return;
            }
        }
        PinchInput::step(1.0 + growth, UIManager::cursor_position(), Point::default());
    }

    /// A turn of the wheel as the window reports it.
    pub(crate) fn on_wheel(delta: Point) {
        // A browser reports a pinch on a trackpad as a wheel turn with
        // Ctrl held, it has no pinch event of its own.
        #[cfg(wasm)]
        if Self::modifiers().control_key() {
            let growth = -delta.y * BROWSER_PINCH_STEP;
            PinchInput::step(1.0 + growth, UIManager::cursor_position(), Point::default());
            return;
        }
        Self::on_scroll(delta);
    }

    pub(crate) fn on_scroll(offset: Point) {
        frame_control::input_arrived();
        UIEvents::on_scroll().trigger(offset);
        Self::check_wheel_scroll(offset);

        // Scroll moves content under a still cursor. Re-pick the hovered
        // view. Frames update on the next layout, so a scroll burst is
        // one event behind until the cursor moves again.
        #[cfg(any(desktop, wasm))]
        Hover::update(UIManager::cursor_position());
    }

    /// Offers a touch to the views of the top touch layer, the front one
    /// first, and says whether one took it. Its position is in points on
    /// the screen. An overlay that closed itself on a press calls this to
    /// let the same press through to what was under it.
    pub(crate) fn offer_to_views(touch: &mut Touch) -> bool {
        let keeps_selection = press_keeps_selection(touch);
        TouchStack::touch_views().any(|view| {
            let covered = touch.is_began() && TouchStack::covered(view, touch.position);
            !covered && check_touch(view, touch, keeps_selection)
        })
    }

    pub(crate) fn process_touch_event(mut touch: Touch) -> bool {
        UIEvents::on_debug_touch().trigger(touch);

        if touch.is_began() {
            frame_control::input_arrived();
        }

        // Before any view can take the touch, the held state is raw.
        Mouse::on_touch(&touch);
        Focus::pointer_used(&touch);

        if UIManager::touch_disabled() && touch.is_began() {
            return false;
        }

        #[cfg(any(feature = "level", feature = "scene"))]
        let original_pos = touch.position;

        touch.position *= 1.0 / UIManager::scale();

        UIManager::set_cursor_position(touch.position);
        UIEvents::on_touch().trigger(touch);
        if touch.is_began() {
            UIEvents::touch_began().trigger(touch);
        }
        TextSelection::on_touch(&touch);

        // Any press ends a tooltip, a finger or a click is an answer to it.
        if touch.is_began() {
            Tooltip::hide();
        }

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

        // The 2 fingers of a pinch belong to it, no view sees them move.
        if PinchInput::swallow(&touch) {
            return true;
        }

        Self::check_scroll_touches(touch);

        if touch.is_moved() {
            LongPress::moved(touch.id, touch.position);
        }

        if touch.is_ended() {
            LongPress::cancel(touch.id);
        }

        let on_screen = touch;
        let taken = Self::offer_to_views(&mut touch);

        // After the views, a pinch looks at which view took each finger.
        if on_screen.is_began() {
            PinchInput::finger_down(&on_screen);
        }

        if taken {
            return true;
        }

        if touch.is_began() {
            LongPress::arm_tooltip_hold(touch.id, touch.position);
        }

        #[cfg(feature = "level")]
        if touch.is_began() && !LevelManager::no_level() {
            return LevelManager::level_weak().add_touch(original_pos);
        }

        #[cfg(feature = "scene")]
        if touch.is_began() && !SceneManager::no_scene() {
            return SceneManager::scene_weak().add_touch(original_pos);
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
    /// the one drawn in front under the cursor. So a scroll view floating
    /// over a deeper one, a toast or a popover, wins the overlap. Smaller
    /// z is closer, and a child is closer than its parent, so a nested
    /// scroll view still beats the one around it.
    fn check_wheel_scroll(delta: Point) {
        let cursor = UIManager::cursor_position();

        let mut front: Option<((f32, f32), Weak<dyn Scrollable>)> = None;

        for scroll in TouchStack::scrolls() {
            if scroll.is_null() || scroll.is_hidden_in_tree() || !scroll.absolute_frame().contains(cursor) {
                continue;
            }

            let z = depth_key(scroll.weak_view());

            if front.as_ref().is_none_or(|(front_z, _)| z <= *front_z) {
                front = Some((z, scroll));
            }
        }

        if let Some((_, mut scroll)) = front {
            scroll.__process_wheel_scroll(delta);
        }
    }
}
