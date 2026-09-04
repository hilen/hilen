use std::ops::Range;

use crate::{
    deps::refs::{Weak, weak_from_ref},
    gm::color::Color,
    ui::Label,
    window::{Font, FontRun, runs_with_fallbacks},
};

/// The look of one byte range of a label's text beyond its color. A
/// range without a font keeps the label's font.
#[derive(Clone, Default)]
pub struct RunStyle {
    pub font:      Option<Weak<Font>>,
    pub underline: bool,
}

impl RunStyle {
    pub fn font(font: Weak<Font>) -> Self {
        Self {
            font:      Some(font),
            underline: false,
        }
    }

    pub fn underline() -> Self {
        Self {
            font:      None,
            underline: true,
        }
    }

    pub fn underlined(mut self) -> Self {
        self.underline = true;
        self
    }
}

/// One styled byte range of a label's text.
pub(crate) struct StyleRun {
    pub range: Range<usize>,
    pub style: RunStyle,
}

impl Label {
    /// Draws byte ranges of the text in their own font, underlined, or
    /// both, the rest keeps the label's font. The line height and the
    /// baseline stay the label font's. Shaping, wrapping and measuring
    /// use the run fonts, so a wider font moves the line breaks. Ranges
    /// are clamped to the text and to char boundaries, sorted, and a
    /// later range wins an overlap, like `set_color_runs`.
    pub fn set_font_runs(&self, runs: impl IntoIterator<Item = (Range<usize>, RunStyle)>) -> &Self {
        let mut this = weak_from_ref(self);

        let clamp = |position: usize| {
            let mut position = position.min(this.text.len());
            while !this.text.is_char_boundary(position) {
                position -= 1;
            }
            position
        };

        let mut all: Vec<StyleRun> = runs
            .into_iter()
            .map(|(range, style)| StyleRun {
                range: clamp(range.start)..clamp(range.end),
                style,
            })
            .filter(|run| run.range.start < run.range.end)
            .collect();

        all.sort_by_key(|run| run.range.start);

        // A later range wins, so the earlier one gives up the overlap.
        let mut runs: Vec<StyleRun> = vec![];
        for run in all {
            if let Some(last) = runs.last_mut()
                && last.range.end > run.range.start
            {
                last.range.end = run.range.start;
                if last.range.start >= last.range.end {
                    runs.pop();
                }
            }
            runs.push(run);
        }

        this.font_runs = runs;
        this.ellipsized = None;
        self
    }

    pub fn clear_font_runs(&self) -> &Self {
        let mut this = weak_from_ref(self);
        this.font_runs.clear();
        this.ellipsized = None;
        self
    }

    pub fn font_runs_len(&self) -> usize {
        self.font_runs.len()
    }

    /// The runs that shape with their own font, clamped to `text`, which
    /// is shorter than the full text when it is the ellipsized copy.
    /// Chars the effective font misses get fallback runs on top.
    pub(crate) fn shaping_runs(&self, text: &str) -> Vec<FontRun> {
        let explicit = self
            .font_runs
            .iter()
            .filter_map(|run| {
                let font = run.style.font?;
                let range = Self::clamp_to(text, &run.range);
                (range.start < range.end).then_some(FontRun { range, font })
            })
            .collect();
        runs_with_fallbacks(text, self.font(), explicit)
    }

    /// Whether any font this label may draw with has color glyphs, the
    /// base, an explicit run font or a registered fallback. The drawer
    /// skips the color glyph pass for every other label.
    pub(crate) fn uses_color_font(&self) -> bool {
        self.font().has_color()
            || self
                .font_runs
                .iter()
                .any(|run| run.style.font.is_some_and(|font| font.has_color()))
            || Font::fallbacks().iter().any(|font| font.has_color())
    }

    /// The underlined byte ranges, clamped like `shaping_runs`.
    pub(crate) fn underline_runs(&self, text: &str) -> Vec<Range<usize>> {
        self.font_runs
            .iter()
            .filter(|run| run.style.underline)
            .map(|run| Self::clamp_to(text, &run.range))
            .filter(|range| range.start < range.end)
            .collect()
    }

    /// The color the text has at `byte`, the run's when one covers it.
    pub(crate) fn color_at(&self, byte: usize) -> Color {
        self.color_runs()
            .iter()
            .find(|run| run.range.contains(&byte))
            .map_or(*self.text_color(), |run| run.color)
    }

    fn clamp_to(text: &str, range: &Range<usize>) -> Range<usize> {
        let clamp = |position: usize| {
            let mut position = position.min(text.len());
            while !text.is_char_boundary(position) {
                position -= 1;
            }
            position
        };
        clamp(range.start)..clamp(range.end)
    }
}
