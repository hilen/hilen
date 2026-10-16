//! The text of a `CodeEditor`: its lines, the cursor, the selection and
//! the moves of the cursor. No view, so every rule has a unit test. The
//! changes and the undo history are in `edit.rs`.

use super::edit::Step;

/// The undo history keeps this many steps, the oldest go first.
pub(super) const UNDO_STEPS: usize = 1000;

/// A place in the text of a `CodeEditor`: a line, from 0, and a byte of
/// that line.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodePos {
    pub line: usize,
    pub byte: usize,
}

impl CodePos {
    pub const fn new(line: usize, byte: usize) -> Self {
        Self { line, byte }
    }
}

/// The cursor and the other end of the selection.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct Spot {
    pub cursor: CodePos,
    pub anchor: Option<CodePos>,
}

/// The lines of a code file and where the user is in them.
#[derive(Debug)]
pub(crate) struct CodeText {
    pub(super) lines:     Vec<String>,
    pub(super) cursor:    CodePos,
    /// The other end of the selection, the cursor is the end that moves.
    /// `None` or equal to the cursor is no selection.
    pub(super) anchor:    Option<CodePos>,
    /// The column a move up or down keeps, so the cursor comes back to it
    /// after a short line.
    pub(super) goal:      Option<usize>,
    pub(super) tab_width: usize,
    pub(super) undo:      Vec<Step>,
    pub(super) redo:      Vec<Step>,
    /// The last step of the history takes the next typed char into it.
    pub(super) open_step: bool,
}

impl Default for CodeText {
    fn default() -> Self {
        Self {
            lines:     vec![String::new()],
            cursor:    CodePos::default(),
            anchor:    None,
            goal:      None,
            tab_width: 4,
            undo:      Vec::new(),
            redo:      Vec::new(),
            open_step: false,
        }
    }
}

