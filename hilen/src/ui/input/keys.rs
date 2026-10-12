use parking_lot::Mutex;
use winit::keyboard::{KeyCode, ModifiersState};

use crate::ui::Input;

/// Physical keys, so `w` is the key left of `e` on every layout.
static HELD: Mutex<Vec<KeyCode>> = Mutex::new(Vec::new());

/// The keys held down right now. The key events fire once per press,
/// with the system's repeat, which is right for typing and wrong for
/// moving, where a held key has to move every frame.
pub struct Keys;

impl Keys {
    pub fn held(code: KeyCode) -> bool {
        HELD.lock().contains(&code)
    }

    /// The modifier keys held right now: Shift, Ctrl, Alt and Cmd. A
    /// handler of `UIEvents::keyboard_input` or `UIEvents::keyboard_key`
    /// reads it to tell Ctrl with a letter from the plain letter, the
    /// way a terminal has to.
    pub fn modifiers() -> ModifiersState {
        Input::modifiers()
    }

    pub(crate) fn set(code: KeyCode, pressed: bool) {
        let mut held = HELD.lock();
        held.retain(|held| *held != code);
        if pressed {
            held.push(code);
        }
    }

    pub(crate) fn clear() {
        HELD.lock().clear();
    }
}

#[cfg(test)]
mod test {
    use winit::keyboard::ModifiersState;

    use super::Keys;
    use crate::ui::Input;

    #[test]
    fn an_app_reads_the_modifiers_the_window_reported() {
        Input::set_modifiers(ModifiersState::CONTROL | ModifiersState::SHIFT);
        let held = Keys::modifiers();
        Input::set_modifiers(ModifiersState::empty());

        assert!(held.control_key() && held.shift_key());
        assert!(!held.alt_key() && !held.super_key());
        assert!(Keys::modifiers().is_empty());
    }
}
