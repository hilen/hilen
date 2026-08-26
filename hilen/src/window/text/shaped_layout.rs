use std::hash::{Hash, Hasher};

use rustybuzz::{Face, UnicodeBuffer, shape};
use wgpu_text::glyph_brush::{
    GlyphPositioner, HorizontalAlign, SectionGeometry, SectionGlyph, ToSectionText,
    ab_glyph::{Font, Glyph, GlyphId, PxScale, Rect, ScaleFont, point},
};

use crate::{deps::refs::main_lock::MainLock, gm::LossyConvert, window::text::ShapeCache};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum VerticalAlign {
    Top,
    Center,
}

/// Per label shaping parameters, collected where the label is drawn.
#[derive(Clone, Copy)]
pub(crate) struct ShapedParams {
    /// Extra pixels added to every glyph advance, `CoreText` style tracking.
    pub tracking:  f32,
    pub multiline: bool,
    pub h_align:   HorizontalAlign,
    pub v_align:   VerticalAlign,
}

/// Positions glyphs with real shaping through rustybuzz, so GPOS kerning
/// and font variations apply like they do in CoreText and browsers. The
/// builtin `glyph_brush` layout only reads the legacy kern table, which
/// modern fonts like SF Pro do not have.
pub(crate) struct ShapedLayout<'a> {
    pub face:      &'a Face<'static>,
    pub font_name: &'a str,
    pub cache:     &'a MainLock<ShapeCache>,
    pub params:    ShapedParams,
}

impl Hash for ShapedLayout<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.font_name.hash(state);
        self.params.tracking.to_bits().hash(state);
        self.params.multiline.hash(state);
        (self.params.h_align as u8).hash(state);
        self.params.v_align.hash(state);
    }
}

#[derive(Clone)]
pub(crate) struct ShapedGlyph {
    id:        u16,
    /// Byte index into the whole text once `shape_line` applies the line
    /// offset. Relative to the line start inside the shape cache.
    cluster:   usize,
    x_advance: f32,
    x_offset:  f32,
    y_offset:  f32,
}

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
/// shaped at. What the caret and the tap to caret mapping read.
pub(crate) struct TextLayout {
    pub lines:       Vec<TextLine>,
    pub ascent:      f32,
    pub descent:     f32,
    pub line_height: f32,
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
    pub(crate) fn nearest_on_line(&self, line: usize, x: f32) -> usize {
        let line = &self.lines[line];
        line.boundaries
            .iter()
            .min_by(|(_, a), (_, b)| (a - x).abs().total_cmp(&(b - x).abs()))
            .map_or(line.start, |(byte, _)| *byte)
    }

