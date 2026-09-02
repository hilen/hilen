use parking_lot::Mutex;
use winit::keyboard::KeyCode;

/// Physical keys, so `w` is the key left of `e` on every layout.
static HELD: Mutex<Vec<KeyCode>> = Mutex::new(Vec::new());

/// The keys held down right now. The key events fire once per press,
/// with the system's repeat, which is right for typing and wrong for
/// moving, where a held key has to move every frame.
pub struct Keys;

impl Keys {
    pub fn held(code: KeyCode) -> bool {
        HELD.lock().contains(&code)
    }

    pub(crate) fn set(code: KeyCode, pressed: bool) {
        let mut held = HELD.lock();
        held.retain(|held| *held != code);
        if pressed {
            held.push(code);
        }
    }

    pub(crate) fn clear() {
        HELD.lock().clear();
    }
}
