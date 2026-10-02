//! How a label reaches the screen: its glyphs through the text brush, its
//! color glyphs as images and its underlines as rects. The other half of
//! `ui_drawer.rs`.

use wgpu_text::{Section, Text, TextBuilder, glyph_brush::HorizontalAlign};

use crate::{
    deps::refs::Weak,
    gm::{
        LossyConvert,
        color::{CLEAR, Color},
        flat::{CornerRadii, Point, Rect},
    },
    pipelines::Pipelines,
    render::data::{UIImageInstance, UIRectInstance},
    ui::{
        Label, TextAlignment, UIManager, VerticalAlignment, ViewFrame,
        ui_drawer::{IMAGE_RECT_DRAWER, UIDrawer},
    },
    window::{Font, ShapedLayout, ShapedParams, VerticalAlign},
};

/// How much nearer each layer of an outlined text sits than the one before,
/// 2 steps of a 24 bit depth buffer around the middle of its range.
pub(super) const LAYER_STEP: f32 = f32::EPSILON;

/// What a copy of the text under the text itself is made of.
pub(super) enum Under {
    /// The glyphs as they are, a hard shadow.
    Plain,
    /// Every glyph edge moved out by this many pixels, an outline.
    Wider(f32),
    /// The glyphs blurred with this radius in pixels, a soft shadow.
    Blurred(f32),
}

/// The copies of a label's text that lie under it, the shadow first and
/// the outline over it, each with its color and its offset in pixels.
pub(super) fn under_layers(label: &Label, scale: f32) -> Vec<(Color, Point, Under)> {
    let mut under = Vec::new();
    if let Some(shadow) = label.text_shadow() {
        let kind = if shadow.blur > 0.0 {
            Under::Blurred(shadow.blur * scale)
        } else {
            Under::Plain
        };
        under.push((shadow.color, shadow.offset * scale, kind));
    }
    if let Some(outline) = label.text_outline() {
        under.push((
            outline.color,
            Point::default(),
            Under::Wider(outline.width * scale),
        ));
    }
    under
}

pub(super) type TextSections<'a> = Vec<(Weak<Font>, Vec<(Section<'a>, ShapedParams)>)>;

impl UIDrawer {
    /// The shaping parameters of a label drawn at `scale`, the same for
    /// the brush and for the color glyph images.
    pub(super) fn label_params(label: &Label, text: &str, scale: f32) -> ShapedParams {
        ShapedParams {
            tracking:    label.letter_spacing() * scale,
            multiline:   label.is_multiline(),
            h_align:     match label.alignment {
                TextAlignment::Left => HorizontalAlign::Left,
                TextAlignment::Center => HorizontalAlign::Center,
                TextAlignment::Right => HorizontalAlign::Right,
            },
            v_align:     match label.vertical_alignment {
                VerticalAlignment::Top => VerticalAlign::Top,
                VerticalAlignment::Center => VerticalAlign::Center,
            },
            line_height: label.line_height().map(|height| height * scale),
            base:        label.font(),
            runs:        label.shaping_runs(text),
            secret:      label.is_secret(),
        }
    }

    /// Where a label's text anchors inside its pixel `frame` and the
    /// bounds it wraps in, the section geometry of the brush.
    pub(super) fn label_geometry(frame: &Rect, label: &Label) -> ((f32, f32), (f32, f32)) {
        let center = frame.center();
        let margin = 16.0;

        let bounds = (
            frame.width() - if label.alignment.center() { 0.0 } else { margin },
            frame.height(),
        );
        let position = (
            match label.alignment {
                TextAlignment::Left => frame.x() + margin,
                TextAlignment::Center => center.x,
                TextAlignment::Right => frame.max_x() - margin,
            },
            match label.vertical_alignment {
                VerticalAlignment::Top => frame.y(),
                VerticalAlignment::Center => center.y,
            },
        );
        (position, bounds)
    }

