use std::hash::{Hash, Hasher};

use rustybuzz::{Face, UnicodeBuffer, shape};
use wgpu_text::glyph_brush::{
    GlyphPositioner, HorizontalAlign, SectionGeometry, SectionGlyph, ToSectionText,
    ab_glyph::{Font, Glyph, GlyphId, Rect, ScaleFont, point},
};

use crate::gm::LossyConvert;

/// Per label shaping parameters, collected where the label is drawn.
#[derive(Clone, Copy)]
pub(crate) struct ShapedParams {
    /// Extra pixels added to every glyph advance, `CoreText` style tracking.
    pub tracking:  f32,
    pub multiline: bool,
    pub h_align:   HorizontalAlign,
}

/// Positions glyphs with real shaping through rustybuzz, so GPOS kerning
/// and font variations apply like they do in CoreText and browsers. The
/// builtin `glyph_brush` layout only reads the legacy kern table, which
/// modern fonts like SF Pro do not have. Vertical alignment is always
/// center, matching how the engine draws labels.
pub(crate) struct ShapedLayout<'a> {
    pub face:      &'a Face<'static>,
    pub font_name: &'a str,
    pub params:    ShapedParams,
}

impl Hash for ShapedLayout<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.font_name.hash(state);
        self.params.tracking.to_bits().hash(state);
        self.params.multiline.hash(state);
        (self.params.h_align as u8).hash(state);
    }
}

struct ShapedGlyph {
    id:        u16,
    cluster:   u32,
    x_advance: f32,
    x_offset:  f32,
    y_offset:  f32,
}

impl ShapedLayout<'_> {
    fn shape_line(&self, line: &str, px_per_unit: f32) -> Vec<ShapedGlyph> {
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(line);

        let shaped = shape(self.face, &[], buffer);

        shaped
            .glyph_infos()
            .iter()
            .zip(shaped.glyph_positions())
            .map(|(info, pos)| ShapedGlyph {
                id:        u16::try_from(info.glyph_id).unwrap_or_default(),
                cluster:   info.cluster,
                x_advance: pos.x_advance.lossy_convert() * px_per_unit + self.params.tracking,
                x_offset:  pos.x_offset.lossy_convert() * px_per_unit,
                y_offset:  pos.y_offset.lossy_convert() * px_per_unit,
            })
            .collect()
    }

    /// Greedy wrap at space glyphs. A line that has no space to break at
    /// overflows, same as the builtin layout.
    fn wrap(line: Vec<ShapedGlyph>, text: &str, max_width: f32) -> Vec<Vec<ShapedGlyph>> {
        let is_space = |glyph: &ShapedGlyph| text.as_bytes().get(glyph.cluster as usize) == Some(&b' ');

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

        for (section_index, section) in sections.iter().enumerate() {
            let section = section.to_section_text();
            let font = &fonts[section.font_id.0];
            let scaled = font.as_scaled(section.scale);

            // The same factor ab_glyph rasterizes with, keeps shaped
            // advances and drawn outlines consistent.
            let px_per_unit = scaled.scale_factor().horizontal;

            let mut lines = vec![];

            for raw_line in section.text.split('\n') {
                let shaped = self.shape_line(raw_line, px_per_unit);
                if self.params.multiline {
                    lines.extend(Self::wrap(shaped, section.text, bound_w));
                } else {
                    lines.push(shaped);
                }
            }

            let line_height = scaled.ascent() - scaled.descent() + scaled.line_gap();
            let line_count: f32 = lines.len().lossy_convert();
            let total_height = line_count * line_height - scaled.line_gap();
            let mut baseline = screen_y - total_height / 2.0 + scaled.ascent();

            for line in lines {
                let line_width: f32 = line.iter().map(|g| g.x_advance).sum();

                let mut x = match self.params.h_align {
                    HorizontalAlign::Left => screen_x,
                    HorizontalAlign::Center => screen_x - line_width / 2.0,
                    HorizontalAlign::Right => screen_x - line_width,
                };

                for glyph in line {
                    result.push(SectionGlyph {
                        section_index,
                        byte_index: glyph.cluster as usize,
                        glyph: Glyph {
                            id:       GlyphId(glyph.id),
                            scale:    section.scale,
                            position: point(x + glyph.x_offset, baseline - glyph.y_offset),
                        },
                        font_id: section.font_id,
                    });
                    x += glyph.x_advance;
                }

                baseline += line_height;
            }
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

        Rect {
            min: point(min_x, y - h / 2.0),
            max: point(max_x, y + h / 2.0),
        }
    }
}
