//! Snippets of a `CodeEditor`: a text with places to fill in, in the syntax
//! of LSP and `TextMate`, like `fn add(${1:rhs}) {\n    $0\n}`. The text goes
//! in with its marks taken out and each line after the first indented like
//! the line it goes into. The cursor goes to the first place, with its
//! default text selected, Tab and Shift Tab go to the next place and the one
//! before, and `$0` is the last place, where the snippet ends. The session
//! ends on Escape too, and when the cursor leaves the place it is at. A
//! place used twice, like the `T` of `impl From<${1:T}> for X { fn
//! from(v: ${1:T}) }`, is 1 place: what the user types in its first copy
//! goes into every copy. The rules are the ones of the Zed editor.

use std::{collections::BTreeMap, iter::Peekable, ops::Range, str::Chars};

use super::{
    edit::{LinesChanged, StepKind},
    pairs::moved,
    text::{CodePos, CodeText},
};

/// The text of a snippet and its places, each a list of byte ranges of the
/// text, in the order Tab visits them. The last one is where it ends.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Parsed {
    pub text:  String,
    pub stops: Vec<Vec<Range<usize>>>,
}

/// The places of a snippet in the text, see `Parsed`, and the one the
/// cursor is at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SnippetSession {
    stops:  Vec<Vec<(CodePos, CodePos)>>,
    active: usize,
}

struct Parser<'a> {
    chars:  Peekable<Chars<'a>>,
    indent: &'a str,
    text:   String,
    stops:  BTreeMap<u32, Vec<Range<usize>>>,
}

impl Parser<'_> {
    /// Reads up to a `}` that closes a place, or to the end with `nested`
    /// off.
    fn content(&mut self, nested: bool) {
        while let Some(ch) = self.chars.next() {
            match ch {
                '\\' => match self.chars.peek() {
                    Some(&next @ ('$' | '}' | '\\' | ',' | '|')) => {
                        self.chars.next();
                        self.text.push(next);
                    }
                    _ => self.text.push('\\'),
                },
                '}' if nested => return,
                '$' => self.dollar(),
                '\n' => {
                    self.text.push('\n');
                    self.text.push_str(self.indent);
                }
                ch => self.text.push(ch),
            }
        }
    }

    fn number(&mut self) -> Option<u32> {
        let mut digits = String::new();
        while let Some(ch) = self.chars.peek().filter(|ch| ch.is_ascii_digit()) {
            digits.push(*ch);
            self.chars.next();
        }
        digits.parse().ok()
    }

    fn name(&mut self) -> String {
        let mut name = String::new();
        while let Some(ch) = self.chars.peek().filter(|ch| ch.is_alphanumeric() || **ch == '_') {
            name.push(*ch);
            self.chars.next();
        }
        name
    }

    fn stop(&mut self, index: u32, range: Range<usize>) {
        self.stops.entry(index).or_default().push(range);
    }

    /// What follows a `$`: a place, a place with a default text or with
    /// choices, or a variable, which this editor knows no value of and
    /// puts in as its default text.
    fn dollar(&mut self) {
        let at = self.text.len();
        match self.chars.peek() {
            Some(ch) if ch.is_ascii_digit() => {
                if let Some(index) = self.number() {
                    self.stop(index, at..at);
                }
            }
            Some('{') => {
                self.chars.next();
                let Some(index) = self.number() else {
                    // A variable, its default text if it has one.
                    self.name();
                    if self.chars.next_if_eq(&':').is_some() {
                        self.content(true);
                    } else {
                        self.chars.next_if_eq(&'}');
                    }
                    return;
                };
                match self.chars.next() {
                    Some(':') => {
                        self.content(true);
                        let end = self.text.len();
                        self.stop(index, at..end);
                    }
                    Some('|') => {
                        // The first choice is the default text.
                        let mut choice = String::new();
                        let mut first = true;
                        while let Some(ch) = self.chars.next() {
                            match ch {
                                '|' if self.chars.next_if_eq(&'}').is_some() => break,
                                ',' => first = false,
                                '\\' => {
                                    if let Some(next) = self.chars.next()
                                        && first
                                    {
                                        choice.push(next);
                                    }
                                }
                                ch if first => choice.push(ch),
                                _ => {}
                            }
                        }
                        self.text.push_str(&choice);
                        let end = self.text.len();
                        self.stop(index, at..end);
                    }
                    _ => self.stop(index, at..at),
                }
            }
            Some(ch) if ch.is_alphabetic() || *ch == '_' => {
                self.name();
            }
            _ => self.text.push('$'),
        }
    }
}

