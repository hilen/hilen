use parking_lot::Mutex;

use crate::{
    ui::{Touch, TouchEvent},
    window::MouseButton,
};

/// Every button or finger down right now, a finger counts as the left
/// button. Keyed by touch id too, so lifting one of two fingers keeps
/// the button held.
static HELD: Mutex<Vec<(MouseButton, usize)>> = Mutex::new(Vec::new());

/// The mouse buttons held down right now. The touch events fire once per
/// press and release and go to the view under them, which is right for a
/// button and wrong for a game, where a held button attacks every frame
/// wherever the cursor is. The state is raw, a press on a view counts too.
pub struct Mouse;

impl Mouse {
    pub fn held(button: MouseButton) -> bool {
        HELD.lock().iter().any(|(held, _)| *held == button)
    }

    pub(crate) fn on_touch(touch: &Touch) {
        let mut held = HELD.lock();
        match touch.event {
            TouchEvent::Began => {
                held.retain(|entry| *entry != (touch.button, touch.id));
                held.push((touch.button, touch.id));
            }
            TouchEvent::Ended => held.retain(|entry| *entry != (touch.button, touch.id)),
            TouchEvent::Moved => (),
        }
    }

    pub(crate) fn clear() {
        HELD.lock().clear();
    }
}
