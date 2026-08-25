use std::collections::VecDeque;

use parking_lot::Mutex;
use serde::Serialize;
use winit::keyboard::{KeyCode, ModifiersState};

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
    modifiers: ModifiersState,
}

static STATE: Mutex<RingState> = Mutex::new(RingState {
    events:    VecDeque::new(),
    modifiers: ModifiersState::empty(),
});

/// A RAM only ring of recent key presses. Attached to a bug report only
/// when the reporter opts in. Plain typing is never recorded, only
/// modifier combos and navigation keys qualify.
pub(crate) struct InputRing;

impl InputRing {
    pub(crate) fn set_modifiers(modifiers: ModifiersState) {
        STATE.lock().modifiers = modifiers;
    }

    pub(crate) fn modifiers() -> ModifiersState {
        STATE.lock().modifiers
    }

    pub(crate) fn record(code: KeyCode) {
        let mut state = STATE.lock();

        if !qualifies(code, state.modifiers) {
            return;
        }

        let mut mods = Vec::new();

        if state.modifiers.control_key() {
            mods.push("Ctrl".to_string());
        }
        if state.modifiers.alt_key() {
            mods.push("Alt".to_string());
        }
        if state.modifiers.shift_key() {
            mods.push("Shift".to_string());
        }
        if state.modifiers.super_key() {
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
fn qualifies(code: KeyCode, modifiers: ModifiersState) -> bool {
    if is_modifier(code) {
        return false;
    }

    if modifiers.control_key() || modifiers.alt_key() || modifiers.super_key() {
        return true;
    }

    is_navigation(code)
}

fn is_modifier(code: KeyCode) -> bool {
    matches!(
        code,
        KeyCode::ControlLeft
            | KeyCode::ControlRight
            | KeyCode::ShiftLeft
            | KeyCode::ShiftRight
            | KeyCode::AltLeft
            | KeyCode::AltRight
            | KeyCode::SuperLeft
            | KeyCode::SuperRight
    )
}

fn is_navigation(code: KeyCode) -> bool {
    matches!(
        code,
        KeyCode::Escape
            | KeyCode::Tab
            | KeyCode::Enter
            | KeyCode::NumpadEnter
            | KeyCode::ArrowUp
            | KeyCode::ArrowDown
            | KeyCode::ArrowLeft
            | KeyCode::ArrowRight
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::PageUp
            | KeyCode::PageDown
            | KeyCode::F1
            | KeyCode::F2
            | KeyCode::F3
            | KeyCode::F4
            | KeyCode::F5
            | KeyCode::F6
            | KeyCode::F7
            | KeyCode::F8
            | KeyCode::F9
            | KeyCode::F10
            | KeyCode::F11
            | KeyCode::F12
    )
}

#[cfg(test)]
mod tests {
    use winit::keyboard::{KeyCode, ModifiersState};

    use super::qualifies;

    #[test]
    fn plain_typing_is_dropped() {
        assert!(!qualifies(KeyCode::KeyA, ModifiersState::empty()));
        assert!(!qualifies(KeyCode::Space, ModifiersState::empty()));
        assert!(!qualifies(KeyCode::Digit5, ModifiersState::empty()));
    }

    #[test]
    fn shift_letter_is_capital_typing_not_a_hotkey() {
        assert!(!qualifies(KeyCode::KeyA, ModifiersState::SHIFT));
    }

    #[test]
    fn modifier_combos_qualify() {
        assert!(qualifies(KeyCode::KeyC, ModifiersState::CONTROL));
        assert!(qualifies(
            KeyCode::KeyR,
            ModifiersState::SUPER | ModifiersState::SHIFT
        ));
        assert!(qualifies(KeyCode::KeyF, ModifiersState::ALT));
    }

    #[test]
    fn navigation_keys_qualify_bare() {
        assert!(qualifies(KeyCode::Escape, ModifiersState::empty()));
        assert!(qualifies(KeyCode::Enter, ModifiersState::empty()));
        assert!(qualifies(KeyCode::ArrowLeft, ModifiersState::empty()));
        assert!(qualifies(KeyCode::F5, ModifiersState::empty()));
    }

    #[test]
    fn modifier_only_presses_are_dropped() {
        assert!(!qualifies(KeyCode::ShiftLeft, ModifiersState::SHIFT));
        assert!(!qualifies(KeyCode::SuperLeft, ModifiersState::SUPER));
        assert!(!qualifies(KeyCode::ControlLeft, ModifiersState::CONTROL));
    }
}