/// A char of a name, a word for a jump of the cursor and for a find of
/// whole words.
pub(super) fn is_word(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

impl CodeText {
    /// A new text, with the cursor at the start and no history. A line
    /// break of Windows counts as 1.
    pub(crate) fn set_text(&mut self, text: &str) {
        *self = Self {
            lines: clean(text).split('\n').map(str::to_string).collect(),
            tab_width: self.tab_width,
            ..Self::default()
        };
    }

    pub(crate) fn text(&self) -> String {
        self.lines.join("\n")
    }

    pub(crate) fn lines(&self) -> &[String] {
        &self.lines
    }

    pub(crate) fn line(&self, line: usize) -> &str {
        self.lines.get(line).map_or("", String::as_str)
    }

    pub(crate) fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub(crate) fn cursor(&self) -> CodePos {
        self.cursor
    }

    pub(crate) fn tab_width(&self) -> usize {
        self.tab_width
    }

    pub(crate) fn set_tab_width(&mut self, width: usize) {
        self.tab_width = width.max(1);
    }

    pub(super) fn spot(&self) -> Spot {
        Spot {
            cursor: self.cursor,
            anchor: self.anchor,
        }
    }

    pub(super) fn set_spot(&mut self, spot: Spot) {
        self.cursor = self.clamp(spot.cursor);
        self.anchor = spot.anchor.map(|anchor| self.clamp(anchor));
    }

    /// The selection, start before end, `None` when nothing is selected.
    pub(crate) fn selection(&self) -> Option<(CodePos, CodePos)> {
        let anchor = self.anchor?;
        if anchor == self.cursor {
            return None;
        }
        Some((anchor.min(self.cursor), anchor.max(self.cursor)))
    }

    /// The selected text, lines joined by a line break, empty when
    /// nothing is selected.
    pub(crate) fn selected_text(&self) -> String {
        let Some((start, end)) = self.selection() else {
            return String::new();
        };
        if start.line == end.line {
            return self.lines[start.line][start.byte..end.byte].to_string();
        }
        let mut parts = vec![&self.lines[start.line][start.byte..]];
        parts.extend(self.lines[start.line + 1..end.line].iter().map(String::as_str));
        parts.push(&self.lines[end.line][..end.byte]);
        parts.join("\n")
    }

    /// The bytes of `line` that are selected, and whether the selection
    /// goes on past its end, over its line break.
    pub(crate) fn selected_on(&self, line: usize) -> Option<(std::ops::Range<usize>, bool)> {
        let (start, end) = self.selection()?;
        if line < start.line || line > end.line {
            return None;
        }
        let len = self.line(line).len();
        let from = if line == start.line { start.byte } else { 0 };
        let to = if line == end.line { end.byte } else { len };
        Some((from..to, line < end.line))
    }

    /// `pos` moved into the text: the last line at most, and a byte on a
    /// char boundary of its line.
    pub(crate) fn clamp(&self, pos: CodePos) -> CodePos {
        let line = pos.line.min(self.lines.len() - 1);
        let text = &self.lines[line];
        CodePos::new(line, text.floor_char_boundary(pos.byte.min(text.len())))
    }

    /// The column of `pos`, its count of chars from the line start.
    pub(crate) fn column(&self, pos: CodePos) -> usize {
        self.line(pos.line)[..pos.byte].chars().count()
    }

    /// The byte of `line` at `column`, the line end for a column past it.
    fn byte_at_column(&self, line: usize, column: usize) -> usize {
        let text = self.line(line);
        text.char_indices().nth(column).map_or(text.len(), |(byte, _)| byte)
    }

    /// Puts the cursor at `pos`. With `extend` the selection grows from
    /// where it started, or from the old cursor. Without it the selection
    /// goes.
    pub(crate) fn move_to(&mut self, pos: CodePos, extend: bool) {
        if extend {
            if self.anchor.is_none() {
                self.anchor = Some(self.cursor);
            }
        } else {
            self.anchor = None;
        }
        self.cursor = self.clamp(pos);
        self.goal = None;
        self.open_step = false;
    }

    pub(crate) fn select(&mut self, from: CodePos, to: CodePos) {
        self.move_to(from, false);
        self.move_to(to, true);
    }

    pub(crate) fn select_all(&mut self) {
        let last = self.lines.len() - 1;
        self.select(CodePos::default(), CodePos::new(last, self.lines[last].len()));
    }

    /// The name or the run of other chars around `pos`, the double click.
    pub(crate) fn select_word(&mut self, pos: CodePos) {
        let pos = self.clamp(pos);
        let text = self.line(pos.line);
        let start = text[..pos.byte]
            .char_indices()
            .rev()
            .take_while(|(_, ch)| is_word(*ch))
            .last()
            .map_or(pos.byte, |(byte, _)| byte);
        let end = text[pos.byte..]
            .char_indices()
            .find(|(_, ch)| !is_word(*ch))
            .map_or(text.len(), |(byte, _)| pos.byte + byte);
        self.select(CodePos::new(pos.line, start), CodePos::new(pos.line, end));
    }

    /// The whole line with its line break, the triple click.
    pub(crate) fn select_line(&mut self, line: usize) {
        let line = line.min(self.lines.len() - 1);
        let end = if line + 1 < self.lines.len() {
            CodePos::new(line + 1, 0)
        } else {
            CodePos::new(line, self.lines[line].len())
        };
        self.select(CodePos::new(line, 0), end);
    }

    pub(crate) fn move_left(&mut self, extend: bool) {
        if !extend && let Some((start, _)) = self.selection() {
            self.move_to(start, false);
            return;
        }
        let pos = self.cursor;
        let target = match self.line(pos.line)[..pos.byte].chars().last() {
            Some(ch) => CodePos::new(pos.line, pos.byte - ch.len_utf8()),
            None if pos.line > 0 => CodePos::new(pos.line - 1, self.lines[pos.line - 1].len()),
            None => pos,
        };
        self.move_to(target, extend);
    }

    pub(crate) fn move_right(&mut self, extend: bool) {
        if !extend && let Some((_, end)) = self.selection() {
            self.move_to(end, false);
            return;
        }
        let pos = self.cursor;
        let target = match self.line(pos.line)[pos.byte..].chars().next() {
            Some(ch) => CodePos::new(pos.line, pos.byte + ch.len_utf8()),
            None if pos.line + 1 < self.lines.len() => CodePos::new(pos.line + 1, 0),
            None => pos,
        };
        self.move_to(target, extend);
    }

    /// Up for a negative `lines`, down for a positive one, in the column
    /// the cursor had before the first of such moves. A move past the
    /// first line goes to its start, past the last line to its end.
    pub(crate) fn move_lines(&mut self, lines: isize, extend: bool) {
        let goal = self.goal.unwrap_or_else(|| self.column(self.cursor));
        let last = self.lines.len() - 1;
        let target = self.cursor.line.checked_add_signed(lines);

        let pos = match target {
            None => CodePos::new(0, 0),
            Some(line) if line > last => CodePos::new(last, self.lines[last].len()),
            Some(line) => CodePos::new(line, self.byte_at_column(line, goal)),
        };
        self.move_to(pos, extend);
        self.goal = Some(goal);
    }

    /// To the first char that is no space, or to the line start when the
    /// cursor is there already.
    pub(crate) fn move_home(&mut self, extend: bool) {
        let line = self.cursor.line;
        let text = self.line(line);
        let first = text.len() - text.trim_start().len();
        let byte = if self.cursor.byte == first { 0 } else { first };
        self.move_to(CodePos::new(line, byte), extend);
    }

    pub(crate) fn move_end(&mut self, extend: bool) {
        let line = self.cursor.line;
        self.move_to(CodePos::new(line, self.line(line).len()), extend);
    }

    pub(crate) fn move_to_start(&mut self, extend: bool) {
        self.move_to(CodePos::default(), extend);
    }

    pub(crate) fn move_to_end(&mut self, extend: bool) {
        let last = self.lines.len() - 1;
        self.move_to(CodePos::new(last, self.lines[last].len()), extend);
    }

    /// Past the spaces, then past the name or the run of other chars.
    /// At a line end to the start of the next line.
    pub(crate) fn move_word_right(&mut self, extend: bool) {
        let pos = self.cursor;
        let text = self.line(pos.line);
        if pos.byte == text.len() {
            self.move_right(extend);
            return;
        }
        let rest = &text[pos.byte..];
        let after_spaces = rest.len() - rest.trim_start().len();
        let rest = &rest[after_spaces..];
        let word = rest.chars().next().is_some_and(is_word);
        let run = rest
            .char_indices()
            .find(|(_, ch)| ch.is_whitespace() || is_word(*ch) != word)
            .map_or(rest.len(), |(byte, _)| byte);
        self.move_to(CodePos::new(pos.line, pos.byte + after_spaces + run), extend);
    }

    /// The mirror of `move_word_right`.
    pub(crate) fn move_word_left(&mut self, extend: bool) {
        let pos = self.cursor;
        if pos.byte == 0 {
            self.move_left(extend);
            return;
        }
        let before = &self.line(pos.line)[..pos.byte];
        let trimmed = before.trim_end();
        let word = trimmed.chars().last().is_some_and(is_word);
        let start = trimmed
            .char_indices()
            .rev()
            .find(|(_, ch)| ch.is_whitespace() || is_word(*ch) != word)
            .map_or(0, |(byte, ch)| byte + ch.len_utf8());
        self.move_to(CodePos::new(pos.line, start), extend);
    }
}

/// Line breaks of Windows and of old Macs as plain line breaks.
pub(super) fn clean(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}
