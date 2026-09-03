use std::{
    hash::{Hash, Hasher},
    ops::Range,
};

use rustybuzz::{UnicodeBuffer, shape};
use unicode_linebreak::{BreakClass, break_property, linebreaks};
use wgpu_text::glyph_brush::{
    GlyphPositioner, HorizontalAlign, SectionGeometry, SectionGlyph, ToSectionText,
    ab_glyph::{Font as AbGlyphFont, Glyph, GlyphId, PxScale, Rect, ScaleFont, point},
};

use crate::{
    deps::refs::Weak,
    gm::LossyConvert,
    window::{
        Font,
        text::{TextLayout, TextLine},
    },
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum VerticalAlign {
    Top,
    Center,
}

/// A byte range of the text shaped and drawn with its own font.
#[derive(Clone)]
pub(crate) struct FontRun {
    pub range: Range<usize>,
    pub font:  Weak<Font>,
}

/// Per label shaping parameters, collected where the label is drawn.
#[derive(Clone)]
pub(crate) struct ShapedParams {
    /// Extra pixels added to every glyph advance, `CoreText` style tracking.
    pub tracking:    f32,
    pub multiline:   bool,
    pub h_align:     HorizontalAlign,
    pub v_align:     VerticalAlign,
    /// Pixels between baselines, the CSS line box. `None` keeps the
    /// font's own line height. Glyphs center in each box with half the
    /// leading above and below, the CSS line-height model.
    pub line_height: Option<f32>,
    /// The label's font. Everything outside the runs shapes with it and
    /// its metrics set the line height and the baseline.
    pub base:        Weak<Font>,
    /// Sorted, not overlapping, on char boundaries.
    pub runs:        Vec<FontRun>,
}

/// Positions glyphs with real shaping through rustybuzz, so GPOS kerning
/// and font variations apply like they do in CoreText and browsers. The
/// builtin `glyph_brush` layout only reads the legacy kern table, which
/// modern fonts like SF Pro do not have.
///
/// Every font owns one brush, so a label with font runs is queued on
/// every brush it touches. Each copy lays out the whole text and keeps
/// only the glyphs of the font that brush draws.
pub(crate) struct ShapedLayout<'a> {
    /// The name of the font whose brush receives the glyphs.
    pub emit:   &'a str,
    pub params: ShapedParams,
}

impl Hash for ShapedLayout<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.emit.hash(state);
        self.params.base.name.hash(state);
        self.params.tracking.to_bits().hash(state);
        self.params.multiline.hash(state);
        self.params.line_height.map(f32::to_bits).hash(state);
        (self.params.h_align as u8).hash(state);
        self.params.v_align.hash(state);
        for run in &self.params.runs {
            run.range.hash(state);
            run.font.name.hash(state);
        }
    }
}

#[derive(Clone)]
pub(crate) struct ShapedGlyph {
    id:        u16,
    /// Byte index into the whole text once `shape_segment` applies the
    /// offset. Relative to the segment start inside the shape cache.
    cluster:   usize,
    /// Index into the sources of the layout, the base font first.
    source:    usize,
    x_advance: f32,
    x_offset:  f32,
    y_offset:  f32,
}

/// The space glyph of a source, what a tab draws as and the column a
/// tab stop is measured in.
#[derive(Clone, Copy)]
struct Space {
    id:      u16,
    advance: f32,
}

/// One font of a layout at the size the text draws at.
struct Source {
    font:        Weak<Font>,
    scale:       PxScale,
    px_per_unit: f32,
    space:       Option<Space>,
}

/// Columns of one tab stop, in spaces. The editor default.
const TAB_SIZE: f32 = 4.0;

struct ShapedLine {
    start:  usize,
    end:    usize,
    glyphs: Vec<ShapedGlyph>,
}

