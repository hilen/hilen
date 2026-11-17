use super::{DictationEvent, Transcript};
use crate::{deps::refs::Weak, ui::TextField};

/// Puts what a dictation hears into a text field at the caret it had at
/// the start, and swaps it for each newer guess.
pub struct DictationField {
    field:      Weak<TextField>,
    start:      usize,
    len:        usize,
    space:      bool,
    transcript: Transcript,
}

impl DictationField {
    pub fn new(field: Weak<TextField>) -> Self {
        let start = field.caret();
        let space = field.text()[..start].chars().next_back().is_some_and(|ch| !ch.is_whitespace());
        Self {
            field,
            start,
            len: 0,
            space,
            transcript: Transcript::default(),
        }
    }

    /// Takes the event in and puts the new text into the field. A field that
    /// is gone takes nothing.
    pub fn apply(&mut self, event: &DictationEvent) {
        if self.field.is_null() || !self.transcript.apply(event) {
            return;
        }
        let words = self.transcript.text();
        let text = if self.space && !words.is_empty() {
            format!(" {words}")
        } else {
            words
        };
        self.field.replace_range(self.start..self.start + self.len, &text);
        self.len = text.len();
    }

    /// The dictated text, with no space in front.
    pub fn text(&self) -> String {
        self.transcript.text()
    }
}
