use super::Label;
use crate::{deps::refs::weak_from_ref, window::BoxChars};

impl Label {
    /// Draws every box char of the text, U+2500 to U+257F, over its whole
    /// cell the way a terminal does: from the top of its line to the top
    /// of the next one, and from its left edge to the next char. The lines
    /// of a table drawn with box chars so join above, below, left and
    /// right with no gap, at any text size and line height. The box chars
    /// of a font with no `─` or no `│` stay as they are. Off by default.
    pub fn set_joins_box_chars(&self, joins: bool) -> &Self {
        let mut this = weak_from_ref(self);
        this.box_chars = if joins { BoxChars::Fitted } else { BoxChars::Glyphs };
        self
    }

    pub(crate) fn box_chars(&self) -> BoxChars {
        self.box_chars
    }
}