/// Reads `snippet`. Each line after the first gets `indent` in front.
pub(super) fn parse(snippet: &str, indent: &str) -> Parsed {
    let mut parser = Parser {
        chars: snippet.chars().peekable(),
        indent,
        text: String::new(),
        stops: BTreeMap::new(),
    };
    parser.content(false);
    let end = parser.stops.remove(&0);
    let mut stops: Vec<Vec<Range<usize>>> = parser.stops.into_values().collect();
    let len = parser.text.len();
    stops.push(end.unwrap_or_else(|| {
        vec![Range {
            start: len,
            end:   len,
        }]
    }));
    Parsed {
        text: parser.text,
        stops,
    }
}

/// The place of byte `offset` of `text` put in at `from`.
fn pos_in(from: CodePos, text: &str, offset: usize) -> CodePos {
    let before = &text[..offset];
    match before.rfind('\n') {
        Some(at) => CodePos::new(from.line + before.matches('\n').count(), offset - at - 1),
        None => CodePos::new(from.line, from.byte + offset),
    }
}

impl CodeText {
    /// Puts `snippet` in place of the text from `from` to `to` as 1 step of
    /// the history, and goes to its first place.
    pub(crate) fn insert_snippet(&mut self, from: CodePos, to: CodePos, snippet: &str) -> LinesChanged {
        let (from, to) = (self.clamp(from.min(to)), self.clamp(from.max(to)));
        let line = &self.lines[from.line];
        let indent = &line[..line.len() - line.trim_start_matches([' ', '\t']).len()];
        let Parsed { text, stops } = parse(snippet, indent);
        let stops: Vec<Vec<(CodePos, CodePos)>> = stops
            .iter()
            .map(|ranges| {
                ranges
                    .iter()
                    .map(|range| (pos_in(from, &text, range.start), pos_in(from, &text, range.end)))
                    .collect()
            })
            .collect();
        // A place of a snippet before this one must not copy into it.
        self.snippet = None;
        let changed = self.replace_to(from, to, &text, StepKind::Other, None);
        self.snippet = Some(SnippetSession { stops, active: 0 });
        self.go_to_stop();
        changed
    }

    /// A change from `from` to `end` of the new text inside the first copy
    /// of the place the cursor is at, typed or put in by the app like a
    /// picked completion, puts the same text into the other copies of that
    /// place. They join the undo step of the change, see `link_lines`.
    pub(super) fn mirror_snippet(&mut self, from: CodePos, end: CodePos) {
        let Some(session) = &self.snippet else {
            return;
        };
        let copies = session.stops[session.active].len();
        let Some(&(start, place_end)) = session.stops[session.active].first() else {
            return;
        };
        if copies < 2 || start.line != place_end.line || from < start || end > place_end {
            return;
        }
        let text = self.lines[start.line][start.byte..place_end.byte].to_string();
        let mut before = BTreeMap::new();
        for copy in 1..copies {
            let Some(&(from, to)) = self
                .snippet
                .as_ref()
                .and_then(|session| session.stops[session.active].get(copy))
            else {
                continue;
            };
            if from.line != to.line || self.lines[from.line][from.byte..to.byte] == text {
                continue;
            }
            before.entry(from.line).or_insert_with(|| self.lines[from.line].clone());
            self.lines[from.line].replace_range(from.byte..to.byte, &text);
            let end = CodePos::new(from.line, from.byte + text.len());
            self.follow_regions(from, to, end);
            self.follow_snippet(from, to, end);
            self.cursor = moved(self.cursor, from, to, end, false);
            self.anchor = self.anchor.map(|anchor| moved(anchor, from, to, end, false));
        }
        if before.is_empty() {
            return;
        }
        self.link_lines(&before);
        self.mirrored.extend(before.into_keys());
    }

    /// `insert_snippet`, joined to the newest step of the history when
    /// the lines are still at `version`, see `replace_joined`.
    pub(crate) fn insert_snippet_joined(
        &mut self,
        version: u64,
        from: CodePos,
        to: CodePos,
        snippet: &str,
    ) -> LinesChanged {
        let joins = version == self.version;
        let changed = self.insert_snippet(from, to, snippet);
        if joins {
            self.join_newest();
        }
        changed
    }

    pub(crate) fn snippet_active(&self) -> bool {
        self.snippet.is_some()
    }

    /// Selects the place the session is at. The last place ends it.
    fn go_to_stop(&mut self) {
        let Some(session) = &self.snippet else {
            return;
        };
        let last = session.active + 1 >= session.stops.len();
        if let Some(&(start, end)) = session.stops[session.active].first() {
            if start == end {
                self.move_to(start, false);
            } else {
                self.select(start, end);
            }
        }
        if last {
            self.snippet = None;
        }
    }

