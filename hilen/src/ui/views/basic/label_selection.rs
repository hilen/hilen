//! The selectable side of a label: the flag, the byte under a point and
//! the rects of a selected range. The selection itself, which views it
//! goes over and what is copied, lives in `ui/text_selection`.

use std::ops::Range;

use crate::{
    deps::refs::weak_from_ref,
    gm::{
        LossyConvert,
        flat::{Point, Rect},
    },
    ui::{Label, TextAlignment, VerticalAlignment, ViewFrame, ViewTouch},
    window::TextLayout,
};

const ELLIPSIS: &str = "…";

/// How wide the mark of a selected line break is, in line heights. A text
/// editor shows it at the end of every line the selection goes on from.
const LINE_BREAK_WIDTH: f32 = 0.3;

/// How the drawn text differs from the full text of a label. The bytes
/// of a selection are bytes of the full text, the layout knows only the
/// drawn one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cut {
    None,
    /// The first bytes are drawn, then an ellipsis.
    Tail(usize),
    /// An ellipsis is drawn, then the text from this byte on.
    Head(usize),
}

impl Cut {
    fn of(full: &str, drawn: &str) -> Self {
        if full == drawn {
            return Self::None;
        }
        if let Some(kept) = drawn.strip_suffix(ELLIPSIS)
            && full.starts_with(kept)
        {
            return Self::Tail(kept.len());
        }
        if let Some(kept) = drawn.strip_prefix(ELLIPSIS)
            && full.ends_with(kept)
        {
            return Self::Head(full.len() - kept.len());
        }
        Self::None
    }

    /// A byte of the drawn text as a byte of the full text. The ellipsis
    /// stands for all the text that is cut.
    fn full_byte(self, full_len: usize, drawn: usize) -> usize {
        match self {
            Self::None => drawn.min(full_len),
            Self::Tail(kept) if drawn <= kept => drawn,
            Self::Tail(_) => full_len,
            Self::Head(_) if drawn < ELLIPSIS.len() => 0,
            Self::Head(start) => (start + drawn - ELLIPSIS.len()).min(full_len),
        }
    }

    /// The inverse of `full_byte`.
    fn drawn_byte(self, drawn_len: usize, full: usize) -> usize {
        match self {
            Self::None => full.min(drawn_len),
            Self::Tail(kept) if full <= kept => full,
            Self::Tail(_) => drawn_len,
            Self::Head(start) if full < start => 0,
            Self::Head(start) => (ELLIPSIS.len() + full - start).min(drawn_len),
        }
    }
}

impl Label {
    /// The text of this label can be selected with the mouse and copied,
    /// like the text of a text editor. Off by default. A drag selects, a
    /// double click takes a word, a triple click a line, Shift with a
    /// click extends, Cmd or Ctrl with C copies. On a touch screen a long
    /// press selects. Inside a `TableView` with `set_text_selectable` the
    /// label is a part of the selection of the whole table.
    ///
    /// A selectable label takes the touches over it. The copy is the full
    /// text, also of a label that draws it cut with an ellipsis. A label
    /// marked with `set_secret` is never selectable.
    pub fn set_selectable(&self, selectable: bool) -> &Self {
        weak_from_ref(self).selectable = selectable;
        if selectable {
            self.enable_touch();
        } else {
            self.disable_touch();
        }
        self
    }

    pub fn is_selectable(&self) -> bool {
        self.selectable && !self.is_secret()
    }

    /// Makes the text a part of the selection of the view that holds the
    /// label, which takes the touches itself.
    pub(crate) fn mark_selectable(&self, selectable: bool) {
        weak_from_ref(self).selectable = selectable;
    }

    /// The top of the first line of `layout` in the points of the label,
    /// the way the drawer places the text.
    pub(crate) fn text_top(&self, layout: &TextLayout) -> f32 {
        match self.vertical_alignment {
            VerticalAlignment::Top => 0.0,
            VerticalAlignment::Center => self.height() / 2.0 - layout.total_height() / 2.0,
        }
    }

    /// Where `line` of `layout` starts in the points of the label.
    pub(crate) fn line_left(&self, layout: &TextLayout, line: usize) -> f32 {
        let width = layout.lines[line].width;
        match self.alignment {
            TextAlignment::Left => self.text_inset(),
            TextAlignment::Center => (self.width() - width) / 2.0,
            TextAlignment::Right => self.width() - self.text_inset() - width,
        }
    }

