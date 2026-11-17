//! What a double and a triple click take, for a text field and for
//! selectable text.

use std::ops::Range;

/// The run of letters and digits around `byte`, empty when `byte` is
/// between 2 other characters.
pub(crate) fn word_range(text: &str, byte: usize) -> Range<usize> {
    let byte = text.floor_char_boundary(byte.min(text.len()));
    let is_word = |ch: char| ch.is_alphanumeric();

    let start = text[..byte]
        .char_indices()
        .rev()
        .take_while(|(_, ch)| is_word(*ch))
        .last()
        .map_or(byte, |(index, _)| index);
    let end = text[byte..]
        .char_indices()
        .find(|(_, ch)| !is_word(*ch))
        .map_or(text.len(), |(index, _)| byte + index);

    start..end
}

/// The line around `byte`, from one line break of the text to the next.
/// A line that wraps on screen is still 1 line.
pub(crate) fn line_range(text: &str, byte: usize) -> Range<usize> {
    let byte = text.floor_char_boundary(byte.min(text.len()));

    let start = text[..byte].rfind('\n').map_or(0, |index| index + 1);
    let end = text[byte..].find('\n').map_or(text.len(), |index| byte + index);

    start..end
}

#[cfg(test)]
mod tests {
    use super::{line_range, word_range};

    #[test]
    fn a_word_is_taken_from_any_byte_of_it() {
        let text = "one, two2 three";
        assert_eq!(word_range(text, 0), 0..3);
        assert_eq!(word_range(text, 2), 0..3);
        assert_eq!(word_range(text, 3), 0..3);
        assert_eq!(word_range(text, 7), 5..9);
        assert_eq!(word_range(text, 15), 10..15);
        assert_eq!(word_range(text, 99), 10..15);
    }

    #[test]
    fn between_2_other_characters_there_is_no_word() {
        assert_eq!(word_range("a - b", 2), 2..2);
    }

    #[test]
    fn a_byte_inside_a_character_goes_to_its_start() {
        let text = "žodis du";
        assert_eq!(word_range(text, 1), 0..6);
    }

    #[test]
    fn a_line_ends_at_the_line_breaks() {
        let text = "first\nsecond line\nthird";
        assert_eq!(line_range(text, 0), 0..5);
        assert_eq!(line_range(text, 5), 0..5);
        assert_eq!(line_range(text, 6), 6..17);
        assert_eq!(line_range(text, 12), 6..17);
        assert_eq!(line_range(text, 99), 18..23);
    }
}
