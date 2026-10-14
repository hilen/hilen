use crate::{
    deps::{refs::Weak, vents::Event},
    ui::{KeyCombo, KeymapKey, touch_stack::TouchStack},
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
    pub(crate) fn is_alive(&self) -> bool {
        !self.subscriber.is_null()
    }

    /// Runs the action when the key is its key and reaches its subscriber,
    /// and says whether it ran.
    pub(crate) fn check(&self, key: KeymapKey, cmd_held: bool, shift_held: bool) -> bool {
        if !self.is_alive()
            || !self.combo.matches(key, cmd_held, shift_held)
            || !TouchStack::key_reaches(self.subscriber.raw())
        {
            return false;
        }
        self.action.trigger(());
        true
    }
}