    /// The byte of the full text whose caret position is closest to
    /// `point`, in the points of the label. A point over or under the
    /// text lands on the first or the last line.
    pub(crate) fn byte_at(&self, point: Point) -> usize {
        let drawn = self.display_text(self.width());
        let layout = self.text_layout_for(drawn);
        if layout.lines.is_empty() {
            return 0;
        }
        let line = layout.line_at_y(point.y - self.text_top(&layout));
        let byte = layout.nearest_on_line(line, point.x - self.line_left(&layout, line));
        Cut::of(&self.text, drawn).full_byte(self.text.len(), byte)
    }

    /// One rect per line of the selected `range` of the full text, in the
    /// points of the label. `goes_on` says the selection goes on after
    /// this label, the last line then shows its line break as selected.
    pub(crate) fn selection_rects(&self, range: Range<usize>, goes_on: bool) -> Vec<Rect> {
        let drawn = self.display_text(self.width());
        let layout = self.text_layout_for(drawn);
        let cut = Cut::of(&self.text, drawn);
        let start = cut.drawn_byte(drawn.len(), range.start);
        let end = cut.drawn_byte(drawn.len(), range.end);

        let top = self.text_top(&layout);
        let last = layout.lines.len().saturating_sub(1);
        let mut rects = Vec::new();

        for (index, line) in layout.lines.iter().enumerate() {
            if start > line.end || end < line.start {
                continue;
            }

            let from = layout.x_on_line(index, start.max(line.start));
            let to = layout.x_on_line(index, end.min(line.end));

            // A line break of the text is 1 byte at the end of its line. A
            // line that only wraps has none, and shows none as selected.
            let breaks = drawn.as_bytes().get(line.end) == Some(&b'\n');
            let over_break = (breaks && end > line.end) || (index == last && goes_on && end >= line.end);
            let width = to - from
                + if over_break {
                    layout.line_height * LINE_BREAK_WIDTH
                } else {
                    0.0
                };

            if width <= 0.0 {
                continue;
            }

            let row: f32 = index.lossy_convert();
            rects.push(Rect::new(
                self.line_left(&layout, index) + from,
                top + row * layout.line_height,
                width,
                layout.line_height,
            ));
        }

        rects
    }
}

#[cfg(test)]
mod tests {
    use super::Cut;

    #[test]
    fn a_text_drawn_whole_maps_byte_for_byte() {
        let cut = Cut::of("whole text", "whole text");
        assert_eq!(cut, Cut::None);
        assert_eq!(cut.full_byte(10, 4), 4);
        assert_eq!(cut.drawn_byte(10, 4), 4);
        assert_eq!(cut.full_byte(10, 40), 10);
    }

    #[test]
    fn the_ellipsis_at_the_end_stands_for_the_cut_text() {
        let full = "a long line of text";
        let drawn = "a long…";
        let cut = Cut::of(full, drawn);
        assert_eq!(cut, Cut::Tail(6));
        assert_eq!(cut.full_byte(full.len(), 3), 3);
        assert_eq!(cut.full_byte(full.len(), 6), 6);
        assert_eq!(cut.full_byte(full.len(), drawn.len()), full.len());
        assert_eq!(cut.drawn_byte(drawn.len(), 6), 6);
        assert_eq!(cut.drawn_byte(drawn.len(), 12), drawn.len());
    }

    #[test]
    fn the_ellipsis_at_the_start_stands_for_the_cut_text() {
        let full = "/a/long/path/file.rs";
        let drawn = "…/file.rs";
        let cut = Cut::of(full, drawn);
        assert_eq!(cut, Cut::Head(12));
        assert_eq!(cut.full_byte(full.len(), 0), 0);
        assert_eq!(cut.full_byte(full.len(), 3), 12);
        assert_eq!(cut.full_byte(full.len(), drawn.len()), full.len());
        assert_eq!(cut.drawn_byte(drawn.len(), 5), 0);
        assert_eq!(cut.drawn_byte(drawn.len(), 12), 3);
        assert_eq!(cut.drawn_byte(drawn.len(), full.len()), drawn.len());
    }
}
