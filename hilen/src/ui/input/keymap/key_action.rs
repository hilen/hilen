use crate::{
    deps::{refs::Weak, vents::Event},
    ui::{KeyCombo, KeymapKey},
};

pub struct KeyAction {
    pub combo:  KeyCombo,
    action:     Event,
    subscriber: Weak,
}

impl KeyAction {
    pub fn new<T: ?Sized>(
        subscriber: Weak<T>,
        combo: impl Into<KeyCombo>,
        action: impl FnMut() + Send + 'static,
    ) -> Self {
        let event = Event::default();
        event.sub(action);
        Self {
            subscriber: subscriber.erase(),
            combo:      combo.into(),
            action:     event,
        }
    }
}

impl KeyAction {
    pub(crate) fn check(&self, key: KeymapKey, cmd_held: bool, shift_held: bool) -> bool {
        if self.subscriber.is_null() {
            return false;
        }
        if self.combo.matches(key, cmd_held, shift_held) {
            self.action.trigger(());
        }
        true
    }
}
