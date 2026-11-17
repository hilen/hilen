use std::ops::Range;

use crate::gm::LossyConvert;

/// One laid out line: the byte range of the text it shows and where
/// every caret position on it sits.
pub(crate) struct TextLine {
    pub start:      usize,
    pub end:        usize,
    /// Byte index and x offset from the line start of every position a
    /// caret can take, the line end included. Ligatures give one entry
    /// per cluster.
    pub boundaries: Vec<(usize, f32)>,
    pub width:      f32,
    /// The top of the box of the line from the top of the text, the
    /// growth of the lines before it included.
    pub top:        f32,
    /// How much the box of the line grew above and below for a tall room
    /// on it, see `Room::reach`. Its text sits under the growth above.
    pub grow:       (f32, f32),
}

/// Where the glyphs of a text land, in the pixels of the size it was
/// shaped at. What the caret and the tap to caret mapping read. Public
/// so a view can map a click to a byte of text a `Label` draws, the way
/// a diff panel selects code, see `Label::text_layout_for`.
pub struct TextLayout {
    pub(crate) lines: Vec<TextLine>,
    pub ascent:       f32,
    pub descent:      f32,
    pub line_height:  f32,
    /// The base font's underline: how far the line sits above the
    /// baseline, negative below it, and how thick it is.
    pub underline:    (f32, f32),
    /// The base font's strikeout line, the same 2 numbers.
    pub strikeout:    (f32, f32),
}

impl TextLayout {
    pub(crate) fn total_height(&self) -> f32 {
        let count: f32 = self.lines.len().lossy_convert();
        let grown: f32 = self.lines.iter().map(|line| line.grow.0 + line.grow.1).sum();
        count * self.line_height - (self.line_height - self.ascent + self.descent) + grown
    }

    /// The line that shows `byte`, the last line for a byte past the text.
    pub fn line_of(&self, byte: usize) -> usize {
        self.lines
            .iter()
            .position(|line| byte <= line.end)
            .unwrap_or(self.lines.len().saturating_sub(1))
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// The byte range of the text on `line`.
    pub fn line_range(&self, line: usize) -> Range<usize> {
        let line = &self.lines[line];
        line.start..line.end
    }

    /// The top of the text of `line` from the top of the text. Its glyphs
    /// sit in 1 line height from there, also in a line whose box grew for
    /// a tall room, see `line_box_top`.
    pub fn line_top(&self, line: usize) -> f32 {
        self.lines.get(line).map_or_else(
            || {
                let line: f32 = line.lossy_convert();
                line * self.line_height
            },
            |line| line.top + line.grow.0,
        )
    }

    /// The top of the box of `line` from the top of the text, above the
    /// growth of the line for a tall room. The same as `line_top` for a
    /// line that did not grow.
    pub fn line_box_top(&self, line: usize) -> f32 {
        self.lines.get(line).map_or_else(|| self.line_top(line), |line| line.top)
    }

    /// The height of the box of `line`, the line height and the growth
    /// for a tall room on it.
    pub fn line_box_height(&self, line: usize) -> f32 {
        self.line_height + self.lines.get(line).map_or(0.0, |line| line.grow.0 + line.grow.1)
    }

    /// The bottom of the box of the last line from the top of the text.
    pub fn lines_bottom(&self) -> f32 {
        let last = self.lines.len().saturating_sub(1);
        if self.lines.is_empty() {
            0.0
        } else {
            self.line_box_top(last) + self.line_box_height(last)
        }
    }

    /// The line at `y` from the top of the text, clamped to the first
    /// and last line so a position outside the text lands on an edge.
    pub fn line_at_y(&self, y: f32) -> usize {
        self.lines.iter().rposition(|line| line.top <= y).unwrap_or(0)
    }

    /// The caret position closest to `x` on `line`.
    pub fn nearest_on_line(&self, line: usize, x: f32) -> usize {
        let line = &self.lines[line];
        line.boundaries
            .iter()
            .min_by(|(_, a), (_, b)| (a - x).abs().total_cmp(&(b - x).abs()))
            .map_or(line.start, |(byte, _)| *byte)
    }

    /// The x offset of `byte` on `line`, the line end for a byte past it.
    pub fn x_on_line(&self, line: usize, byte: usize) -> f32 {
        let line = &self.lines[line];
        line.boundaries
            .iter()
            .find(|(b, _)| *b >= byte)
            .or(line.boundaries.last())
            .map_or(0.0, |(_, x)| *x)
    }

    /// Where the caret sits for `byte`, x from the line start, and the
    /// line index.
    pub(crate) fn position_of(&self, byte: usize) -> (usize, f32) {
        let index = self.line_of(byte);
        let line = &self.lines[index];
        let x = line
            .boundaries
            .iter()
            .find(|(b, _)| *b >= byte)
            .or(line.boundaries.last())
            .map_or(0.0, |(_, x)| *x);
        (index, x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> TextLayout {
        let line = |start, end, width, top| TextLine {
            start,
            end,
            boundaries: vec![(start, 0.0), (end, width)],
            width,
            top,
            grow: (0.0, 0.0),
        };
        TextLayout {
            lines:       vec![line(0, 5, 50.0, 0.0), line(5, 9, 40.0, 12.0)],
            ascent:      8.0,
            descent:     2.0,
            line_height: 12.0,
            underline:   (0.0, 1.0),
            strikeout:   (3.0, 1.0),
        }
    }

    #[test]
    fn lines_by_index() {
        let layout = layout();
        assert_eq!(layout.line_count(), 2);
        assert_eq!(layout.line_range(1), 5..9);
        assert!((layout.line_top(1) - 12.0).abs() < f32::EPSILON);
        assert_eq!(layout.line_of(3), 0);
        assert_eq!(layout.line_of(7), 1);
        assert_eq!(layout.line_of(50), 1);
    }

    #[test]
    fn line_at_y_clamps_to_the_text() {
        let layout = layout();
        assert_eq!(layout.line_at_y(-3.0), 0);
        assert_eq!(layout.line_at_y(11.9), 0);
        assert_eq!(layout.line_at_y(12.0), 1);
        assert_eq!(layout.line_at_y(100.0), 1);
    }

    #[test]
    fn a_grown_line_moves_the_lines_under_it() {
        let mut layout = layout();
        // The first line grew 5 above and 3 below for a tall room, so
        // the second one starts 8 lower.
        layout.lines[0].grow = (5.0, 3.0);
        layout.lines[1].top = 20.0;
        let near = |a: f32, b: f32| (a - b).abs() < f32::EPSILON;

        assert!(near(layout.line_box_top(0), 0.0));
        assert!(near(layout.line_top(0), 5.0));
        assert!(near(layout.line_box_height(0), 20.0));
        assert!(near(layout.line_top(1), 20.0));
        assert!(near(layout.line_box_height(1), 12.0));
        assert!(near(layout.lines_bottom(), 32.0));
        assert!(near(layout.total_height(), 26.0));
        assert_eq!(layout.line_at_y(19.9), 0);
        assert_eq!(layout.line_at_y(20.0), 1);
    }
}
