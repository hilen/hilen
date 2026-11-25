//! The line keys of a code editor, the ones of the Zed keymap: deleting a
//! word or to an end of the line, deleting, moving and duplicating whole
//! lines, a new line below or above, selecting and joining lines, a
//! comment, and a cut or a paste of a whole line. Each change is 1 step
//! of the undo history.

use super::{
    comments::CommentRules,
    edit::{LinesChanged, StepKind},
    text::{CodePos, CodeText, Spot},
};

/// A change inside 1 line: the bytes `from..to` give way to `text`, with
/// no line break. With `stays` a place right at an insert keeps in
/// front of the new text, else it goes behind it.
struct LineEdit {
    line:  usize,
    from:  usize,
    to:    usize,
    text:  String,
    stays: bool,
}

impl LineEdit {
    fn insert(line: usize, at: usize, text: &str, stays: bool) -> Self {
        Self {
            line,
            from: at,
            to: at,
            text: text.to_string(),
            stays,
        }
    }

    fn remove(line: usize, from: usize, to: usize) -> Self {
        Self {
            line,
            from,
            to,
            text: String::new(),
            stays: false,
        }
    }
}

/// The spaces and tabs a line starts with, in bytes.
fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start_matches([' ', '\t']).len()
}

/// The byte of `line` at `column` chars, the line end for a column past
/// it.
fn byte_at_column(line: &str, column: usize) -> usize {
    line.char_indices().nth(column).map_or(line.len(), |(byte, _)| byte)
}

/// Where `pos` goes when `edits` change its line, each by the place it
/// had before them.
fn moved(pos: CodePos, edits: &[LineEdit]) -> CodePos {
    let (mut added, mut taken) = (0, 0);
    for edit in edits.iter().filter(|edit| edit.line == pos.line && edit.from <= pos.byte) {
        if edit.from == edit.to && edit.from == pos.byte {
            if !edit.stays {
                added += edit.text.len();
            }
        } else if pos.byte < edit.to {
            taken += pos.byte - edit.from;
        } else {
            added += edit.text.len();
            taken += edit.to - edit.from;
        }
    }
    CodePos::new(pos.line, pos.byte + added - taken)
}

impl CodeText {
    /// Puts `new` in place of the lines `first..=last` as 1 step, the
    /// selection at `after`. The pairs and a snippet end, like with
    /// Replace All.
    fn put_lines(&mut self, first: usize, last: usize, new: Vec<String>, after: Spot) -> LinesChanged {
        self.regions.clear();
        self.snippet = None;
        self.apply(first, last - first + 1, new, after, StepKind::Other)
    }

    /// Makes `edits` in the lines `first..=last` as 1 step, the cursor and
    /// the selection go along.
    fn edit_lines(&mut self, first: usize, last: usize, mut edits: Vec<LineEdit>) -> LinesChanged {
        let after = Spot {
            cursor: moved(self.cursor, &edits),
            anchor: self.anchor.map(|anchor| moved(anchor, &edits)),
        };
        let mut new = self.lines[first..=last].to_vec();
        edits.sort_by_key(|edit| (edit.line, edit.from));
        for edit in edits.iter().rev() {
            new[edit.line - first].replace_range(edit.from..edit.to, &edit.text);
        }
        self.put_lines(first, last, new, after)
    }

    /// The selection with every place in the lines `first..=last`, and an
    /// end of it at the start of the line after them, `by` lines down, or
    /// up when less than 0.
    fn spot_moved(&self, first: usize, last: usize, by: isize) -> Spot {
        let selects = self.selection().is_some();
        let moved = |pos: CodePos| {
            let inside =
                (first..=last).contains(&pos.line) || (selects && pos.line == last + 1 && pos.byte == 0);
            if inside {
                CodePos::new(pos.line.saturating_add_signed(by), pos.byte)
            } else {
                pos
            }
        };
        Spot {
            cursor: moved(self.cursor),
            anchor: self.anchor.map(moved),
        }
    }

    /// Deletes the selection, or the text from the cursor to where `to`
    /// moves it. The undo puts the cursor back where it was.
    fn delete_to(&mut self, to: impl FnOnce(&mut Self)) -> Option<LinesChanged> {
        if self.selection().is_some() {
            return self.delete_selection();
        }
        let spot = self.spot();
        to(self);
        let target = self.selection();
        self.set_spot(spot);
        let (from, to) = target?;
        Some(self.replace_to(from, to, "", StepKind::Other, None))
    }

