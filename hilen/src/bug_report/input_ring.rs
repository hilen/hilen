use std::collections::VecDeque;

use parking_lot::Mutex;
use serde::Serialize;

use crate::ui::{Code, Modifiers};

/// Enough to show what led to the bug without turning into surveillance.
const CAPACITY: usize = 20;

/// One recorded key press. Only the physical key code and the held
/// modifier names are stored, never the typed character, so the ring can
/// not become a keylogger.
#[derive(Debug, Clone, Serialize)]
pub struct KeyPress {
    pub code:  String,
    pub mods:  Vec<String>,
    pub at_ms: i64,
}

impl KeyPress {
    /// One line for the dialog and the attachment viewer, relative to the
    /// first recorded press.
    pub(crate) fn display(&self, first_ms: i64) -> String {
        let delta_ms = self.at_ms - first_ms;
        let seconds = delta_ms / 1000;
        let hundredths = delta_ms % 1000 / 10;

        let mut combo = self.mods.join("+");

        if !combo.is_empty() {
            combo.push('+');
        }

        combo.push_str(&self.code);

        format!("+{seconds}.{hundredths:02}s  {combo}")
    }
}

struct RingState {
    events:    VecDeque<KeyPress>,
    modifiers: Modifiers,
}

static STATE: Mutex<RingState> = Mutex::new(RingState {
    events:    VecDeque::new(),
    modifiers: Modifiers::empty(),
});

/// A RAM only ring of recent key presses. Attached to a bug report only
/// when the reporter opts in. Plain typing is never recorded, only
/// modifier combos and navigation keys qualify.
pub(crate) struct InputRing;

impl InputRing {
    pub(crate) fn set_modifiers(modifiers: Modifiers) {
        STATE.lock().modifiers = modifiers;
    }

    /// Only the native shortcut reads it, the browser keeps the combo.
    #[cfg(not_wasm)]
    pub(crate) fn modifiers() -> Modifiers {
        STATE.lock().modifiers
    }

    pub(crate) fn record(code: Code) {
        let mut state = STATE.lock();

        if !qualifies(code, state.modifiers) {
            return;
        }

        let mut mods = Vec::new();

        if state.modifiers.ctrl() {
            mods.push("Ctrl".to_string());
        }
        if state.modifiers.alt() {
            mods.push("Alt".to_string());
        }
        if state.modifiers.shift() {
            mods.push("Shift".to_string());
        }
        if state.modifiers.meta() {
            mods.push("Meta".to_string());
        }

        let press = KeyPress {
            code: format!("{code:?}"),
            mods,
            at_ms: chrono::Utc::now().timestamp_millis(),
        };

        if state.events.len() == CAPACITY {
            state.events.pop_front();
        }

        state.events.push_back(press);
    }

    pub(crate) fn snapshot() -> Vec<KeyPress> {
        STATE.lock().events.iter().cloned().collect()
    }
}

/// A press qualifies when a non shift modifier is held, a real hotkey, or
/// the key itself is a navigation or control key. Shift plus letter is
/// capital typing, not a hotkey. A modifier key on its own never
/// qualifies, holding one would otherwise burn ring slots.
fn qualifies(code: Code, modifiers: Modifiers) -> bool {
    if is_modifier(code) {
        return false;
    }

    if modifiers.ctrl() || modifiers.alt() || modifiers.meta() {
        return true;
    }

    is_navigation(code)
}

fn is_modifier(code: Code) -> bool {
    matches!(
        code,
        Code::ControlLeft
            | Code::ControlRight
            | Code::ShiftLeft
            | Code::ShiftRight
            | Code::AltLeft
            | Code::AltRight
            | Code::MetaLeft
            | Code::MetaRight
    )
}

fn is_navigation(code: Code) -> bool {
    matches!(
        code,
        Code::Escape
            | Code::Tab
            | Code::Enter
            | Code::NumpadEnter
            | Code::ArrowUp
            | Code::ArrowDown
            | Code::ArrowLeft
            | Code::ArrowRight
            | Code::Home
            | Code::End
            | Code::PageUp
            | Code::PageDown
            | Code::F1
            | Code::F2
            | Code::F3
            | Code::F4
            | Code::F5
            | Code::F6
            | Code::F7
            | Code::F8
            | Code::F9
            | Code::F10
            | Code::F11
            | Code::F12
    )
}

#[cfg(test)]
mod tests {
    use super::qualifies;
    use crate::ui::{Code, Modifiers};

    #[test]
    fn plain_typing_is_dropped() {
        assert!(!qualifies(Code::KeyA, Modifiers::empty()));
        assert!(!qualifies(Code::Space, Modifiers::empty()));
        assert!(!qualifies(Code::Digit5, Modifiers::empty()));
    }

    #[test]
    fn shift_letter_is_capital_typing_not_a_hotkey() {
        assert!(!qualifies(Code::KeyA, Modifiers::SHIFT));
    }

    #[test]
    fn modifier_combos_qualify() {
        assert!(qualifies(Code::KeyC, Modifiers::CONTROL));
        assert!(qualifies(Code::KeyR, Modifiers::META | Modifiers::SHIFT));
        assert!(qualifies(Code::KeyF, Modifiers::ALT));
    }

    #[test]
    fn navigation_keys_qualify_bare() {
        assert!(qualifies(Code::Escape, Modifiers::empty()));
        assert!(qualifies(Code::Enter, Modifiers::empty()));
        assert!(qualifies(Code::ArrowLeft, Modifiers::empty()));
        assert!(qualifies(Code::F5, Modifiers::empty()));
    }

    #[test]
    fn modifier_only_presses_are_dropped() {
        assert!(!qualifies(Code::ShiftLeft, Modifiers::SHIFT));
        assert!(!qualifies(Code::MetaLeft, Modifiers::META));
        assert!(!qualifies(Code::ControlLeft, Modifiers::CONTROL));
    }
}