/// One wrapped piece of a line. `end` is where its text stops, before
/// the spaces a break removed. The last piece has none, it ends with
/// the line.
struct WrappedChunk {
    glyphs: Vec<ShapedGlyph>,
    end:    Option<usize>,
}

impl ShapedLayout<'_> {
    /// The base font and every run font, at scales that draw the same em
    /// size. `base_scale` is the base font's `PxScale`.
    fn sources(&self, base_scale: PxScale) -> Vec<Source> {
        let base = self.params.base;
        let em = base_scale.x / base.em_scale();

        let tracking = self.params.tracking;
        let source = |font: Weak<Font>| {
            let scale = PxScale::from(em * font.em_scale());
            let px_per_unit = font.ab().as_scaled(scale).scale_factor().horizontal;
            let space = font.face().glyph_index(' ').map(|id| Space {
                id:      id.0,
                advance: f32::from(font.face().glyph_hor_advance(id).unwrap_or_default()) * px_per_unit
                    + tracking,
            });
            Source {
                font,
                scale,
                px_per_unit,
                space,
            }
        };

        let mut sources = vec![source(base)];
        sources.extend(self.params.runs.iter().map(|run| source(run.font)));
        sources
    }

    fn shape_segment(&self, segment: &str, offset: usize, index: usize, source: &Source) -> Vec<ShapedGlyph> {
        let font = source.font;
        let tracking = self.params.tracking;

        let mut glyphs =
            font.shape_cache()
                .get_mut()
                .get_or_shape(segment, source.px_per_unit, tracking, || {
                    let mut buffer = UnicodeBuffer::new();
                    buffer.push_str(segment);

                    let shaped = shape(font.face(), &[], buffer);

                    shaped
                        .glyph_infos()
                        .iter()
                        .zip(shaped.glyph_positions())
                        .map(|(info, pos)| ShapedGlyph {
                            id:        u16::try_from(info.glyph_id).unwrap_or_default(),
                            cluster:   info.cluster as usize,
                            source:    0,
                            x_advance: pos.x_advance.lossy_convert() * source.px_per_unit + tracking,
                            x_offset:  pos.x_offset.lossy_convert() * source.px_per_unit,
                            y_offset:  pos.y_offset.lossy_convert() * source.px_per_unit,
                        })
                        .collect()
                });

        for glyph in &mut glyphs {
            glyph.cluster += offset;
            glyph.source = index;
        }

        glyphs
    }

    /// Shapes the line at `range` of `text`, one segment per font run it
    /// crosses and one per gap between them. A line without runs is one
    /// segment, so kerning inside it is what the font says.
    fn shape_line(&self, text: &str, range: Range<usize>, sources: &[Source]) -> Vec<ShapedGlyph> {
        let mut glyphs = vec![];
        let mut cursor = range.start;

        for (index, run) in self.params.runs.iter().enumerate() {
            let start = run.range.start.max(cursor);
            let stop = run.range.end.min(range.end);
            if start >= stop {
                continue;
            }
            if cursor < start {
                glyphs.extend(self.shape_segment(&text[cursor..start], cursor, 0, &sources[0]));
            }
            glyphs.extend(self.shape_segment(&text[start..stop], start, index + 1, &sources[index + 1]));
            cursor = stop;
        }

        if cursor < range.end {
            glyphs.extend(self.shape_segment(&text[cursor..range.end], cursor, 0, &sources[0]));
        }

        glyphs
    }

    /// Greedy wrap at the Unicode line break opportunities of the line,
    /// UAX 14, so Latin breaks after spaces and Japanese between
    /// characters. Spaces before a break die with it. A Latin word wider
    /// than the bound overflows, the `UIKit` word wrapping behavior. Thai
    /// and the other complex scripts have no opportunity without a
    /// dictionary, so a too wide piece of them breaks at the cluster
    /// that overflows.
    fn wrap(line: Vec<ShapedGlyph>, text: &str, range: Range<usize>, max_width: f32) -> Vec<WrappedChunk> {
        let is_space = |glyph: &ShapedGlyph| text.as_bytes().get(glyph.cluster) == Some(&b' ');
        let is_complex = |glyph: &ShapedGlyph| {
            text[glyph.cluster..]
                .chars()
                .next()
                .is_some_and(|c| break_property(u32::from(c)) == BreakClass::ComplexContext)
        };

        // Byte positions a line may start at, in text coordinates. The
        // mandatory break at the end of the line is not one.
        let opportunities: Vec<usize> = linebreaks(&text[range.clone()])
            .map(|(index, _)| range.start + index)
            .filter(|&index| index < range.end)
            .collect();

        let mut chunks = vec![];
        let mut current: Vec<ShapedGlyph> = vec![];
        let mut width = 0.0;
        let mut last_break: Option<usize> = None;

        for glyph in line {
            // A mark shares the cluster of its base, so only the base
            // glyph may start a piece, never the mark drawn over it.
            let first_of_cluster = current.last().is_none_or(|last| last.cluster != glyph.cluster);
            let starts_piece = first_of_cluster && opportunities.contains(&glyph.cluster);

            if starts_piece && !current.is_empty() {
                last_break = Some(current.len());
            }

            if width + glyph.x_advance > max_width && !current.is_empty() {
                let split = match last_break {
                    Some(index) => Some(index),
                    None if first_of_cluster && is_complex(&glyph) => Some(current.len()),
                    None => None,
                };

                if let Some(split) = split
                    && split > 0
                {
                    let rest = current.split_off(split);
                    let mut end = rest.first().map_or(glyph.cluster, |first| first.cluster);
                    while let Some(last) = current.last()
                        && is_space(last)
                    {
                        end = last.cluster;
                        current.pop();
                    }
                    chunks.push(WrappedChunk {
                        glyphs: current,
                        end:    Some(end),
                    });
                    width = rest.iter().map(|g| g.x_advance).sum();
                    current = rest;
                    last_break = None;
                }
            }

            width += glyph.x_advance;
            current.push(glyph);
        }

        chunks.push(WrappedChunk {
            glyphs: current,
            end:    None,
        });
        chunks
    }

    /// A tab is not in most cmaps, so the shaper hands back notdef and
    /// the line shows a box. Draws it as the space glyph and stretches it
    /// to the next `TAB_SIZE` column, so tabs align like in an editor.
    fn expand_tabs(glyphs: &mut [ShapedGlyph], text: &str, sources: &[Source]) {
        let mut x = 0.0;
        for glyph in glyphs {
            if text.as_bytes().get(glyph.cluster) == Some(&b'\t')
                && let Some(space) = sources[glyph.source].space
            {
                let stop = space.advance * TAB_SIZE;
                let mut advance = stop - x % stop;
                // Rounding after a run of spaces can leave x a hair short
                // of a stop, a tab still has to move a visible amount.
                if advance < 0.5 {
                    advance += stop;
                }
                glyph.id = space.id;
                glyph.x_offset = 0.0;
                glyph.y_offset = 0.0;
                glyph.x_advance = advance;
            }
            x += glyph.x_advance;
        }
    }

    fn shape_text(&self, text: &str, sources: &[Source], bound_w: f32) -> Vec<ShapedLine> {
        let mut lines = vec![];
        let mut offset = 0;

        for raw_line in text.split('\n') {
            let line_end = offset + raw_line.len();
            let mut shaped = self.shape_line(text, offset..line_end, sources);
            Self::expand_tabs(&mut shaped, text, sources);

            let chunks = if self.params.multiline {
                Self::wrap(shaped, text, offset..line_end, bound_w)
            } else {
                vec![WrappedChunk {
                    glyphs: shaped,
                    end:    None,
                }]
            };

            for chunk in chunks {
                let start = chunk.glyphs.first().map_or(offset, |g| g.cluster);
                lines.push(ShapedLine {
                    start,
                    end: chunk.end.unwrap_or(line_end),
                    glyphs: chunk.glyphs,
                });
            }

            offset = line_end + 1;
        }

        lines
    }

    /// The baseline pitch: the custom line box when one is set, the
    /// font's own line height otherwise.
    fn line_pitch<S: ScaleFont<F>, F: AbGlyphFont>(&self, scaled: &S) -> f32 {
        self.params
            .line_height
            .unwrap_or_else(|| scaled.ascent() - scaled.descent() + scaled.line_gap())
    }

    fn first_baseline<S: ScaleFont<F>, F: AbGlyphFont>(
        &self,
        scaled: &S,
        screen_y: f32,
        line_count: usize,
    ) -> f32 {
        let count: f32 = line_count.lossy_convert();

        // A custom line box centers the glyphs in each box with half
        // the leading above and below, the CSS line-height model.
        if let Some(line_height) = self.params.line_height {
            let leading = line_height - (scaled.ascent() - scaled.descent());
            let first = leading / 2.0 + scaled.ascent();
            return match self.params.v_align {
                VerticalAlign::Top => screen_y + first,
                VerticalAlign::Center => screen_y - count * line_height / 2.0 + first,
            };
        }

        let line_height = scaled.ascent() - scaled.descent() + scaled.line_gap();
        let total_height = count * line_height - scaled.line_gap();

        match self.params.v_align {
            VerticalAlign::Top => screen_y + scaled.ascent(),
            VerticalAlign::Center => screen_y - total_height / 2.0 + scaled.ascent(),
        }
    }

    /// Lines and caret positions of `text` at the base font's `scale`.
    pub(crate) fn text_layout(&self, scale: PxScale, text: &str, bound_w: f32) -> TextLayout {
        let base = self.params.base;
        let scaled = base.ab().as_scaled(scale);
        let sources = self.sources(scale);

        let lines = self
            .shape_text(text, &sources, bound_w)
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

        let px_per_unit = sources[0].px_per_unit;
        let underline = base.face().underline_metrics().map_or(
            (scaled.descent() / 2.0, scaled.ascent() * 0.05),
            |metrics| {
                (
                    f32::from(metrics.position) * px_per_unit,
                    f32::from(metrics.thickness) * px_per_unit,
                )
            },
        );

        TextLayout {
            lines,
            ascent: scaled.ascent(),
            descent: scaled.descent(),
            line_height: self.line_pitch(&scaled),
            underline,
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
        F: AbGlyphFont,
        S: ToSectionText,
    {
        // Every glyph goes to font id 0, the one font the brush owns.
        debug_assert_eq!(fonts.len(), 1, "a brush owns exactly one font");

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
        let base = self.params.base;
        let scaled = base.ab().as_scaled(first.scale);

        // The same factors ab_glyph rasterizes with, keep shaped advances
        // and drawn outlines consistent.
        let sources = self.sources(first.scale);

        let lines = self.shape_text(&text, &sources, bound_w);

        let line_height = self.line_pitch(&scaled);
        let mut baseline = self.first_baseline(&scaled, screen_y, lines.len());

        for line in lines {
            let line_width: f32 = line.glyphs.iter().map(|g| g.x_advance).sum();

            let mut x = match self.params.h_align {
                HorizontalAlign::Left => screen_x,
                HorizontalAlign::Center => screen_x - line_width / 2.0,
                HorizontalAlign::Right => screen_x - line_width,
            };

            for glyph in line.glyphs {
                let source = &sources[glyph.source];
                if source.font.name != self.emit {
                    x += glyph.x_advance;
                    continue;
                }

                let section_index = starts.partition_point(|start| *start <= glyph.cluster).saturating_sub(1);

                result.push(SectionGlyph {
                    section_index,
                    byte_index: glyph.cluster - starts[section_index],
                    glyph: Glyph {
                        id:       GlyphId(glyph.id),
                        scale:    source.scale,
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
