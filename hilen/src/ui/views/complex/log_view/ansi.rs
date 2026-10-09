//! ANSI escape codes of a log line. The color and style codes become
//! spans over the plain text, every other escape code is dropped.

use std::{iter::Peekable, ops::Range, str::Chars};

const ESCAPE: char = '\x1b';
const BELL: char = '\x07';

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AnsiColor {
    /// 0 to 15 are the colors of the style, 16 to 255 the fixed table
    /// every terminal has.
    Palette(u8),
    Rgb(u8, u8, u8),
}

/// The styles of a part of a line that are on, any mix of the 4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct Marks(u8);

impl Marks {
    pub const BOLD: Self = Self(1);
    pub const DIM: Self = Self(2);
    pub const STRIKE: Self = Self(8);
    pub const UNDERLINE: Self = Self(4);

    pub fn has(self, mark: Self) -> bool {
        self.0 & mark.0 != 0
    }

    fn set(&mut self, mark: Self, on: bool) {
        if on {
            self.0 |= mark.0;
        } else {
            self.0 &= !mark.0;
        }
    }
}

/// How a part of a line is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct Look {
    /// None is the color the line has without codes.
    pub color: Option<AnsiColor>,
    pub marks: Marks,
}

/// A line with its escape codes taken out.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Styled {
    pub text:  String,
    /// Byte ranges of `text` that do not have the plain look, in order.
    pub spans: Vec<(Range<usize>, Look)>,
}

impl Styled {
    fn close(&mut self, start: usize, look: Look) {
        if look != Look::default() && start < self.text.len() {
            self.spans.push((start..self.text.len(), look));
        }
    }
}

/// Reads a line. The look starts plain, a line does not go on with the
/// look of the line before it, the view shows lines in any order.
pub(super) fn parse(raw: &str) -> Styled {
    let mut styled = Styled::default();
    if !raw.contains([ESCAPE, '\r']) {
        styled.text.push_str(raw);
        return styled;
    }

    let mut look = Look::default();
    let mut start = 0;
    let mut chars = raw.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            ESCAPE => {
                let Some(codes) = escape(&mut chars) else {
                    continue;
                };
                let mut next = look;
                apply(&mut next, &codes);
                if next != look {
                    styled.close(start, look);
                    start = styled.text.len();
                    look = next;
                }
            }
            '\r' => {}
            _ => styled.text.push(ch),
        }
    }
    styled.close(start, look);
    styled
}

/// Reads the rest of an escape code, `chars` stands after the escape
/// char. Gives the numbers of a color and style code, none for any other
/// code.
fn escape(chars: &mut Peekable<Chars>) -> Option<Vec<u16>> {
    match chars.next()? {
        '[' => {
            let mut params = String::new();
            // Parameter and intermediate bytes, then 1 final byte.
            while let Some(ch) = chars.next_if(|ch| (' '..='?').contains(ch)) {
                params.push(ch);
            }
            (chars.next()? == 'm').then(|| numbers(&params))
        }
        // A code with a text in it, a window title or a link. It ends
        // with a bell or with escape and a backslash.
        ']' => {
            while let Some(ch) = chars.next() {
                if ch == BELL || (ch == ESCAPE && chars.next_if_eq(&'\\').is_some()) {
                    break;
                }
            }
            None
        }
        _ => None,
    }
}

fn numbers(params: &str) -> Vec<u16> {
    params.split([';', ':']).map(|part| part.parse().unwrap_or(0)).collect()
}

fn apply(look: &mut Look, codes: &[u16]) {
    let mut codes = codes.iter().copied();

    while let Some(code) = codes.next() {
        match code {
            0 => *look = Look::default(),
            1 => look.marks.set(Marks::BOLD, true),
            2 => look.marks.set(Marks::DIM, true),
            22 => {
                look.marks.set(Marks::BOLD, false);
                look.marks.set(Marks::DIM, false);
            }
            4 => look.marks.set(Marks::UNDERLINE, true),
            24 => look.marks.set(Marks::UNDERLINE, false),
            9 => look.marks.set(Marks::STRIKE, true),
            29 => look.marks.set(Marks::STRIKE, false),
            30..=37 => look.color = Some(AnsiColor::Palette(low(code - 30))),
            90..=97 => look.color = Some(AnsiColor::Palette(low(code - 90 + 8))),
            39 => look.color = None,
            38 => {
                if let Some(color) = long_color(&mut codes) {
                    look.color = Some(color);
                }
            }
            // A background color is not drawn, its numbers still have to
            // be read so they are not taken for other codes.
            48 => {
                long_color(&mut codes);
            }
            _ => {}
        }
    }
}