    /// The color glyphs of a label, the emoji of a color font, as images
    /// at the positions the shaper gave them. The brush skips these
    /// glyphs, see `ShapedLayout::color_glyphs`.
    pub(super) fn draw_color_glyphs(frame: &Rect, label: &Label, scale: f32, opacity: f32) {
        if !label.uses_color_font() {
            return;
        }

        let text = label.display_text(frame.size.width);
        let params = Self::label_params(label, text, scale);
        let font = params.base;

        let frame = frame * scale;
        let (position, bounds) = Self::label_geometry(&frame, label);
        let layout = ShapedLayout {
            emit: &font.name,
            params,
        };
        let scale_px = label.text_size() * scale * font.em_scale();
        let z = label.z_position() - UIManager::additional_z_offset();

        for placed in layout.color_glyphs(scale_px, text, position, bounds.0) {
            let Some(glyph) = placed.font.color_glyph(placed.id, placed.px_per_em) else {
                continue;
            };
            // Snapped to whole pixels, a fractional origin would blur the
            // image and land on different pixels per GPU.
            let x = (placed.x + glyph.left).round() / scale;
            let y = (placed.baseline + glyph.top).round() / scale;
            let size = glyph.size / scale;
            let rect: Rect = (x, y, size.width, size.height).into();

            IMAGE_RECT_DRAWER.get_mut().add_with_image(
                UIImageInstance::new(
                    rect,
                    (0, 0, 1, 1).into(),
                    CLEAR,
                    0.0,
                    CornerRadii::default(),
                    z,
                    false,
                    false,
                    scale,
                )
                .with_opacity(opacity),
                glyph.image,
            );
        }
    }

