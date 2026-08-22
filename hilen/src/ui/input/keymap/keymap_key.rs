use crate::window::NamedKey;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeymapKey {
    Char(char),
    Named(NamedKey),
}

impl From<char> for KeymapKey {
    fn from(key: char) -> Self {
        Self::Char(key)
    }
}

impl From<NamedKey> for KeymapKey {
    fn from(key: NamedKey) -> Self {
        Self::Named(key)
    }
}
