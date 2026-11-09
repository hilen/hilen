use web_time::Instant;

use super::{DiffPanel, DiffRow, HEADER_H, cell::DiffLineCell};
use crate::{
    refs::Weak,
    system::Clipboard,
    ui::{Point, ViewFrame},
};

// The old app had the browser's text selection over the diff: click and
// drag selects code character by character across lines, gutters and
// markers stay out, cmd or ctrl C copies, a click elsewhere clears. This
// module is that behavior over the engine table. In split mode the
// selection stays in the half the drag began on.

/// Two clicks this close in time and place grow the click count: the
/// second selects the word, the third the line.
const MULTI_CLICK_MS: u128 = 400;
const MULTI_CLICK_DISTANCE: f32 = 6.0;

impl DiffPanel {
    pub(super) fn sel_began(mut self: Weak<Self>, position: Point) {
        if position.y <= HEADER_H {
            self.sel_clear();
            return;
        }
        let now = Instant::now();
        let clicks = match self.last_click {
            Some((at, at_pos, n))
                if now.duration_since(at).as_millis() < MULTI_CLICK_MS
                    && (at_pos - position).length() < MULTI_CLICK_DISTANCE =>
            {
                n % 3 + 1
            }
            _ => 1,
        };
        self.last_click = Some((now, position, clicks));
        let Some((row, byte, right)) = self.sel_pos(position, None) else {
            self.sel_clear();
            return;
        };
        self.sel.right = right;
        self.sel.dragging = clicks == 1;
        match clicks {
            2 => self.select_word(row, byte),
            3 => self.select_line(row),
            _ => {
                self.sel_anchor = Some((row, byte));
                self.sel_caret = Some((row, byte));
            }
        }
        self.apply_selection();
    }

    pub(super) fn sel_moved(mut self: Weak<Self>, position: Point) {
        if !self.sel.dragging {
            return;
        }
        let Some((row, byte, _)) = self.sel_pos(position, Some(self.sel.right)) else {
            return;
        };
        if self.sel_caret == Some((row, byte)) {
            return;
        }
        self.sel_caret = Some((row, byte));
        self.apply_selection();
    }

    pub(super) fn sel_ended(mut self: Weak<Self>) {
        self.sel.dragging = false;
        if self.sel_anchor == self.sel_caret {
            self.sel_clear();
        }
    }

    pub(super) fn sel_clear(mut self: Weak<Self>) {
        if self.sel_anchor.is_none() && self.sel_caret.is_none() {
            return;
        }
        self.sel_anchor = None;
        self.sel_caret = None;
        self.sel.dragging = false;
        self.apply_selection();
    }

    /// The row, byte and half under a panel position. The position is
    /// clamped into the table and its content, so a drag past an edge
    /// lands on the nearest row. `lock_right` keeps a drag in the half
    /// it began on.
    pub(super) fn sel_pos(
        self: Weak<Self>,
        position: Point,
        lock_right: Option<bool>,
    ) -> Option<(usize, usize, bool)> {
        let x = position.x.clamp(0.0, (self.table.width() - 0.1).max(0.0));
        let y = (position.y - HEADER_H).clamp(0.0, (self.table.height() - 0.1).max(0.0));
        let offset = self.table.content_offset();
        let content = self.table.content_height();
        let y = (y - offset).clamp(0.0, (content - 0.1).max(0.0)) + offset;
        let index = self.table.index_at(Point { x, y })?;
        let (_, view) = self.table.visible_cells().into_iter().find(|(i, _)| *i == index)?;
        let cell = view.downcast::<DiffLineCell>()?;
        let right = lock_right.unwrap_or_else(|| cell.on_right_half(x));
        Some((index, cell.byte_at(x, right), right))
    }

    fn select_word(mut self: Weak<Self>, row: usize, byte: usize) {
        let (start, end) = word_range(self.sel_text_of(row), byte);
        self.sel_anchor = Some((row, start));
        self.sel_caret = Some((row, end));
    }

    fn select_line(mut self: Weak<Self>, row: usize) {
        let len = self.sel_text_of(row).len();
        self.sel_anchor = Some((row, 0));
        self.sel_caret = Some((row, len));
    }

    fn sel_text_of(&self, row: usize) -> &str {
        sel_text(self.rows.get(row), self.sel.right)
    }

    /// The selected byte range of one row, `None` when the selection
    /// misses the row or covers nothing of it.
    pub(super) fn row_selection(&self, row: usize) -> Option<(usize, usize, bool)> {
        let (a, c) = ordered(self.sel_anchor?, self.sel_caret?);
        let (start, end) = row_range(row, a, c, self.sel_text_of(row).len())?;
        Some((start, end, self.sel.right))
    }

    pub(super) fn apply_selection(self: Weak<Self>) {
        for (index, view) in self.table.visible_cells() {
            let Some(cell) = view.downcast::<DiffLineCell>() else {
                continue;
            };
            cell.set_selection(self.row_selection(index));
        }
    }

