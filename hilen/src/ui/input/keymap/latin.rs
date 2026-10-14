use winit::keyboard::KeyCode;

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
pub(crate) fn latin_candidate(typed: char, code: Option<KeyCode>, system: Option<char>) -> Option<char> {
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

fn digit(code: KeyCode) -> Option<char> {
    Some(match code {
        KeyCode::Digit0 => '0',
        KeyCode::Digit1 => '1',
        KeyCode::Digit2 => '2',
        KeyCode::Digit3 => '3',
        KeyCode::Digit4 => '4',
        KeyCode::Digit5 => '5',
        KeyCode::Digit6 => '6',
        KeyCode::Digit7 => '7',
        KeyCode::Digit8 => '8',
        KeyCode::Digit9 => '9',
        _ => return None,
    })
}

/// The character of the key on a US layout. The physical key codes are
/// named after it.
fn us_char(code: KeyCode) -> Option<char> {
    Some(match code {
        KeyCode::KeyA => 'a',
        KeyCode::KeyB => 'b',
        KeyCode::KeyC => 'c',
        KeyCode::KeyD => 'd',
        KeyCode::KeyE => 'e',
        KeyCode::KeyF => 'f',
        KeyCode::KeyG => 'g',
        KeyCode::KeyH => 'h',
        KeyCode::KeyI => 'i',
        KeyCode::KeyJ => 'j',
        KeyCode::KeyK => 'k',
        KeyCode::KeyL => 'l',
        KeyCode::KeyM => 'm',
        KeyCode::KeyN => 'n',
        KeyCode::KeyO => 'o',
        KeyCode::KeyP => 'p',
        KeyCode::KeyQ => 'q',
        KeyCode::KeyR => 'r',
        KeyCode::KeyS => 's',
        KeyCode::KeyT => 't',
        KeyCode::KeyU => 'u',
        KeyCode::KeyV => 'v',
        KeyCode::KeyW => 'w',
        KeyCode::KeyX => 'x',
        KeyCode::KeyY => 'y',
        KeyCode::KeyZ => 'z',
        KeyCode::Minus => '-',
        KeyCode::Equal => '=',
        KeyCode::BracketLeft => '[',
        KeyCode::BracketRight => ']',
        KeyCode::Backslash => '\\',
        KeyCode::Semicolon => ';',
        KeyCode::Quote => '\'',
        KeyCode::Backquote => '`',
        KeyCode::Comma => ',',
        KeyCode::Period => '.',
        KeyCode::Slash => '/',
        _ => return None,
    })
}

#[cfg(test)]
mod test {
    use winit::keyboard::KeyCode;

    use super::latin_candidate;

    #[test]
    fn a_non_latin_letter_gets_the_us_letter_of_its_key() {
        assert_eq!(latin_candidate('и', Some(KeyCode::KeyB), None), Some('b'));
        assert_eq!(latin_candidate('β', Some(KeyCode::KeyB), None), Some('b'));
        assert_eq!(latin_candidate('נ', Some(KeyCode::KeyB), None), Some('b'));
        assert_eq!(latin_candidate('ю', Some(KeyCode::Period), None), Some('.'));
    }

    #[test]
    fn the_system_letter_wins_over_the_us_place() {
        // Russian over German: the key in the US Y place is labeled Z.
        assert_eq!(latin_candidate('н', Some(KeyCode::KeyY), Some('z')), Some('z'));
    }

    #[test]
    fn a_system_answer_that_is_not_ascii_is_ignored() {
        assert_eq!(latin_candidate('и', Some(KeyCode::KeyB), Some('ü')), Some('b'));
    }

    #[test]
    fn the_latin_letter_follows_the_case_of_the_typed_one() {
        assert_eq!(latin_candidate('Я', Some(KeyCode::KeyZ), None), Some('Z'));
    }

    #[test]
    fn a_latin_layout_keeps_its_own_letters() {
        // AZERTY, the key in the US Q place types a.
        assert_eq!(latin_candidate('a', Some(KeyCode::KeyQ), Some('q')), None);
        assert_eq!(latin_candidate('b', Some(KeyCode::KeyB), None), None);
    }

    #[test]
    fn the_digit_row_always_offers_its_digit() {
        assert_eq!(latin_candidate('&', Some(KeyCode::Digit1), None), Some('1'));
        assert_eq!(latin_candidate('ą', Some(KeyCode::Digit1), None), Some('1'));
        assert_eq!(latin_candidate('1', Some(KeyCode::Digit1), None), None);
    }

    #[test]
    fn a_key_with_no_known_place_gets_nothing() {
        assert_eq!(latin_candidate('и', None, None), None);
        assert_eq!(latin_candidate('и', Some(KeyCode::IntlBackslash), None), None);
    }
}
