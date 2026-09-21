use std::cell::RefCell;

use winit::keyboard::{ModifiersState, NamedKey};

use crate::{
    deps::refs::Weak,
    ui::{Input, KeyAction, KeyCombo, KeymapKey},
};

#[derive(Default)]
pub struct Keymap {
    keys: RefCell<Vec<KeyAction>>,
}

impl Keymap {
    pub fn add<T: ?Sized>(
        &self,
        subscriber: Weak<T>,
        combo: impl Into<KeyCombo>,
        action: impl FnMut() + Send + 'static,
    ) {
        self.keys.borrow_mut().push(KeyAction::new(subscriber, combo, action));
    }

    pub(crate) fn check(&self, key: impl Into<KeymapKey>) {
        let key = key.into();
        let modifiers = Input::modifiers();
        let cmd = command_held(key, modifiers);
        let shift = modifiers.shift_key();
        self.keys.borrow_mut().retain(|a| a.check(key, cmd, shift));
    }
}

/// Whether the command modifier is held besides the pressed key.
///
/// A press of Ctrl itself must not count as Ctrl held. Windows reports the new
/// modifier state before the key event and a Mac after it, so without this a
/// plain `NamedKey::Control` binding, the advance key of a human test run,
/// fires on a Mac and never on Windows.
fn command_held(key: KeymapKey, modifiers: ModifiersState) -> bool {
    let control = modifiers.control_key() && key != KeymapKey::Named(NamedKey::Control);
    let command = modifiers.super_key() && key != KeymapKey::Named(NamedKey::Super);
    control || command
}

#[cfg(test)]
mod test {
    use winit::keyboard::{ModifiersState, NamedKey};

    use super::command_held;
    use crate::ui::KeymapKey;

    #[test]
    fn a_modifier_press_does_not_hold_itself() {
        // What Windows hands over for a bare Ctrl press.
        assert!(!command_held(NamedKey::Control.into(), ModifiersState::CONTROL));
        assert!(!command_held(NamedKey::Super.into(), ModifiersState::SUPER));
        // What a Mac hands over for the same press.
        assert!(!command_held(NamedKey::Control.into(), ModifiersState::empty()));
    }

    #[test]
    fn a_held_modifier_counts_for_every_other_key() {
        assert!(command_held(KeymapKey::Char('b'), ModifiersState::CONTROL));
        assert!(command_held(KeymapKey::Char('b'), ModifiersState::SUPER));
        assert!(command_held(NamedKey::Enter.into(), ModifiersState::CONTROL));
        assert!(!command_held(KeymapKey::Char('b'), ModifiersState::SHIFT));
    }

    #[test]
    fn the_other_modifier_still_counts() {
        // Cmd held while Ctrl is pressed is a real combo.
        assert!(command_held(
            NamedKey::Control.into(),
            ModifiersState::CONTROL | ModifiersState::SUPER
        ));
    }
}
