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
}

impl TextLayout {
    pub(crate) fn total_height(&self) -> f32 {
        let count: f32 = self.lines.len().lossy_convert();
        count * self.line_height - (self.line_height - self.ascent + self.descent)
    }

    pub(crate) fn line_of(&self, byte: usize) -> usize {
        self.lines
            .iter()
            .position(|line| byte <= line.end)
            .unwrap_or(self.lines.len().saturating_sub(1))
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
