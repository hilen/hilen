use crate::{ui::KeymapKey, window::NamedKey};

/// A key with the modifiers that must be held for it to fire.
/// `cmd` is the command modifier: Cmd on a Mac, Ctrl everywhere else.
/// A plain key never fires while the command modifier is held, so a
/// `'b'` binding and a `KeyCombo::cmd('b')` binding stay distinct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyCombo {
    pub key:   KeymapKey,
    pub cmd:   bool,
    pub shift: bool,
}

impl KeyCombo {
    pub fn cmd(key: impl Into<KeymapKey>) -> Self {
        Self {
            key:   key.into(),
            cmd:   true,
            shift: false,
        }
    }

    pub fn cmd_shift(key: impl Into<KeymapKey>) -> Self {
        Self {
            key:   key.into(),
            cmd:   true,
            shift: true,
        }
    }

    pub(crate) fn matches(&self, key: KeymapKey, cmd_held: bool, shift_held: bool) -> bool {
        if self.key != key {
            return false;
        }
        if self.cmd != cmd_held {
            return false;
        }
        // Shift changes what character arrives, so only a command combo
        // tells Cmd+B from Cmd+Shift+B. A plain binding takes the shifted
        // character as its key instead.
        !self.cmd || self.shift == shift_held
    }
}

impl From<KeymapKey> for KeyCombo {
    fn from(key: KeymapKey) -> Self {
        Self {
            key,
            cmd: false,
            shift: false,
        }
    }
}

impl From<char> for KeyCombo {
    fn from(key: char) -> Self {
        KeymapKey::from(key).into()
    }
}

impl From<NamedKey> for KeyCombo {
    fn from(key: NamedKey) -> Self {
        KeymapKey::from(key).into()
    }
}