    /// Alt Backspace: the selection, else back to the start of the word
    /// before the cursor. At a line start it joins the line to the one
    /// above.
    pub(crate) fn delete_word_backward(&mut self) -> Option<LinesChanged> {
        self.delete_to(|text| text.move_word_left(true))
    }

    /// Alt Delete: the mirror of `delete_word_backward`.
    pub(crate) fn delete_word_forward(&mut self) -> Option<LinesChanged> {
        self.delete_to(|text| text.move_word_right(true))
    }

    /// Cmd Backspace: from the start of the line of the selection start
    /// to the selection end. A cursor at a line start joins the line to
    /// the one above, like Backspace.
    pub(crate) fn delete_to_line_start(&mut self) -> Option<LinesChanged> {
        let (start, end) = self.selection().unwrap_or((self.cursor, self.cursor));
        if start == end && start.byte == 0 {
            return self.delete_backward();
        }
        Some(self.replace_to(CodePos::new(start.line, 0), end, "", StepKind::Other, None))
    }

    /// Cmd Delete: from the selection start to the end of its last line.
    /// A cursor at a line end joins the next line to it, like Delete.
    pub(crate) fn delete_to_line_end(&mut self) -> Option<LinesChanged> {
        let (start, end) = self.selection().unwrap_or((self.cursor, self.cursor));
        let line_end = CodePos::new(end.line, self.lines[end.line].len());
        if start == end && end == line_end {
            return self.delete_forward();
        }
        Some(self.replace_to(start, line_end, "", StepKind::Other, None))
    }

    /// Ctrl K: cuts from the cursor to the line end, or the line break at
    /// a line end, and gives what went, the text Ctrl Y puts back.
    pub(crate) fn kill_to_line_end(&mut self) -> Option<(String, LinesChanged)> {
        let pos = self.cursor;
        let len = self.lines[pos.line].len();
        let (to, text) = if pos.byte < len {
            (
                CodePos::new(pos.line, len),
                self.lines[pos.line][pos.byte..].to_string(),
            )
        } else if pos.line + 1 < self.lines.len() {
            (CodePos::new(pos.line + 1, 0), "\n".to_string())
        } else {
            return None;
        };
        let changed = self.replace_to(pos, to, "", StepKind::Other, None);
        Some((text, changed))
    }

    /// Cmd Shift K: the lines of the selection, or the line of the cursor,
    /// go with their breaks. The cursor keeps its column on the line that
    /// takes their place.
    pub(crate) fn delete_lines(&mut self) -> LinesChanged {
        let (first, last) = self.block();
        let column = self.column(self.cursor);
        let (start, end, kept) = if last + 1 < self.lines.len() {
            (first, last + 1, self.lines[last + 1].clone())
        } else if first > 0 {
            (first - 1, last, self.lines[first - 1].clone())
        } else {
            (first, last, String::new())
        };
        let after = Spot {
            cursor: CodePos::new(start, byte_at_column(&kept, column)),
            anchor: None,
        };
        self.put_lines(start, end, vec![kept], after)
    }

    /// Alt Up and Alt Down: the lines of the selection, or the line of the
    /// cursor, change place with the line above or below them. `None` at
    /// the top or the end of the text.
    pub(crate) fn move_block(&mut self, down: bool) -> Option<LinesChanged> {
        let (first, last) = self.block();
        if down {
            let below = self.lines.get(last + 1)?.clone();
            let mut new = vec![below];
            new.extend_from_slice(&self.lines[first..=last]);
            // A block that goes to the end of the text has no line after
            // it, a selection that ended at its start ends at the end of
            // the block.
            let count = self.lines.len();
            let block_end = CodePos::new(count - 1, self.lines[last].len());
            let inside = |pos: CodePos| if pos.line >= count { block_end } else { pos };
            let moved = self.spot_moved(first, last, 1);
            let after = Spot {
                cursor: inside(moved.cursor),
                anchor: moved.anchor.map(inside),
            };
            Some(self.put_lines(first, last + 1, new, after))
        } else {
            let above = first.checked_sub(1)?;
            let mut new = self.lines[first..=last].to_vec();
            new.push(self.lines[above].clone());
            let after = self.spot_moved(first, last, -1);
            Some(self.put_lines(above, last, new, after))
        }
    }

