use parking_lot::Mutex;

use crate::ui::{Code, Input, Modifiers};

/// Physical keys, so `w` is the key left of `e` on every layout.
static HELD: Mutex<Vec<Code>> = Mutex::new(Vec::new());

/// The keys held down right now. The key events fire once per press,
/// with the system's repeat, which is right for typing and wrong for
/// moving, where a held key has to move every frame.
pub struct Keys;

impl Keys {
    pub fn held(code: Code) -> bool {
        HELD.lock().contains(&code)
    }

    /// The modifier keys held right now: Shift, Ctrl, Alt and Cmd. A
    /// handler of `UIEvents::keyboard_input` or `UIEvents::keyboard_key`
    /// reads it to tell Ctrl with a letter from the plain letter, the
    /// way a terminal has to.
    pub fn modifiers() -> Modifiers {
        Input::modifiers()
    }

    #[cfg(feature = "gpu")]
    pub(crate) fn set(code: Code, pressed: bool) {
        let mut held = HELD.lock();
        held.retain(|held| *held != code);
        if pressed {
            held.push(code);
        }
    }

    #[cfg(feature = "gpu")]
    pub(crate) fn clear() {
        HELD.lock().clear();
    }
}

#[cfg(all(test, feature = "gpu"))]
mod test {
    use super::Keys;
    use crate::ui::{Input, Modifiers};

    #[test]
    fn an_app_reads_the_modifiers_the_window_reported() {
        Input::set_modifiers(Modifiers::CONTROL | Modifiers::SHIFT);
        let held = Keys::modifiers();
        Input::set_modifiers(Modifiers::empty());

        assert!(held.ctrl() && held.shift());
        assert!(!held.alt() && !held.meta());
        assert!(Keys::modifiers().is_empty());
    }
}