    pub(super) fn draw_label<'a>(
        frame: &Rect,
        label: &'a Label,
        sections: &mut TextSections<'a>,
        scale: f32,
        opacity: f32,
    ) {
        // The full text, or the ellipsized copy when the label opted in
        // and the text overflows this width.
        let text = label.display_text(frame.size.width);

        let frame = frame * scale;

        let params = Self::label_params(label, text, scale);
        let font = params.base;

        let scale_px = label.text_size() * scale * font.em_scale();
        let z = label.z_position() - UIManager::additional_z_offset();

        let make_text = |slice: &'a str, color: &Color| {
            Text::new(slice)
                .with_scale(scale_px)
                .with_color(color.faded(opacity).as_slice())
                .with_z(z)
        };

        let mut section = Section::new();

        if label.color_runs().is_empty() {
            let mut colored = make_text(text, label.text_color());

            // After `with_color`, which sets both ends of the ramp so that a
            // label without a gradient stays flat.
            if let Some(end) = label.text_end_color() {
                colored = colored.with_end_color(end.faded(opacity).as_slice());
            }

            section = section.add_text(colored);
        } else {
            // One glyph_brush text per run and per gap between runs. The
            // layout shapes them as one string and only picks the color
            // per glyph, so a run boundary never breaks kerning.
            //
            // Runs are byte ranges of the full text. An ellipsized copy is
            // shorter and ends in the multi byte ellipsis, so a clamped
            // range backs off to a char boundary of what is drawn, which
            // keeps the ellipsis itself in the text color.
            let clamp = |position: usize| {
                let mut position = position.min(text.len());
                while !text.is_char_boundary(position) {
                    position -= 1;
                }
                position
            };
            let mut cursor = 0;

            for run in label.color_runs() {
                let (start, end) = (clamp(run.range.start), clamp(run.range.end));
                if start >= end {
                    continue;
                }
                if cursor < start {
                    section = section.add_text(make_text(&text[cursor..start], label.text_color()));
                }
                section = section.add_text(make_text(&text[start..end], &run.color));
                cursor = end;
            }

            if cursor < text.len() {
                section = section.add_text(make_text(&text[cursor..], label.text_color()));
            }
        }

        let (position, bounds) = Self::label_geometry(&frame, label);
        let mut section = section.with_bounds(bounds).with_screen_position(position);

        // A hard shadow is the same text again in 1 color, queued before
        // the text itself. The outline and a soft shadow are images, see
        // `label_effect.rs`. Text writes depth over whole glyph boxes, so
        // every layer sits a little nearer than the one before it, or it
        // would be cut by the boxes of the last one. The text is 2 depth
        // steps in front of its view, there is no room behind it, so the
        // layers start where plain text sits and the text itself comes
        // forward. 3 layers stay far inside the gap to the next view.
        let under = under_layers(label, scale);
        let mut layers = Vec::with_capacity(under.len() + 1);
        for (index, (color, offset, kind)) in under.iter().enumerate() {
            if !matches!(kind, Under::Plain) {
                continue;
            }
            let forward: f32 = index.lossy_convert();
            let copy = Text::new(text)
                .with_scale(scale_px)
                .with_color(color.faded(opacity).as_slice())
                .with_z(z - forward * LAYER_STEP);
            layers.push(
                Section::new()
                    .add_text(copy)
                    .with_bounds(bounds)
                    .with_screen_position((position.0 + offset.x, position.1 + offset.y)),
            );
        }
        if !under.is_empty() {
            let forward: f32 = under.len().lossy_convert();
            for piece in &mut section.text {
                piece.extra.z = z - forward * LAYER_STEP;
            }
        }
        layers.push(section);

        // A font run draws through its own font's brush, so the section
        // is queued once per font it touches. Every copy lays the whole
        // text out and keeps the glyphs of its own font.
        let mut fonts = vec![font];
        for run in &params.runs {
            if !fonts.iter().any(|f| f.name == run.font.name) {
                fonts.push(run.font);
            }
        }

        for font in fonts {
            let queued = layers.iter().map(|layer| (layer.clone(), params.clone()));
            match sections.iter_mut().find(|(f, _)| f.name == font.name) {
                Some((_, list)) => list.extend(queued),
                None => sections.push((font, queued.collect())),
            }
        }
    }

    /// One rect under every line piece of an underlined run, in the
    /// color the text has there. Between the label background and its
    /// glyphs, so a descender paints over the line like in a browser.
    pub(super) fn draw_underlines(frame: &Rect, label: &Label, scale: f32, opacity: f32) {
        let text = label.display_text(frame.size.width);
        let ranges = label.underline_runs(text);
        if ranges.is_empty() {
            return;
        }

        let layout = label.text_layout_for(text);
        let inset = label.text_inset();
        // Snapped to whole screen pixels. A hairline on a fractional row
        // gets its two partial rows blended differently by every GPU, a
        // whole row reads the same everywhere, and a crisp line is what a
        // browser draws too.
        let (position, thickness) = layout.underline;
        let thickness = (thickness * scale).round().max(1.0) / scale;
        let z = label.z_position() - UIManager::additional_z_offset() / 2.0;

        let top = match label.vertical_alignment {
            VerticalAlignment::Top => frame.y(),
            VerticalAlignment::Center => frame.y() + frame.height() / 2.0 - layout.total_height() / 2.0,
        };

        for (index, line) in layout.lines.iter().enumerate() {
            let line_x = match label.alignment {
                TextAlignment::Left => frame.x() + inset,
                TextAlignment::Center => frame.x() + (frame.width() - line.width) / 2.0,
                TextAlignment::Right => frame.max_x() - inset - line.width,
            };
            let count: f32 = index.lossy_convert();
            let baseline = top + layout.ascent + count * layout.line_height;
            let y = ((baseline - position) * scale).round() / scale;

            for range in &ranges {
                let start = range.start.max(line.start);
                let end = range.end.min(line.end);
                if start >= end {
                    continue;
                }
                let x0 = layout.x_on_line(index, start);
                let x1 = layout.x_on_line(index, end);

                Pipelines::rect().add(UIRectInstance::new(
                    (line_x + x0, y, x1 - x0, thickness).into(),
                    label.color_at(start).faded(opacity),
                    CLEAR,
                    0.0,
                    CornerRadii::default(),
                    z,
                    scale,
                ));
            }
        }
    }
}