    /// Alt Shift Up and Alt Shift Down: the lines of the selection, or the
    /// line of the cursor, twice. The cursor goes with the upper copy or
    /// with the lower one.
    pub(crate) fn duplicate_lines(&mut self, up: bool) -> LinesChanged {
        let (first, last) = self.block();
        let block = self.lines[first..=last].to_vec();
        let mut new = block.clone();
        new.extend(block);
        let after = if up {
            self.spot()
        } else {
            let count = isize::try_from(last - first + 1).unwrap_or(isize::MAX);
            self.spot_moved(first, last, count)
        };
        self.put_lines(first, last, new, after)
    }

    /// Cmd Enter: a new line under the line of the cursor with the indent
    /// Enter gives at its end, the cursor on it.
    pub(crate) fn new_line_below(&mut self) -> LinesChanged {
        let line = self.cursor.line;
        let text = &self.lines[line];
        let mut indent = text[..indent_of(text)].to_string();
        if text.trim_end().ends_with(['{', '(', '[']) {
            indent.push_str(&" ".repeat(self.tab_width));
        }
        let end = CodePos::new(line, text.len());
        let cursor = CodePos::new(line + 1, indent.len());
        self.replace_to(end, end, &format!("\n{indent}"), StepKind::Other, Some(cursor))
    }

    /// Cmd Shift Enter: a new line over the line of the cursor with its
    /// indent, the cursor on it.
    pub(crate) fn new_line_above(&mut self) -> LinesChanged {
        let line = self.cursor.line;
        let text = &self.lines[line];
        let indent = text[..indent_of(text)].to_string();
        let start = CodePos::new(line, 0);
        let cursor = CodePos::new(line, indent.len());
        self.replace_to(
            start,
            start,
            &format!("{indent}\n"),
            StepKind::Other,
            Some(cursor),
        )
    }

    /// Cmd L: the whole lines of the selection with their breaks, so each
    /// press takes 1 line more.
    pub(crate) fn select_lines(&mut self) {
        let (start, end) = self.selection().unwrap_or((self.cursor, self.cursor));
        let to = if end.line + 1 < self.lines.len() {
            CodePos::new(end.line + 1, 0)
        } else {
            CodePos::new(end.line, self.lines[end.line].len())
        };
        self.select(CodePos::new(start.line, 0), to);
    }

    /// Ctrl J: the line of the cursor and the next one, or the lines of
    /// the selection, become 1 line. Each joined line loses its indent
    /// and a line comment it starts with, and 1 space goes before what is
    /// left. The cursor goes to the last place joined. `None` on the last
    /// line.
    pub(crate) fn join_lines(&mut self, comments: &CommentRules) -> Option<LinesChanged> {
        let (start, end) = self.selection().unwrap_or((self.cursor, self.cursor));
        let first = start.line;
        let last = if start.line == end.line {
            first + 1
        } else if end.byte == 0 && first + 1 < end.line {
            end.line - 1
        } else {
            end.line
        };
        let last = last.min(self.lines.len() - 1);
        if last <= first {
            return None;
        }
        let mut joined = self.lines[first].clone();
        let mut cursor = joined.len();
        for line in &self.lines[first + 1..=last] {
            cursor = joined.len();
            let code = &line[indent_of(line)..];
            let rest = &code[comments.line_prefix_len(code)..];
            if !rest.is_empty() {
                joined.push(' ');
            }
            joined.push_str(rest);
        }
        let after = Spot {
            cursor: CodePos::new(first, cursor),
            anchor: None,
        };
        Some(self.put_lines(first, last, vec![joined], after))
    }

    /// Cmd /: comments the lines of the selection, or the line of the
    /// cursor, or takes the comment out when every line that is not blank
    /// has one. A line comment goes at the smallest indent of the lines,
    /// a block comment around them. `None` for a language with no comment.
    pub(crate) fn toggle_comment(&mut self, comments: &CommentRules) -> Option<LinesChanged> {
        let (first, last) = self.block();
        let edits = if comments.line.is_empty() {
            let (start, end) = comments.block.as_ref()?;
            self.block_comment_edits(start, end, first, last)
        } else {
            self.line_comment_edits(comments, first, last)
        };
        Some(self.edit_lines(first, last, edits))
    }