    pub(super) fn copy_selection(self: Weak<Self>) {
        let (Some(anchor), Some(caret)) = (self.sel_anchor, self.sel_caret) else {
            return;
        };
        let (a, c) = ordered(anchor, caret);
        let mut parts = Vec::new();
        for row in a.0..=c.0 {
            let text = self.sel_text_of(row);
            match row_range(row, a, c, text.len()) {
                Some((start, end)) => parts.push(text[start..end].to_string()),
                None => parts.push(String::new()),
            }
        }
        if let Err(err) = Clipboard::set_text(parts.join("\n")) {
            log::error!("Failed to copy the diff selection: {err}");
        }
    }
}

fn ordered(a: (usize, usize), b: (usize, usize)) -> ((usize, usize), (usize, usize)) {
    if a <= b { (a, b) } else { (b, a) }
}

/// The byte range of `row` inside the ordered selection `a..c`, against
/// a row text of `len` bytes. `None` when the row is outside or its part
/// is empty, so an untouched row shows no highlight while a covered
/// empty line still copies as an empty part.
fn row_range(row: usize, a: (usize, usize), c: (usize, usize), len: usize) -> Option<(usize, usize)> {
    if row < a.0 || row > c.0 {
        return None;
    }
    let start = if row == a.0 { a.1.min(len) } else { 0 };
    let end = if row == c.0 { c.1.min(len) } else { len };
    (start < end).then_some((start, end))
}

/// The run of letters, digits and underscores around `byte`.
pub fn word_range(text: &str, byte: usize) -> (usize, usize) {
    let byte = byte.min(text.len());
    let is_word = |ch: char| ch.is_alphanumeric() || ch == '_';
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
    (start, end)
}

/// The selectable text of one row: code text, hunk title or note, with
/// gutters and markers never part of it, like the original's
/// select-none spans. A split row answers for the chosen half.
fn sel_text(row: Option<&DiffRow>, right: bool) -> &str {
    match row {
        Some(DiffRow::Hunk { title, .. }) => title,
        Some(DiffRow::Line { text, .. } | DiffRow::Note(text)) => text,
        Some(DiffRow::Split { left, right: r, .. }) => {
            let half = if right { r } else { left };
            half.as_ref().map_or("", |h| h.text.as_str())
        }
        None => "",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::{DiffRow, Half},
        ordered, row_range, sel_text, word_range,
    };

    #[test]
    fn orders_by_row_then_byte() {
        assert_eq!(ordered((2, 1), (1, 9)), ((1, 9), (2, 1)));
        assert_eq!(ordered((1, 7), (1, 3)), ((1, 3), (1, 7)));
        assert_eq!(ordered((1, 3), (2, 0)), ((1, 3), (2, 0)));
    }

    #[test]
    fn row_ranges_across_rows() {
        let a = (1, 4);
        let c = (3, 2);
        assert_eq!(row_range(0, a, c, 10), None);
        assert_eq!(row_range(1, a, c, 10), Some((4, 10)));
        assert_eq!(row_range(2, a, c, 10), Some((0, 10)));
        assert_eq!(row_range(3, a, c, 10), Some((0, 2)));
        assert_eq!(row_range(4, a, c, 10), None);
    }

    #[test]
    fn row_range_on_one_row_and_empty_lines() {
        assert_eq!(row_range(1, (1, 2), (1, 6), 10), Some((2, 6)));
        assert_eq!(row_range(1, (1, 2), (1, 2), 10), None);
        // A covered empty line highlights nothing but still copies.
        assert_eq!(row_range(2, (1, 0), (3, 0), 0), None);
        // The anchor byte clamps to a shorter row.
        assert_eq!(row_range(1, (1, 50), (2, 0), 4), None);
    }

    #[test]
    fn words_and_their_edges() {
        assert_eq!(word_range("let foo_bar = 1;", 5), (4, 11));
        assert_eq!(word_range("let foo_bar = 1;", 0), (0, 3));
        assert_eq!(word_range("let foo_bar = 1;", 12), (12, 12));
        // A click past the line end still selects the word before it.
        assert_eq!(word_range("x", 9), (0, 1));
    }

    #[test]
    fn split_rows_answer_for_their_half() {
        let half = |text: &str| Half {
            no:         Some(1),
            kind:       '-',
            text:       text.to_string(),
            line_index: 0,
            global:     0,
        };
        let row = DiffRow::Split {
            left:       Some(half("old")),
            right:      None,
            hunk_index: 0,
        };
        assert_eq!(sel_text(Some(&row), false), "old");
        assert_eq!(sel_text(Some(&row), true), "");
        let hunk = DiffRow::Hunk {
            title:      "@@ -1,1 +1,1 @@".to_string(),
            hunk_index: 0,
        };
        assert_eq!(sel_text(Some(&hunk), true), "@@ -1,1 +1,1 @@");
        assert_eq!(sel_text(None, false), "");
    }
}
