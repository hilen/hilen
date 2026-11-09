#[cfg(feature = "gpu")]
use crate::deps::vents::Event;
#[cfg(feature = "gpu")]
use crate::ui::{KeymapKey, touch_stack::TouchStack};
use crate::{deps::refs::Weak, ui::KeyCombo};

pub struct KeyAction {
    pub combo:  KeyCombo,
    #[cfg(feature = "gpu")]
    action:     Event,
    #[cfg(feature = "gpu")]
    subscriber: Weak,
}

impl KeyAction {
    pub fn new<T: ?Sized>(
        subscriber: Weak<T>,
        combo: impl Into<KeyCombo>,
        action: impl FnMut() + Send + 'static,
    ) -> Self {
        #[cfg(feature = "gpu")]
        {
            let event = Event::default();
            event.sub(action);
            Self {
                subscriber: subscriber.erase(),
                combo:      combo.into(),
                action:     event,
            }
        }
        // A build with no window gets no keys, nothing would run the action.
        #[cfg(not(feature = "gpu"))]
        {
            drop((subscriber, action));
            Self { combo: combo.into() }
        }
    }
}

impl KeyAction {
    #[cfg(feature = "gpu")]
    pub(crate) fn is_alive(&self) -> bool {
        !self.subscriber.is_null()
    }

    /// Runs the action when the key is its key and reaches its subscriber,
    /// and says whether it ran.
    #[cfg(feature = "gpu")]
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