    fn line_comment_edits(&self, comments: &CommentRules, first: usize, last: usize) -> Vec<LineEdit> {
        let mut found = Vec::new();
        let (mut commented, mut plain) = (0, 0);
        for (line, text) in (first..=last).zip(&self.lines[first..=last]) {
            let indent = indent_of(text);
            let len = comments.line_prefix_len(&text[indent..]);
            // A blank line in a block never has a comment, it must not
            // keep a commented block from losing its comments.
            if first == last || !text.trim().is_empty() {
                if len > 0 {
                    commented += 1;
                } else {
                    plain += 1;
                }
            }
            found.push((line, indent, len));
        }
        if plain == 0 && commented > 0 {
            return found
                .into_iter()
                .filter(|(_, _, len)| *len > 0)
                .map(|(line, indent, len)| LineEdit::remove(line, indent, indent + len))
                .collect();
        }
        let column = found.iter().map(|(_, indent, _)| *indent).min().unwrap_or(0);
        found
            .into_iter()
            .map(|(line, _, _)| LineEdit::insert(line, column, &comments.line[0], false))
            .collect()
    }

    fn block_comment_edits(&self, start: &str, end: &str, first: usize, last: usize) -> Vec<LineEdit> {
        let first_text = &self.lines[first];
        let indent = indent_of(first_text);
        let start_rules = CommentRules {
            line:  vec![start.to_string()],
            block: None,
        };
        let opened = start_rules.line_prefix_len(&first_text[indent..]);
        let last_text = &self.lines[last];
        let bare_end = end.trim_start_matches(' ');
        let closed = last_text.ends_with(bare_end).then(|| {
            let at = last_text.len() - bare_end.len();
            let space = end.starts_with(' ') && last_text[..at].ends_with(' ');
            at - usize::from(space)
        });
        // An opener and a closer that share bytes are no comment.
        let apart = first != last || closed.is_some_and(|at| at >= indent + opened);
        match closed {
            Some(at) if opened > 0 && apart => vec![
                LineEdit::remove(first, indent, indent + opened),
                LineEdit::remove(last, at, last_text.len()),
            ],
            _ => vec![
                LineEdit::insert(first, indent, start, false),
                LineEdit::insert(last, last_text.len(), end, true),
            ],
        }
    }

    /// Cmd C with no selection: the line of the cursor with its break,
    /// the last line too.
    pub(crate) fn line_with_break(&self) -> String {
        format!("{}\n", self.lines[self.cursor.line])
    }

    /// Cmd X with no selection: the line of the cursor goes with its
    /// break, the last line only with its text. Gives what went.
    pub(crate) fn cut_line(&mut self) -> (String, LinesChanged) {
        let line = self.cursor.line;
        let from = CodePos::new(line, 0);
        let (to, text) = if line + 1 < self.lines.len() {
            (CodePos::new(line + 1, 0), self.line_with_break())
        } else {
            (
                CodePos::new(line, self.lines[line].len()),
                self.lines[line].clone(),
            )
        };
        let changed = self.replace_to(from, to, "", StepKind::Other, None);
        (text, changed)
    }

    /// The paste of a whole line that Cmd C or Cmd X took: it goes at the
    /// start of the line of the cursor, which moves along with its line.
    /// A browser has no paste here, see `CodeEditor::paste`.
    #[cfg(not_wasm)]
    pub(crate) fn paste_line(&mut self, text: &str) -> LinesChanged {
        let pos = self.cursor;
        let start = CodePos::new(pos.line, 0);
        let breaks = text.matches('\n').count();
        let tail = text.rsplit('\n').next().unwrap_or_default().len();
        let cursor = if breaks == 0 {
            CodePos::new(pos.line, pos.byte + tail)
        } else {
            CodePos::new(pos.line + breaks, pos.byte + tail)
        };
        self.replace_to(start, start, text, StepKind::Other, Some(cursor))
    }

    /// Ctrl T: the chars on both sides of the cursor change place, and
    /// the cursor steps over them. At a line end the 2 chars before it.
    pub(crate) fn transpose(&mut self) -> Option<LinesChanged> {
        let pos = self.cursor;
        let text = &self.lines[pos.line];
        let before = text[..pos.byte].chars().next_back()?;
        let (first, second, end) = if let Some(after) = text[pos.byte..].chars().next() {
            (before, after, pos.byte + after.len_utf8())
        } else {
            let start = pos.byte - before.len_utf8();
            let earlier = text[..start].chars().next_back()?;
            (earlier, before, pos.byte)
        };
        let from = end - first.len_utf8() - second.len_utf8();
        let swapped = format!("{second}{first}");
        let cursor = CodePos::new(pos.line, end);
        Some(self.replace_to(
            CodePos::new(pos.line, from),
            CodePos::new(pos.line, end),
            &swapped,
            StepKind::Other,
            Some(cursor),
        ))
    }
}
