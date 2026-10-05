use std::{cell::RefCell, mem::take};

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
        // An action may bind keys itself, a screen it swaps in binds its
        // own in `setup`, so the list is not borrowed while actions run.
        // A binding added by an action joins after the run, it never
        // sees the press that added it.
        let mut actions = take(&mut *self.keys.borrow_mut());
        actions.retain(|a| a.check(key, cmd, shift));
        actions.append(&mut self.keys.borrow_mut());
        *self.keys.borrow_mut() = actions;
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
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use winit::keyboard::{ModifiersState, NamedKey};

    use super::{Keymap, command_held};
    use crate::{
        deps::{hreads::set_current_thread_as_main, refs::Own},
        ui::KeymapKey,
    };

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

    // A screen swapped in by a key action binds its own keys in `setup`,
    // on the same keymap, while the press is still being dispatched. That
    // used to panic with the key list borrowed twice.
    #[test]
    fn an_action_can_bind_a_key_while_its_press_is_checked() {
        // A `Weak` is read on the main thread only, the test thread is it.
        set_current_thread_as_main();
        thread_local! {
            static KEYMAP: Keymap = Keymap::default();
        }
        // Any owned value stands in for a screen, an empty type has no
        // address for a `Weak` to point at.
        let screen = Own::new(1_u8);
        let next_screen = Own::new(2_u8);
        let fired = Arc::new(AtomicUsize::new(0));

        let next = next_screen.weak();
        let count = fired.clone();
        KEYMAP.with(|keymap| {
            keymap.add(screen.weak(), NamedKey::Escape, move || {
                let count = count.clone();
                KEYMAP.with(|keymap| {
                    keymap.add(next, NamedKey::Escape, move || {
                        count.fetch_add(1, Ordering::SeqCst);
                    });
                });
            });
        });

        KEYMAP.with(|keymap| keymap.check(NamedKey::Escape));
        assert_eq!(
            fired.load(Ordering::SeqCst),
            0,
            "a binding never sees the press that added it"
        );

        KEYMAP.with(|keymap| keymap.check(NamedKey::Escape));
        assert_eq!(
            fired.load(Ordering::SeqCst),
            1,
            "the added binding fires on the next press"
        );
    }
}
