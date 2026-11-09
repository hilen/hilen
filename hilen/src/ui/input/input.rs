pub use cursor_icon::CursorIcon;
use log::warn;
use parking_lot::Mutex;
#[cfg(feature = "gpu")]
use winit::event::{MouseScrollDelta, TouchPhase};

use super::Modifiers;
#[cfg(feature = "gpu")]
use super::NamedKey;
#[cfg(feature = "level")]
use crate::level::LevelManager;
#[cfg(feature = "scene")]
use crate::scene::SceneManager;
#[cfg(any(desktop, wasm))]
use crate::ui::Hover;
#[cfg(feature = "gpu")]
use crate::{
    deps::refs::Weak,
    gm::flat::Point,
    ui::{Cursor, DropDown, Scrollable, TextField, depth_key},
};
use crate::{
    gm::color::Color,
    ui::{
        Container, Focus, LongPress, Mouse, NavigationView, PinchInput, RowDrag, Setup, SideSwipe,
        TextSelection, Tooltip, Touch, TouchStack, UIEvents, UIManager, ViewData, ViewFrame, check_touch,
        press_keeps_selection,
    },
    window::frame_control,
};

/// Points one wheel line scrolls. X11 and Windows report a wheel notch as
/// one line, this is about the Windows default of three text lines. It
/// used to be 28 and then went through a trackpad factor as well, 7 a
/// notch, a 30 point row took four notches.
#[cfg(feature = "gpu")]
const LINE_SCROLL_POINTS: f32 = 60.0;

/// The points a wheel event moves the content, whichever unit the platform
/// sent. A trackpad reports physical pixels, the scale turns them into
/// points, so the content moves exactly as far as the fingers, like in
/// every other app. It used to move a quarter of the pixels, half the
/// distance on a 2x screen.
#[cfg(feature = "gpu")]
pub(crate) fn wheel_points_at(delta: MouseScrollDelta, scale: f32) -> Point {
    match delta {
        MouseScrollDelta::LineDelta(x, y) => Point::new(x, y) * LINE_SCROLL_POINTS,
        MouseScrollDelta::PixelDelta(delta) => {
            let pixels: Point = (delta.x, delta.y).into();
            pixels * (1.0 / scale)
        }
    }
}

const LOG_TOUCHES: bool = false;
const DRAW_TOUCHES: bool = false;

/// How much a pinch in a browser grows the content per pixel of its wheel
/// turn, the step browsers themselves zoom a page by.
#[cfg(wasm)]
const BROWSER_PINCH_STEP: f32 = 0.01;

static MODIFIERS: Mutex<Modifiers> = Mutex::new(Modifiers::empty());

/// The Latin character of the key whose character is being handed out
/// right now, see `Input::shortcut_char`.
static LATIN: Mutex<Option<char>> = Mutex::new(None);

pub(crate) struct Input;

impl Input {
    /// The points a wheel event of the window moves the content, at the UI
    /// scale. None for the end of a trackpad scroll: macOS can merge the end
    /// of 1 scroll with the start of the next while the main thread is busy,
    /// the end then carries the first delta of the next scroll and that start
    /// arrives with it again. A real end carries no delta, the web views and
    /// the native views of macOS move nothing on it either.
    #[cfg(feature = "gpu")]
    pub(crate) fn wheel_points(delta: MouseScrollDelta, phase: TouchPhase) -> Option<Point> {
        (phase != TouchPhase::Ended).then(|| wheel_points_at(delta, UIManager::scale()))
    }

    #[cfg(feature = "gpu")]
    pub(crate) fn set_modifiers(modifiers: Modifiers) {
        *MODIFIERS.lock() = modifiers;
    }

    /// The modifier keys held right now, so a key handler can tell Shift
    /// plus an arrow or Cmd plus a letter from the plain key.
    pub(crate) fn modifiers() -> Modifiers {
        *MODIFIERS.lock()
    }

    /// Cmd on a Mac, Ctrl everywhere else, the key that turns a letter
    /// into a command like copy or select all.
    pub(crate) fn command_held() -> bool {
        let modifiers = Self::modifiers();
        modifiers.meta() || modifiers.ctrl()
    }

    /// A typed character. `latin` is the Latin one of the same key when the
    /// layout typed another, see `latin_candidate`. Bindings and commands
    /// read it, text gets only the typed one.
    #[cfg(feature = "gpu")]
    pub(crate) fn on_char(ch: char, latin: Option<char>) {
        frame_control::input_arrived();
        *LATIN.lock() = latin;
        UIManager::keymap().check_char(ch, latin);
        TextSelection::on_char(Self::shortcut_char(ch));
        UIEvents::keyboard_input().trigger(ch);
        *LATIN.lock() = None;
    }

    /// The character a command like Cmd and C reads from a typed one: the
    /// Latin character of the key when the layout typed another one.
    pub(crate) fn shortcut_char(ch: char) -> char {
        LATIN.lock().unwrap_or(ch)
    }

