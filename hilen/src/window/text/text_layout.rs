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
}

/// Where the glyphs of a text land, in the pixels of the size it was
/// shaped at. What the caret and the tap to caret mapping read. Public
/// so a view can map a click to a byte of text a `Label` draws, the way
/// a diff panel selects code, see `Label::text_layout_for`.
pub struct TextLayout {
    pub(crate) lines:     Vec<TextLine>,
    pub ascent:           f32,
    pub descent:          f32,
    pub line_height:      f32,
    /// The base font's underline: how far the line sits above the
    /// baseline, negative below it, and how thick it is.
    pub(crate) underline: (f32, f32),
    /// The base font's strikeout line, the same 2 numbers.
    pub(crate) strikeout: (f32, f32),
}

impl TextLayout {
    pub(crate) fn total_height(&self) -> f32 {
        let count: f32 = self.lines.len().lossy_convert();
        count * self.line_height - (self.line_height - self.ascent + self.descent)
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

    /// The top of `line` from the top of the text.
    pub fn line_top(&self, line: usize) -> f32 {
        let line: f32 = line.lossy_convert();
        line * self.line_height
    }

    /// The line at `y` from the top of the text, clamped to the first
    /// and last line so a position outside the text lands on an edge.
    pub fn line_at_y(&self, y: f32) -> usize {
        let line: usize = (y / self.line_height).max(0.0).lossy_convert();
        line.min(self.lines.len().saturating_sub(1))
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
        let line = |start, end, width| TextLine {
            start,
            end,
            boundaries: vec![(start, 0.0), (end, width)],
            width,
        };
        TextLayout {
            lines:       vec![line(0, 5, 50.0), line(5, 9, 40.0)],
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
}
