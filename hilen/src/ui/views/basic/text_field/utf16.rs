//! A browser counts places in a text in UTF-16 units, the engine in bytes.

/// The UTF-16 place of the byte `byte`. A byte inside a char counts as the
/// start of that char.
#[cfg_attr(test, allow(dead_code))]
pub(super) fn units_of_byte(text: &str, byte: usize) -> u32 {
    let byte = text.floor_char_boundary(byte);
    u32::try_from(text[..byte].encode_utf16().count()).unwrap_or(u32::MAX)
}

/// The byte of the UTF-16 place `units`. A place inside a pair counts as the
/// start of its char, a place past the text as its end.
pub(super) fn byte_of_units(text: &str, units: u32) -> usize {
    let mut counted = 0;
    for (byte, ch) in text.char_indices() {
        let next = counted + u32::try_from(ch.len_utf16()).unwrap_or(1);
        if next > units {
            return byte;
        }
        counted = next;
    }
    text.len()
}

#[cfg(test)]
mod tests {
    use super::{byte_of_units, units_of_byte};

    #[test]
    fn plain_text_counts_the_same() {
        assert_eq!(units_of_byte("abc", 2), 2);
        assert_eq!(byte_of_units("abc", 2), 2);
    }

    #[test]
    fn a_letter_of_2_bytes_is_1_unit() {
        assert_eq!(units_of_byte("žab", 2), 1);
        assert_eq!(byte_of_units("žab", 1), 2);
    }

    #[test]
    fn japanese_is_3_bytes_and_1_unit() {
        assert_eq!(units_of_byte("日本語", 6), 2);
        assert_eq!(byte_of_units("日本語", 2), 6);
    }

    #[test]
    fn an_emoji_is_4_bytes_and_2_units() {
        assert_eq!(units_of_byte("a😀b", 5), 3);
        assert_eq!(byte_of_units("a😀b", 3), 5);
    }

    #[test]
    fn a_place_inside_a_char_goes_to_its_start() {
        assert_eq!(units_of_byte("a😀b", 3), 1);
        assert_eq!(byte_of_units("a😀b", 2), 1);
    }

    #[test]
    fn a_place_past_the_end_is_the_end() {
        assert_eq!(units_of_byte("ab", 9), 2);
        assert_eq!(byte_of_units("ab", 9), 2);
    }
}