    #[cfg(feature = "gpu")]
    pub(crate) fn on_key(key: NamedKey) {
        frame_control::input_arrived();
        // A game's Escape gives the mouse back and nothing else sees it.
        if key == NamedKey::Escape && Cursor::captured() {
            Cursor::release();
            return;
        }
        if key == NamedKey::Escape && RowDrag::escape() {
            return;
        }
        // An open list goes first, Escape must not also close the modal it is
        // in.
        if key == NamedKey::Escape && DropDown::escape() {
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

    /// The release of a key. Only the key focus reads it, for a held Enter.
    #[cfg(feature = "gpu")]
    pub(crate) fn on_key_up(key: NamedKey) {
        Focus::on_key_up(key);
    }

    /// A pinch on a trackpad. `growth` is the share the content grew by
    /// since the last event, what macOS calls the magnification. The view
    /// under the cursor at the start keeps the pinch until it ends.
    #[cfg(feature = "gpu")]
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
    #[cfg(feature = "gpu")]
    pub(crate) fn on_wheel(delta: Point) {
        // A browser reports a pinch on a trackpad as a wheel turn with
        // Ctrl held, it has no pinch event of its own.
        #[cfg(wasm)]
        if Self::modifiers().ctrl() {
            let growth = -delta.y * BROWSER_PINCH_STEP;
            PinchInput::step(1.0 + growth, UIManager::cursor_position(), Point::default());
            return;
        }
        Self::on_scroll(delta);
    }

    #[cfg(feature = "gpu")]
    pub(crate) fn on_scroll(offset: Point) {
        frame_control::input_arrived();
        UIEvents::on_scroll().trigger(offset);
        Self::check_wheel_scroll(offset);

        // Scroll moves content under a still cursor. The hovered view is
        // picked again after the next layout, when the frames have moved.
        #[cfg(any(desktop, wasm))]
        Hover::scrolled();
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

        // A swipe from the left edge of a navigation stack takes the finger
        // once it moves right.
        if NavigationView::swallow(&touch) {
            SideSwipe::reset();
            RowDrag::reset();
            return true;
        }

        // A held row of a table follows the finger.
        if RowDrag::swallow(&touch) {
            SideSwipe::reset();
            return true;
        }

        // A swipe left or right over a view that asked for it.
        if SideSwipe::swallow(&touch) {
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

        // A scene in a `SceneView` gets its touches from that view.
        #[cfg(feature = "scene")]
        if touch.is_began() && !SceneManager::no_scene() && SceneManager::host().is_none() {
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
    ///
    /// A scroll view that cannot move by `delta`, like a text field with
    /// nothing to scroll or a list at its end, passes the move on to the
    /// next one behind it, the way a page in a browser does.
    #[cfg(feature = "gpu")]
    fn check_wheel_scroll(delta: Point) {
        if let Some(mut scroll) = Self::wheel_scroll_taking(delta) {
            scroll.__process_wheel_scroll(delta);
        }
    }

    /// The front scrollable view under the cursor that a wheel move by
    /// `delta` moves.
    #[cfg(feature = "gpu")]
    fn wheel_scroll_taking(delta: Point) -> Option<Weak<dyn Scrollable>> {
        let cursor = UIManager::cursor_position();

        let mut front: Option<((f32, f32), Weak<dyn Scrollable>)> = None;

        for scroll in TouchStack::scrolls() {
            if scroll.is_null()
                || scroll.is_hidden_in_tree()
                || !scroll.absolute_frame().contains(cursor)
                || !scroll.__takes_wheel(delta)
            {
                continue;
            }

            let z = depth_key(scroll.weak_view());

            if front.as_ref().is_none_or(|(front_z, _)| z <= *front_z) {
                front = Some((z, scroll));
            }
        }

        front.map(|(_, scroll)| scroll)
    }

    /// Whether the browser keeps a wheel move, in the delta of the engine,
    /// so a sideways swipe of 2 fingers goes back or forward in the page
    /// history. It does when the move is mostly sideways and the view under
    /// the cursor cannot take it, the rule a page of plain html follows.
    #[cfg(wasm)]
    pub(crate) fn wheel_goes_to_browser(delta: Point) -> bool {
        delta.x.abs() > delta.y.abs()
            && Self::wheel_scroll().is_none_or(|scroll| !scroll.__takes_wheel_x(delta.x))
    }

    /// The front scrollable view under the cursor, for the sideways swipe
    /// a browser may keep.
    #[cfg(wasm)]
    fn wheel_scroll() -> Option<Weak<dyn Scrollable>> {
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

        front.map(|(_, scroll)| scroll)
    }
}

#[cfg(all(test, feature = "gpu"))]
mod wheel_tests {
    use winit::{dpi::PhysicalPosition, event::MouseScrollDelta};

    use super::wheel_points_at;

    // One X11 or Windows wheel notch is one line and must move a few rows,
    // and a trackpad moves the content as far as the fingers, on any scale.
    #[test]
    fn a_wheel_notch_is_sixty_points_and_a_trackpad_moves_its_own_distance() {
        let notch = wheel_points_at(MouseScrollDelta::LineDelta(0.0, -1.0), 2.0);
        assert_eq!((notch.x, notch.y), (0.0, -60.0));
        let retina = wheel_points_at(
            MouseScrollDelta::PixelDelta(PhysicalPosition::new(8.0, -40.0)),
            2.0,
        );
        assert_eq!((retina.x, retina.y), (4.0, -20.0));
        let plain = wheel_points_at(
            MouseScrollDelta::PixelDelta(PhysicalPosition::new(8.0, -40.0)),
            1.0,
        );
        assert_eq!((plain.x, plain.y), (8.0, -40.0));
    }
}
