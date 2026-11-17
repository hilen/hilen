//! The changes of a `CodeText` and its undo history. Every change is 1
//! step: some lines from a start line gave way to new ones. Typed chars
//! and deleted chars on 1 line join the step before them until the
//! cursor moves, so an undo takes back a run of typing, not 1 char.

use super::text::{CodePos, CodeText, Spot, UNDO_STEPS, clean};

/// Which lines a change replaced: `removed` lines from `start` gave way
/// to `inserted` new ones. The view moves the colors and the marks of
/// the lines after them along.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LinesChanged {
    pub start:    usize,
    pub removed:  usize,
    pub inserted: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StepKind {
    Typing,
    Deleting,
    Other,
}

/// 1 step of the undo history: the lines from `start` before and after.
#[derive(Debug, Clone)]
pub(crate) struct Step {
    start:  usize,
    old:    Vec<String>,
    new:    Vec<String>,
    before: Spot,
    after:  Spot,
    kind:   StepKind,
}

/// Brackets after which a new line starts 1 level deeper.
const OPENERS: [char; 3] = ['{', '(', '['];

impl CodeText {
    pub(crate) fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub(crate) fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Puts `new` in place of `removed` lines from `start` and the cursor
    /// at `after`, as 1 step of the history.
    fn apply(
        &mut self,
        start: usize,
        removed: usize,
        new: Vec<String>,
        after: Spot,
        kind: StepKind,
    ) -> LinesChanged {
        let before = self.spot();
        let old: Vec<String> = self.lines.splice(start..start + removed, new.iter().cloned()).collect();
        let changed = LinesChanged {
            start,
            removed,
            inserted: new.len(),
        };
        self.set_spot(after);
        let after = self.spot();
        self.goal = None;
        self.redo.clear();

        let joins = self.open_step
            && kind != StepKind::Other
            && removed == 1
            && new.len() == 1
            && self.undo.last().is_some_and(|last| {
                last.kind == kind && last.start == start && last.new.len() == 1 && last.after == before
            });

        if joins && let Some(last) = self.undo.last_mut() {
            last.new = new;
            last.after = after;
        } else {
            self.undo.push(Step {
                start,
                old,
                new,
                before,
                after,
                kind,
            });
            if self.undo.len() > UNDO_STEPS {
                self.undo.remove(0);
            }
        }
        self.open_step = kind != StepKind::Other;
        changed
    }

    /// Puts `text` in place of the text from `from` to `to`, the cursor
    /// right after it.
    fn replace(&mut self, from: CodePos, to: CodePos, text: &str, kind: StepKind) -> LinesChanged {
        let (from, to) = (self.clamp(from.min(to)), self.clamp(from.max(to)));
        let suffix = &self.lines[to.line][to.byte..];
        let joined = format!("{}{text}{suffix}", &self.lines[from.line][..from.byte]);
        let suffix_len = suffix.len();

        let new: Vec<String> = joined.split('\n').map(str::to_string).collect();
        let last = new.len() - 1;
        let cursor = CodePos::new(from.line + last, new[last].len() - suffix_len);

        let after = Spot { cursor, anchor: None };
        self.apply(from.line, to.line - from.line + 1, new, after, kind)
    }

    /// Puts `text` in place of the text from `from` to `to` as a step of
    /// its own, the cursor right after it. The replace of 1 match.
    pub(crate) fn replace_range(&mut self, from: CodePos, to: CodePos, text: &str) -> LinesChanged {
        self.replace(from, to, &clean(text), StepKind::Other)
    }

    /// Puts each text in place of its range, all in 1 step of the
    /// history, the cursor at the start of the first range. The ranges
    /// are in the order of the text, none over a line break and none
    /// over another. The Replace All of the find bar. `None` for no
    /// ranges.
    pub(crate) fn replace_all(&mut self, edits: &[(CodePos, CodePos, String)]) -> Option<LinesChanged> {
        let (first, _, _) = edits.first()?;
        let (last, _, _) = edits.last()?;
        let (first, last) = (first.line, last.line);
        let mut lines = self.lines[first..=last].to_vec();
        for (from, to, text) in edits.iter().rev() {
            lines[from.line - first].replace_range(from.byte..to.byte, &clean(text));
        }
        // A replacement with a line break makes more lines.
        let new: Vec<String> = lines.join("\n").split('\n').map(str::to_string).collect();
        let after = Spot {
            cursor: edits[0].0,
            anchor: None,
        };
        Some(self.apply(first, last - first + 1, new, after, StepKind::Other))
    }

    /// The selection, or the empty place of the cursor.
    fn target(&self) -> (CodePos, CodePos) {
        self.selection().unwrap_or((self.cursor, self.cursor))
    }

    /// Puts `text` in place of the selection, or at the cursor, as a
    /// step of its own. A paste.
    pub(crate) fn insert(&mut self, text: &str) -> LinesChanged {
        let (from, to) = self.target();
        self.replace(from, to, &clean(text), StepKind::Other)
    }

    /// A typed char, it joins the typing before it on the line.
    pub(crate) fn type_char(&mut self, ch: char) -> LinesChanged {
        let (from, to) = self.target();
        let kind = if from == to {
            StepKind::Typing
        } else {
            StepKind::Other
        };
        self.replace(from, to, ch.encode_utf8(&mut [0; 4]), kind)
    }

    /// Enter: a line break and the indent of the line, 1 level more after
    /// an open bracket.
    pub(crate) fn new_line(&mut self) -> LinesChanged {
        let (from, to) = self.target();
        let text = self.line(from.line);
        let before = &text[..from.byte];
        let indent = before.len() - before.trim_start().len();
        let mut spaces = indent;
        if before.trim_end().ends_with(OPENERS) {
            spaces += self.tab_width;
        }
        let inserted = format!("\n{}", " ".repeat(spaces));
        self.replace(from, to, &inserted, StepKind::Other)
    }