    /// The x offset of `byte` on `line`, the line end for a byte past it.
    pub(crate) fn x_on_line(&self, line: usize, byte: usize) -> f32 {
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

struct ShapedLine {
    start:  usize,
    end:    usize,
    glyphs: Vec<ShapedGlyph>,
}

impl ShapedLayout<'_> {
    fn shape_line(&self, line: &str, offset: usize, px_per_unit: f32) -> Vec<ShapedGlyph> {
        let mut glyphs = self.cache.get_mut().get_or_shape(line, px_per_unit, self.params.tracking, || {
            let mut buffer = UnicodeBuffer::new();
            buffer.push_str(line);

            let shaped = shape(self.face, &[], buffer);

            shaped
                .glyph_infos()
                .iter()
                .zip(shaped.glyph_positions())
                .map(|(info, pos)| ShapedGlyph {
                    id:        u16::try_from(info.glyph_id).unwrap_or_default(),
                    cluster:   info.cluster as usize,
                    x_advance: pos.x_advance.lossy_convert() * px_per_unit + self.params.tracking,
                    x_offset:  pos.x_offset.lossy_convert() * px_per_unit,
                    y_offset:  pos.y_offset.lossy_convert() * px_per_unit,
                })
                .collect()
        });

        for glyph in &mut glyphs {
            glyph.cluster += offset;
        }

        glyphs
    }

    /// Greedy wrap at space glyphs. A line that has no space to break at
    /// overflows, same as the builtin layout.
    fn wrap(line: Vec<ShapedGlyph>, text: &str, max_width: f32) -> Vec<Vec<ShapedGlyph>> {
        let is_space = |glyph: &ShapedGlyph| text.as_bytes().get(glyph.cluster) == Some(&b' ');

        let mut lines = vec![];
        let mut current: Vec<ShapedGlyph> = vec![];
        let mut width = 0.0;
        let mut last_space: Option<usize> = None;

        for glyph in line {
            if width + glyph.x_advance > max_width
                && let Some(space) = last_space
            {
                let mut rest = current.split_off(space);
                // The space itself dies with the break.
                rest.remove(0);
                lines.push(current);
                width = rest.iter().map(|g| g.x_advance).sum();
                current = rest;
                last_space = None;
            }

            if is_space(&glyph) {
                last_space = Some(current.len());
            }

            width += glyph.x_advance;
            current.push(glyph);
        }

        lines.push(current);
        lines
    }

    fn shape_text(&self, text: &str, px_per_unit: f32, bound_w: f32) -> Vec<ShapedLine> {
        let mut lines = vec![];
        let mut offset = 0;

        for raw_line in text.split('\n') {
            let shaped = self.shape_line(raw_line, offset, px_per_unit);
            let line_end = offset + raw_line.len();

            let chunks = if self.params.multiline {
                Self::wrap(shaped, text, bound_w)
            } else {
                vec![shaped]
            };

            let count = chunks.len();
            for (index, glyphs) in chunks.into_iter().enumerate() {
                let start = glyphs.first().map_or(offset, |g| g.cluster);
                lines.push(ShapedLine {
                    start,
                    end: 0,
                    glyphs,
                });
                if index + 1 == count {
                    lines.last_mut().expect("just pushed").end = line_end;
                }
            }

            offset = line_end + 1;
        }

        // A wrapped chunk ends at the space the break removed, which is one
        // byte before the next chunk starts.
        for index in 0..lines.len().saturating_sub(1) {
            if lines[index].end == 0 {
                lines[index].end = lines[index + 1].start.saturating_sub(1);
            }
        }

        lines
    }

    fn first_baseline<S: ScaleFont<F>, F: Font>(&self, scaled: &S, screen_y: f32, line_count: usize) -> f32 {
        let line_height = scaled.ascent() - scaled.descent() + scaled.line_gap();
        let count: f32 = line_count.lossy_convert();
        let total_height = count * line_height - scaled.line_gap();

        match self.params.v_align {
            VerticalAlign::Top => screen_y + scaled.ascent(),
            VerticalAlign::Center => screen_y - total_height / 2.0 + scaled.ascent(),
        }
    }

    pub(crate) fn text_layout<F: Font>(
        &self,
        font: &F,
        scale: PxScale,
        text: &str,
        bound_w: f32,
    ) -> TextLayout {
        let scaled = font.as_scaled(scale);
        let px_per_unit = scaled.scale_factor().horizontal;

        let lines = self
            .shape_text(text, px_per_unit, bound_w)
            .into_iter()
            .map(|line| {
                let mut boundaries = vec![];
                let mut x = 0.0;
                for glyph in &line.glyphs {
                    if boundaries.last().is_none_or(|(byte, _)| *byte != glyph.cluster) {
                        boundaries.push((glyph.cluster, x));
                    }
                    x += glyph.x_advance;
                }
                boundaries.push((line.end, x));
                TextLine {
                    start: line.start,
                    end: line.end,
                    boundaries,
                    width: x,
                }
            })
            .collect();

        TextLayout {
            lines,
            ascent: scaled.ascent(),
            descent: scaled.descent(),
            line_height: scaled.ascent() - scaled.descent() + scaled.line_gap(),
        }
    }
}

impl GlyphPositioner for ShapedLayout<'_> {
    fn calculate_glyphs<F, S>(
        &self,
        fonts: &[F],
        geometry: &SectionGeometry,
        sections: &[S],
    ) -> Vec<SectionGlyph>
    where
        F: Font,
        S: ToSectionText,
    {
        let (screen_x, screen_y) = geometry.screen_position;
        let (bound_w, _) = geometry.bounds;

        let mut result = vec![];

        let Some(first) = sections.first() else {
            return result;
        };

        // Every text of a section is a slice of one string, split only
        // where the color changes. Shaped as one string, so kerning across
        // a split is what the font says, then each glyph goes back to the
        // text its cluster came from.
        let sections: Vec<_> = sections.iter().map(ToSectionText::to_section_text).collect();
        let text: String = sections.iter().map(|section| section.text).collect();

        let mut starts = Vec::with_capacity(sections.len());
        let mut start = 0;
        for section in &sections {
            starts.push(start);
            start += section.text.len();
        }

        let first = first.to_section_text();
        let font = &fonts[first.font_id.0];
        let scaled = font.as_scaled(first.scale);

        // The same factor ab_glyph rasterizes with, keeps shaped
        // advances and drawn outlines consistent.
        let px_per_unit = scaled.scale_factor().horizontal;

        let lines = self.shape_text(&text, px_per_unit, bound_w);

        let line_height = scaled.ascent() - scaled.descent() + scaled.line_gap();
        let mut baseline = self.first_baseline(&scaled, screen_y, lines.len());

        for line in lines {
            let line_width: f32 = line.glyphs.iter().map(|g| g.x_advance).sum();

            let mut x = match self.params.h_align {
                HorizontalAlign::Left => screen_x,
                HorizontalAlign::Center => screen_x - line_width / 2.0,
                HorizontalAlign::Right => screen_x - line_width,
            };

            for glyph in line.glyphs {
                let section_index = starts.partition_point(|start| *start <= glyph.cluster).saturating_sub(1);

                result.push(SectionGlyph {
                    section_index,
                    byte_index: glyph.cluster - starts[section_index],
                    glyph: Glyph {
                        id:       GlyphId(glyph.id),
                        scale:    first.scale,
                        position: point(x + glyph.x_offset, baseline - glyph.y_offset),
                    },
                    font_id: first.font_id,
                });
                x += glyph.x_advance;
            }

            baseline += line_height;
        }

        result
    }

    fn bounds_rect(&self, geometry: &SectionGeometry) -> Rect {
        let (x, y) = geometry.screen_position;
        let (w, h) = geometry.bounds;

        let (min_x, max_x) = match self.params.h_align {
            HorizontalAlign::Left => (x, x + w),
            HorizontalAlign::Center => (x - w / 2.0, x + w / 2.0),
            HorizontalAlign::Right => (x - w, x),
        };

        let (min_y, max_y) = match self.params.v_align {
            VerticalAlign::Top => (y, y + h),
            VerticalAlign::Center => (y - h / 2.0, y + h / 2.0),
        };

        Rect {
            min: point(min_x, min_y),
            max: point(max_x, max_y),
        }
    }
}
