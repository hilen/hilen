use std::cell::RefCell;

use crate::{
    deps::refs::Weak,
    ui::{KeyAction, KeymapKey},
};

#[derive(Default)]
pub struct Keymap {
    keys: RefCell<Vec<KeyAction>>,
}

impl Keymap {
    pub fn add<T: ?Sized>(
        &self,
        subscriber: Weak<T>,
        key: impl Into<KeymapKey>,
        action: impl FnMut() + Send + 'static,
    ) {
        self.keys.borrow_mut().push(KeyAction::new(subscriber, key, action));
    }

    pub(crate) fn check(&self, key: impl Into<KeymapKey>) {
        let key = key.into();
        self.keys.borrow_mut().retain(|a| a.check(key));
    }
}