    pub(crate) fn delete_selection(&mut self) -> Option<LinesChanged> {
        let (from, to) = self.selection()?;
        Some(self.replace(from, to, "", StepKind::Other))
    }

    /// Backspace. In the indent of a line it goes back to the tab stop
    /// before, at a line start it joins the line to the one above.
    pub(crate) fn delete_backward(&mut self) -> Option<LinesChanged> {
        if self.selection().is_some() {
            return self.delete_selection();
        }
        let pos = self.cursor;
        if pos.byte == 0 {
            let line = pos.line.checked_sub(1)?;
            let end = CodePos::new(line, self.lines[line].len());
            return Some(self.replace(end, pos, "", StepKind::Deleting));
        }
        let before = &self.lines[pos.line][..pos.byte];
        let start = if before.chars().all(|ch| ch == ' ') {
            let back = (pos.byte - 1) % self.tab_width + 1;
            pos.byte - back
        } else {
            before.chars().last().map_or(0, |ch| pos.byte - ch.len_utf8())
        };
        Some(self.replace(CodePos::new(pos.line, start), pos, "", StepKind::Deleting))
    }

    /// Delete. At a line end it joins the next line to this one.
    pub(crate) fn delete_forward(&mut self) -> Option<LinesChanged> {
        if self.selection().is_some() {
            return self.delete_selection();
        }
        let pos = self.cursor;
        let text = &self.lines[pos.line];
        let end = match text[pos.byte..].chars().next() {
            Some(ch) => CodePos::new(pos.line, pos.byte + ch.len_utf8()),
            None if pos.line + 1 < self.lines.len() => CodePos::new(pos.line + 1, 0),
            None => return None,
        };
        Some(self.replace(pos, end, "", StepKind::Deleting))
    }

    /// The lines a Tab or Shift Tab changes: those of the selection, a
    /// last line the selection only touches at its start left out, or
    /// the line of the cursor.
    fn block(&self) -> (usize, usize) {
        match self.selection() {
            Some((start, end)) if end.line > start.line && end.byte == 0 => (start.line, end.line - 1),
            Some((start, end)) => (start.line, end.line),
            None => (self.cursor.line, self.cursor.line),
        }
    }

    /// Tab. With a selection every line of it gets 1 level more, an empty
    /// line stays empty. With none spaces go in up to the next tab stop.
    pub(crate) fn indent(&mut self) -> LinesChanged {
        if self.selection().is_none() {
            let spaces = self.tab_width - self.column(self.cursor) % self.tab_width;
            return self.replace(self.cursor, self.cursor, &" ".repeat(spaces), StepKind::Other);
        }
        let (first, last) = self.block();
        let width = self.tab_width;
        let pad = " ".repeat(width);
        let new: Vec<String> = self.lines[first..=last]
            .iter()
            .map(|line| {
                if line.is_empty() {
                    String::new()
                } else {
                    format!("{pad}{line}")
                }
            })
            .collect();

        let shift = |pos: CodePos| {
            let grows = pos.line >= first && pos.line <= last && !self.lines[pos.line].is_empty();
            CodePos::new(pos.line, if grows { pos.byte + width } else { pos.byte })
        };
        let after = Spot {
            cursor: shift(self.cursor),
            anchor: self.anchor.map(shift),
        };
        self.apply(first, last - first + 1, new, after, StepKind::Other)
    }

    /// Shift Tab. Every line of the selection, or the line of the cursor,
    /// loses up to 1 level of spaces, or 1 tab char. `None` when no line
    /// had any.
    pub(crate) fn outdent(&mut self) -> Option<LinesChanged> {
        let (first, last) = self.block();
        let width = self.tab_width;
        let cut = |line: &str| {
            if line.starts_with('\t') {
                1
            } else {
                line.chars().take(width).take_while(|ch| *ch == ' ').count()
            }
        };
        let cuts: Vec<usize> = self.lines[first..=last].iter().map(|line| cut(line)).collect();
        if cuts.iter().all(|cut| *cut == 0) {
            return None;
        }
        let new: Vec<String> = self.lines[first..=last]
            .iter()
            .zip(&cuts)
            .map(|(line, cut)| line[*cut..].to_string())
            .collect();

        let shift = |pos: CodePos| {
            if pos.line < first || pos.line > last {
                return pos;
            }
            CodePos::new(pos.line, pos.byte.saturating_sub(cuts[pos.line - first]))
        };
        let after = Spot {
            cursor: shift(self.cursor),
            anchor: self.anchor.map(shift),
        };
        Some(self.apply(first, last - first + 1, new, after, StepKind::Other))
    }

    pub(crate) fn undo(&mut self) -> Option<LinesChanged> {
        let step = self.undo.pop()?;
        self.lines
            .splice(step.start..step.start + step.new.len(), step.old.iter().cloned());
        self.set_spot(step.before);
        self.goal = None;
        self.open_step = false;
        let changed = LinesChanged {
            start:    step.start,
            removed:  step.new.len(),
            inserted: step.old.len(),
        };
        self.redo.push(step);
        Some(changed)
    }

    pub(crate) fn redo(&mut self) -> Option<LinesChanged> {
        let step = self.redo.pop()?;
        self.lines
            .splice(step.start..step.start + step.old.len(), step.new.iter().cloned());
        self.set_spot(step.after);
        self.goal = None;
        self.open_step = false;
        let changed = LinesChanged {
            start:    step.start,
            removed:  step.old.len(),
            inserted: step.new.len(),
        };
        self.undo.push(step);
        Some(changed)
    }
}
