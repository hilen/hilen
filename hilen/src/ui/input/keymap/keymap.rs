use std::cell::RefCell;

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
        let cmd = modifiers.super_key() || modifiers.control_key();
        let shift = modifiers.shift_key();
        self.keys.borrow_mut().retain(|a| a.check(key, cmd, shift));
    }
}
