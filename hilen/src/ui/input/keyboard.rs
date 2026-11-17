//! The key and mouse types of the engine. The keys come from
//! keyboard-types, the W3C key list that winit uses too, so the views
//! build with no window crate. The window code maps winit to these.

pub use keyboard_types::{Code, Modifiers, NamedKey};
use serde::{Deserialize, Serialize};

/// A button of a mouse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}

/// Whether a key or a button went down or up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ElementState {
    Pressed,
    Released,
}

impl ElementState {
    pub fn is_pressed(self) -> bool {
        self == Self::Pressed
    }
}

/// The character a named key also types, the way winit gives it with the
/// key: Enter as `\r`, Backspace as 0x08, Tab and Escape. For keys that do
/// not come from a keyboard, like the ones the inspector injects.
#[cfg(feature = "inspect")]
pub(crate) fn named_key_char(key: NamedKey) -> Option<char> {
    match key {
        NamedKey::Enter => Some('\r'),
        NamedKey::Backspace => Some('\x08'),
        NamedKey::Tab => Some('\t'),
        NamedKey::Escape => Some('\x1b'),
        _ => None,
    }
}
