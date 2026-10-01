//! The parts of a label a media screen needs on top of plain text: a limit
//! of lines with an ellipsis on the last one, and an outline or a shadow
//! behind the glyphs that keeps text readable over a picture.

use zeroize::Zeroizing;

use crate::{
    deps::refs::weak_from_ref,
    gm::{ToF32, color::Color, flat::Point},
    ui::Label,
    window::TextLayout,
    wipe::joined,
};

/// A ring of this color around every glyph, `width` points out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TextOutline {
    pub color: Color,
    pub width: f32,
}

/// A copy of the glyphs in this color, moved by `offset` points and blurred
/// with a radius of `blur` points, 0 for a hard copy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TextShadow {
    pub color:  Color,
    pub offset: Point,
    pub blur:   f32,
}

impl Label {
    /// A multiline label shows at most this many lines, and when the text
    /// needs more the last one ends in an ellipsis, the CSS `line-clamp`.
    /// 0 is no limit. A single line label uses `set_ellipsize`.
    pub fn set_max_lines(&self, lines: usize) -> &Self {
        let mut this = weak_from_ref(self);
        this.max_lines = lines;
        this.ellipsized = None;
        self
    }

    pub fn max_lines(&self) -> usize {
        self.max_lines
    }

    /// Draws a ring of `color` around the glyphs, `width` points out, so
    /// the text reads over a bright or a dark picture alike. A width of 0
    /// takes it off. The widest ring is 12 pixels on the screen, a wider
    /// one is drawn at that.
    pub fn set_text_outline(&self, color: impl Into<Color>, width: impl ToF32) -> &Self {
        let width = width.to_f32();
        weak_from_ref(self).text_outline = (width > 0.0).then(|| TextOutline {
            color: color.into(),
            width,
        });
        self
    }

    /// Draws a copy of the glyphs in `color` behind the text, moved by
    /// `offset` points. A clear color takes it off.
    pub fn set_text_shadow(&self, color: impl Into<Color>, offset: impl Into<Point>) -> &Self {
        let color = color.into();
        let mut this = weak_from_ref(self);
        this.text_shadow = (color.a > 0.0).then(|| TextShadow {
            color,
            offset: offset.into(),
            blur: this.text_shadow_blur,
        });
        self
    }

    /// Makes the shadow soft, a blur with a radius of `radius` points like
    /// the blur of a CSS text shadow. 0 is the hard shadow, the default.
    /// It is kept when the shadow is set again. The widest blur is 12
    /// pixels on the screen.
    pub fn set_text_shadow_blur(&self, radius: impl ToF32) -> &Self {
        let blur = radius.to_f32().max(0.0);
        let mut this = weak_from_ref(self);
        this.text_shadow_blur = blur;
        if let Some(shadow) = &mut this.text_shadow {
            shadow.blur = blur;
        }
        self
    }

    pub(crate) fn text_outline(&self) -> Option<TextOutline> {
        self.text_outline
    }

    pub(crate) fn text_shadow(&self) -> Option<TextShadow> {
        self.text_shadow
    }

    /// The label wraps and has a line limit.
    pub(super) fn limits_lines(&self) -> bool {
        self.is_multiline() && self.max_lines > 0
    }

    /// The layout of `text` wrapped at a frame `width`.
    fn layout_at(&self, text: &str, width: f32) -> TextLayout {
        self.font().text_layout(
            text,
            self.text_size(),
            Some(width - self.text_inset()),
            self.shaping(text),
        )
    }

    /// The longest start of the text that, with an ellipsis after it, wraps
    /// into the line limit at this frame width. None when the whole text
    /// already does.
    pub(super) fn truncate_to_lines(&self, width: f32) -> Option<Zeroizing<String>> {
        const ELLIPSIS: &str = "…";

        let fits = |text: &str| self.layout_at(text, width).line_count() <= self.max_lines;
        if fits(&self.text) {
            return None;
        }

        // The byte each kept character count ends on.
        let ends: Vec<usize> = self.text.char_indices().map(|(index, _)| index).collect();
        // The spaces before the cut go, an ellipsis after a space reads as
        // a stray word.
        let candidate = |kept: usize| joined(&[self.text[..ends[kept]].trim_end(), ELLIPSIS]);

        // The longest fitting cut by binary search. When even the bare
        // ellipsis does not fit it still draws, like CSS does.
        let mut best = 0;
        let (mut low, mut high) = (0, ends.len() - 1);
        while low <= high {
            let mid = usize::midpoint(low, high);
            if fits(&candidate(mid)) {
                best = mid;
                low = mid + 1;
            } else if mid == 0 {
                break;
            } else {
                high = mid - 1;
            }
        }

        Some(candidate(best))
    }
}