/// The numbers after 38 or 48: `5, n` for the table of 256 colors and
/// `2, r, g, b` for any color.
fn long_color(codes: &mut impl Iterator<Item = u16>) -> Option<AnsiColor> {
    match codes.next()? {
        5 => Some(AnsiColor::Palette(low(codes.next()?))),
        2 => {
            let red = low(codes.next()?);
            let green = low(codes.next()?);
            let blue = low(codes.next()?);
            Some(AnsiColor::Rgb(red, green, blue))
        }
        _ => None,
    }
}

fn low(value: u16) -> u8 {
    u8::try_from(value).unwrap_or(u8::MAX)
}

#[cfg(test)]
mod tests {
    use super::{AnsiColor, Look, Marks, Styled, parse};

    fn color(index: u8) -> Look {
        Look {
            color: Some(AnsiColor::Palette(index)),
            marks: Marks::default(),
        }
    }

    fn marked(mut look: Look, marks: &[Marks]) -> Look {
        for mark in marks {
            look.marks.set(*mark, true);
        }
        look
    }

    #[test]
    fn a_line_with_no_codes_stays_as_it_is() {
        assert_eq!(
            parse("plain text"),
            Styled {
                text:  "plain text".to_string(),
                spans: vec![],
            }
        );
    }

    #[test]
    fn a_color_holds_until_the_reset() {
        let styled = parse("ok \x1b[31mfailed\x1b[0m done");
        assert_eq!(styled.text, "ok failed done");
        assert_eq!(styled.spans, vec![(3..9, color(1))]);
    }

    #[test]
    fn a_color_with_no_reset_holds_to_the_end() {
        let styled = parse("\x1b[92mall green");
        assert_eq!(styled.text, "all green");
        assert_eq!(styled.spans, vec![(0..9, color(10))]);
    }

    #[test]
    fn several_codes_in_one_escape() {
        let styled = parse("\x1b[1;4;33mwarn\x1b[22;24m rest\x1b[39m.");
        assert_eq!(styled.text, "warn rest.");
        assert_eq!(
            styled.spans,
            vec![
                (0..4, marked(color(3), &[Marks::BOLD, Marks::UNDERLINE])),
                (4..9, color(3)),
            ]
        );
    }

    #[test]
    fn the_long_colors() {
        let styled = parse("\x1b[38;5;208ma\x1b[38;2;1;2;3mb\x1b[48;5;17;2mc");
        assert_eq!(styled.text, "abc");
        assert_eq!(
            styled.spans,
            vec![
                (0..1, color(208)),
                (
                    1..2,
                    Look {
                        color: Some(AnsiColor::Rgb(1, 2, 3)),
                        ..Look::default()
                    }
                ),
                (
                    2..3,
                    marked(
                        Look {
                            color: Some(AnsiColor::Rgb(1, 2, 3)),
                            ..Look::default()
                        },
                        &[Marks::DIM],
                    ),
                ),
            ]
        );
    }

    #[test]
    fn an_empty_code_is_a_reset() {
        let styled = parse("\x1b[9mgone\x1b[m here");
        assert_eq!(styled.text, "gone here");
        assert_eq!(
            styled.spans,
            vec![(0..4, marked(Look::default(), &[Marks::STRIKE]))]
        );
    }

    #[test]
    fn other_escape_codes_are_dropped() {
        let styled = parse("a\x1b[2Kb\x1b[10;5Hc\x1b]0;title\x07d\x1b]8;;http://x\x1b\\e\x1b=f\r");
        assert_eq!(styled.text, "abcdef");
        assert_eq!(styled.spans, vec![]);
    }

    #[test]
    fn spans_are_bytes_of_the_plain_text() {
        let styled = parse("žalia \x1b[32mšviesa\x1b[0m");
        assert_eq!(styled.text, "žalia šviesa");
        assert_eq!(styled.spans, vec![(7..14, color(2))]);
        assert_eq!(&styled.text[7..14], "šviesa");
    }

    #[test]
    fn a_cut_escape_at_the_end_shows_nothing() {
        assert_eq!(parse("text\x1b[3").text, "text");
        assert_eq!(parse("text\x1b").text, "text");
    }
}
