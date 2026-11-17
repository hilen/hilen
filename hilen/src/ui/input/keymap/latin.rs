use crate::ui::Code;

/// The Latin character a key binding is also checked against, after the
/// character the layout typed.
///
/// A layout like Cyrillic, Greek or Hebrew types no Latin letters, so a
/// `KeyCombo::cmd('b')` binding would never fire on it. `system` is the
/// Latin letter the system gives the physical key, see `latin_key` in the
/// winit fork, and with no answer the US place of the key decides. A Latin
/// layout like AZERTY keeps its own letters, the way Qt and Windows match a
/// shortcut. The digit row is the exception: AZERTY types `&` and Lithuanian
/// types `ą` on the 1 key, so its digit is always a candidate.
pub(crate) fn latin_candidate(typed: char, code: Option<Code>, system: Option<char>) -> Option<char> {
    let latin = if let Some(digit) = code.and_then(digit) {
        digit
    } else if typed.is_ascii() {
        return None;
    } else {
        let latin = system.filter(char::is_ascii_graphic).or_else(|| code.and_then(us_char))?;
        // Shift or Caps Lock made the typed letter upper case, a binding
        // like `'Z'` asks for the same of the Latin one.
        if typed.is_uppercase() {
            latin.to_ascii_uppercase()
        } else {
            latin
        }
    };
    (latin != typed).then_some(latin)
}

fn digit(code: Code) -> Option<char> {
    Some(match code {
        Code::Digit0 => '0',
        Code::Digit1 => '1',
        Code::Digit2 => '2',
        Code::Digit3 => '3',
        Code::Digit4 => '4',
        Code::Digit5 => '5',
        Code::Digit6 => '6',
        Code::Digit7 => '7',
        Code::Digit8 => '8',
        Code::Digit9 => '9',
        _ => return None,
    })
}

/// The character of the key on a US layout. The physical key codes are
/// named after it.
fn us_char(code: Code) -> Option<char> {
    Some(match code {
        Code::KeyA => 'a',
        Code::KeyB => 'b',
        Code::KeyC => 'c',
        Code::KeyD => 'd',
        Code::KeyE => 'e',
        Code::KeyF => 'f',
        Code::KeyG => 'g',
        Code::KeyH => 'h',
        Code::KeyI => 'i',
        Code::KeyJ => 'j',
        Code::KeyK => 'k',
        Code::KeyL => 'l',
        Code::KeyM => 'm',
        Code::KeyN => 'n',
        Code::KeyO => 'o',
        Code::KeyP => 'p',
        Code::KeyQ => 'q',
        Code::KeyR => 'r',
        Code::KeyS => 's',
        Code::KeyT => 't',
        Code::KeyU => 'u',
        Code::KeyV => 'v',
        Code::KeyW => 'w',
        Code::KeyX => 'x',
        Code::KeyY => 'y',
        Code::KeyZ => 'z',
        Code::Minus => '-',
        Code::Equal => '=',
        Code::BracketLeft => '[',
        Code::BracketRight => ']',
        Code::Backslash => '\\',
        Code::Semicolon => ';',
        Code::Quote => '\'',
        Code::Backquote => '`',
        Code::Comma => ',',
        Code::Period => '.',
        Code::Slash => '/',
        _ => return None,
    })
}

#[cfg(test)]
mod test {
    use super::latin_candidate;
    use crate::ui::Code;

    #[test]
    fn a_non_latin_letter_gets_the_us_letter_of_its_key() {
        assert_eq!(latin_candidate('и', Some(Code::KeyB), None), Some('b'));
        assert_eq!(latin_candidate('β', Some(Code::KeyB), None), Some('b'));
        assert_eq!(latin_candidate('נ', Some(Code::KeyB), None), Some('b'));
        assert_eq!(latin_candidate('ю', Some(Code::Period), None), Some('.'));
    }

    #[test]
    fn the_system_letter_wins_over_the_us_place() {
        // Russian over German: the key in the US Y place is labeled Z.
        assert_eq!(latin_candidate('н', Some(Code::KeyY), Some('z')), Some('z'));
    }

    #[test]
    fn a_system_answer_that_is_not_ascii_is_ignored() {
        assert_eq!(latin_candidate('и', Some(Code::KeyB), Some('ü')), Some('b'));
    }

    #[test]
    fn the_latin_letter_follows_the_case_of_the_typed_one() {
        assert_eq!(latin_candidate('Я', Some(Code::KeyZ), None), Some('Z'));
    }

    #[test]
    fn a_latin_layout_keeps_its_own_letters() {
        // AZERTY, the key in the US Q place types a.
        assert_eq!(latin_candidate('a', Some(Code::KeyQ), Some('q')), None);
        assert_eq!(latin_candidate('b', Some(Code::KeyB), None), None);
    }

    #[test]
    fn the_digit_row_always_offers_its_digit() {
        assert_eq!(latin_candidate('&', Some(Code::Digit1), None), Some('1'));
        assert_eq!(latin_candidate('ą', Some(Code::Digit1), None), Some('1'));
        assert_eq!(latin_candidate('1', Some(Code::Digit1), None), None);
    }

    #[test]
    fn a_key_with_no_known_place_gets_nothing() {
        assert_eq!(latin_candidate('и', None, None), None);
        assert_eq!(latin_candidate('и', Some(Code::IntlBackslash), None), None);
    }
}