    /// Tab in a snippet: the next place. False with no snippet.
    pub(crate) fn next_stop(&mut self) -> bool {
        let Some(session) = &mut self.snippet else {
            return false;
        };
        session.active += 1;
        self.go_to_stop();
        true
    }

    /// Shift Tab in a snippet: the place before. False with no snippet or
    /// at its first place.
    pub(crate) fn previous_stop(&mut self) -> bool {
        let Some(session) = self.snippet.as_mut().filter(|session| session.active > 0) else {
            return false;
        };
        session.active -= 1;
        self.go_to_stop();
        true
    }

    pub(crate) fn end_snippet(&mut self) {
        self.snippet = None;
    }

    /// The session ends when the cursor left the place it is at.
    pub(crate) fn drop_far_snippet(&mut self) {
        let cursor = self.cursor;
        let inside = self.snippet.as_ref().is_some_and(|session| {
            session.stops[session.active]
                .first()
                .is_some_and(|&(start, end)| start <= cursor && cursor <= end)
        });
        if !inside {
            self.snippet = None;
        }
    }

    /// The places follow a change from `from` to `to` that put in a text
    /// ending at `end`, like the regions of the pairs.
    pub(super) fn follow_snippet(&mut self, from: CodePos, to: CodePos, end: CodePos) {
        let Some(session) = &mut self.snippet else {
            return;
        };
        for range in session.stops.iter_mut().flatten() {
            range.0 = super::pairs::moved(range.0, from, to, end, false);
            range.1 = super::pairs::moved(range.1, from, to, end, true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stops(parsed: &Parsed) -> Vec<Vec<&str>> {
        parsed
            .stops
            .iter()
            .map(|ranges| ranges.iter().map(|range| &parsed.text[range.clone()]).collect())
            .collect()
    }

    #[test]
    fn the_marks_go_and_the_places_stay() {
        let parsed = parse("fn add(${1:rhs}: ${2:Self}) {\n    $0\n}", "    ");
        assert_eq!(parsed.text, "fn add(rhs: Self) {\n        \n    }");
        assert_eq!(stops(&parsed), [vec!["rhs"], vec!["Self"], vec![""]]);
        assert_eq!(parsed.stops[2], vec![28..28]);
    }

    #[test]
    fn a_snippet_with_no_end_ends_after_its_text() {
        let parsed = parse("println!($1)", "");
        assert_eq!(parsed.text, "println!()");
        let at = |byte| {
            vec![Range {
                start: byte,
                end:   byte,
            }]
        };
        assert_eq!(parsed.stops, [at(9), at(10)]);
    }

    #[test]
    fn escapes_choices_variables_and_nested_places() {
        let parsed = parse(r"a\$b ${1|one,two|} ${TM_SELECTED_TEXT:x} ${2:f(${3:y})} $$", "");
        assert_eq!(parsed.text, "a$b one x f(y) $$");
        assert_eq!(stops(&parsed), [vec!["one"], vec!["f(y)"], vec!["y"], vec![""]]);
        // A place used twice keeps both.
        let twice = parse("${1:a} = ${1:a}", "");
        assert_eq!(stops(&twice), [vec!["a", "a"], vec![""]]);
    }

    fn text(lines: &str) -> CodeText {
        let mut text = CodeText::default();
        text.set_text(lines);
        text
    }

    #[test]
    fn tab_goes_from_place_to_place_and_the_end_ends_the_session() {
        let mut code = text("    fn");
        code.insert_snippet(
            CodePos::new(0, 4),
            CodePos::new(0, 6),
            "fn add(${1:rhs}) {\n    $0\n}",
        );
        assert_eq!(code.lines(), ["    fn add(rhs) {", "        ", "    }"]);
        assert_eq!(code.selection(), Some((CodePos::new(0, 11), CodePos::new(0, 14))));
        assert!(code.snippet_active());
        // Typing takes the place of the default text, the place follows.
        code.type_char('x');
        code.drop_far_snippet();
        assert_eq!(code.lines()[0], "    fn add(x) {");
        assert!(code.next_stop());
        assert_eq!(code.cursor(), CodePos::new(1, 8));
        assert!(!code.snippet_active(), "the last place ends the session");
    }

    #[test]
    fn shift_tab_goes_back_and_a_far_cursor_ends_the_session() {
        let mut code = text("");
        code.insert_snippet(CodePos::default(), CodePos::default(), "f(${1:a}, ${2:b})$0");
        assert!(code.next_stop());
        assert_eq!(code.selected_text(), "b");
        assert!(code.previous_stop());
        assert_eq!(code.selected_text(), "a");
        assert!(!code.previous_stop(), "no place before the first");
        code.move_to(CodePos::new(0, 0), false);
        code.drop_far_snippet();
        assert!(!code.snippet_active());
        // A snippet with only its end puts the cursor there with no session.
        let mut code = text("");
        code.insert_snippet(CodePos::default(), CodePos::default(), "len()");
        assert_eq!(code.cursor(), CodePos::new(0, 5));
        assert!(!code.snippet_active());
    }

    fn typed(code: &mut CodeText, chars: &str) {
        for ch in chars.chars() {
            code.type_char(ch);
            code.drop_far_snippet();
        }
    }

    const IMPL: &str =
        "impl From<${1:T}> for ${2:Type} {\n    fn from(value: ${1:T}) -> Self {\n        $0\n    }\n}";

    #[test]
    fn typing_in_a_place_fills_its_copies() {
        let mut code = text("");
        code.insert_snippet(CodePos::default(), CodePos::default(), IMPL);
        assert_eq!(code.selected_text(), "T");
        typed(&mut code, "f64");
        assert_eq!(code.lines()[0], "impl From<f64> for Type {");
        assert_eq!(code.lines()[1], "    fn from(value: f64) -> Self {");
        assert_eq!(code.cursor(), CodePos::new(0, 13));
        assert_eq!(code.take_mirrored(), [1]);
        code.delete_backward();
        code.drop_far_snippet();
        assert_eq!(code.lines()[1], "    fn from(value: f6) -> Self {");
        // The place after it is not copied.
        assert!(code.next_stop());
        assert_eq!(code.selected_text(), "Type");
        typed(&mut code, "Meters");
        assert_eq!(code.lines()[0], "impl From<f6> for Meters {");
        assert_eq!(code.lines()[1], "    fn from(value: f6) -> Self {");
        assert_eq!(code.take_mirrored(), Vec::<usize>::new());
    }

    #[test]
    fn a_picked_completion_in_a_place_fills_its_copies_too() {
        let mut code = text("");
        code.insert_snippet(CodePos::default(), CodePos::default(), IMPL);
        typed(&mut code, "f6");
        // The app puts the completion f64 in place of the typed f6.
        code.replace_kept(CodePos::new(0, 10), CodePos::new(0, 12), "f64");
        assert_eq!(code.lines()[0], "impl From<f64> for Type {");
        assert_eq!(code.lines()[1], "    fn from(value: f64) -> Self {");
        assert_eq!(code.take_mirrored(), [1]);
        // A change of the app outside the place leaves the copies be.
        code.replace_kept(CodePos::new(1, 4), CodePos::new(1, 6), "pub fn");
        assert_eq!(code.lines()[1], "    pub fn from(value: f64) -> Self {");
        assert_eq!(code.take_mirrored(), Vec::<usize>::new());
        code.undo();
        code.undo();
        assert_eq!(code.lines()[0], "impl From<f6> for Type {");
        assert_eq!(code.lines()[1], "    fn from(value: f6) -> Self {");
    }

    #[test]
    fn a_copy_on_the_line_of_the_place_moves_the_cursor_on() {
        let mut code = text("");
        code.insert_snippet(
            CodePos::default(),
            CodePos::default(),
            "let ${1:a} = ${1:a} + ${1:a};",
        );
        typed(&mut code, "xy");
        assert_eq!(code.lines(), ["let xy = xy + xy;"]);
        assert_eq!(code.cursor(), CodePos::new(0, 6));
        assert!(code.snippet_active());
    }

    #[test]
    fn an_undo_takes_back_a_run_of_typing_with_its_copies() {
        let mut code = text("");
        code.insert_snippet(CodePos::default(), CodePos::default(), IMPL);
        // The first char takes the place of the default text, a step of
        // its own like with no copies, the rest is 1 run of typing.
        typed(&mut code, "f64");
        code.undo();
        assert_eq!(code.lines()[0], "impl From<f> for Type {");
        assert_eq!(code.lines()[1], "    fn from(value: f) -> Self {");
        code.undo();
        assert_eq!(code.lines()[0], "impl From<T> for Type {");
        assert_eq!(code.lines()[1], "    fn from(value: T) -> Self {");
        code.redo();
        code.redo();
        assert_eq!(code.lines()[1], "    fn from(value: f64) -> Self {");
        code.undo();
        code.undo();
        code.undo();
        assert_eq!(code.lines(), [""]);
    }
}
