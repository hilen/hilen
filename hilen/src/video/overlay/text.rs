//! The text of an overlay as pixels, drawn on the CPU so an export needs no
//! window. It mirrors what a `Label` with the default font, 1 line per line
//! break and the default alignment does in `window/text/shaped_layout.rs`:
//! rustybuzz shapes every line, the size is pixels per em, the lines stand
//! in the middle, and `ab_glyph` fills the same outlines the glyph atlas of
//! a label holds.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{LazyLock, Mutex, PoisonError},
};

use anyhow::{Context, Result, anyhow};
use rustybuzz::{Face, UnicodeBuffer, shape};
use wgpu_text::glyph_brush::ab_glyph::{
    Font as AbGlyphFont, FontRef, Glyph, GlyphId, OutlinedGlyph, PxScale, ScaleFont, point,
};

use crate::{
    gm::{
        LossyConvert,
        color::Color,
        flat::{Point, Size},
    },
    video::overlay::Layer,
    window::DEFAULT_FONT_DATA,
};

/// A font for the outlines and for shaping. A `Font` of the engine cannot
/// be used here, it belongs to the main thread and to a window.
pub(crate) struct TextFace {
    outlines: FontRef<'static>,
    shaping:  Face<'static>,
    /// Turns pixels per em into the `PxScale` of `ab_glyph`, see
    /// `Font::em_scale`.
    em_scale: f32,
}

impl TextFace {
    fn new(data: &'static [u8]) -> Result<Self> {
        let outlines = FontRef::try_from_slice(data)?;
        let shaping = Face::from_slice(data, 0).ok_or_else(|| anyhow!("the font does not shape"))?;
        let units_per_em = outlines.units_per_em().ok_or_else(|| anyhow!("the font has no units per em"))?;
        Ok(Self {
            em_scale: outlines.height_unscaled() / units_per_em,
            outlines,
            shaping,
        })
    }
}

/// The font a `Label` has by default.
pub(super) static FACE: LazyLock<TextFace> =
    LazyLock::new(|| TextFace::new(DEFAULT_FONT_DATA).expect("the built in font has outlines and shapes"));

/// The faces of font files by their path. A face borrows its bytes for the
/// whole run, so each file is read and kept once, an app that builds its
/// overlays again on every edit reads nothing new.
static FILES: LazyLock<Mutex<HashMap<PathBuf, (&'static Path, &'static TextFace)>>> =
    LazyLock::new(Mutex::default);

/// The path and the face of a font file, read on the first call for its
/// path.
pub(super) fn face_of_file(path: &Path) -> Result<(&'static Path, &'static TextFace)> {
    let mut files = FILES.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(known) = files.get(path) {
        return Ok(*known);
    }
    let data = std::fs::read(path).with_context(|| format!("the font {}", path.display()))?;
    let face = TextFace::new(Vec::leak(data)).with_context(|| format!("the font {}", path.display()))?;
    let known: (&'static Path, &'static TextFace) = (
        Box::leak(path.to_path_buf().into_boxed_path()),
        Box::leak(Box::new(face)),
    );
    files.insert(path.to_path_buf(), known);
    Ok(known)
}

/// The glyphs of a text on their baselines, and the box of its lines.
struct Laid {
    glyphs: Vec<Glyph>,
    size:   Size,
}

/// Lays `text` out at `em` pixels per em with the middle of its lines at
/// `middle`.
fn lay_out(face: &TextFace, text: &str, em: f32, middle: Point) -> Laid {
    let scale = PxScale::from(em * face.em_scale);
    let scaled = face.outlines.as_scaled(scale);
    let per_unit = scaled.scale_factor().horizontal;
    let pixels = |units: i32| -> f32 {
        let units: f32 = units.lossy_convert();
        units * per_unit
    };
    let pitch = scaled.ascent() - scaled.descent() + scaled.line_gap();
    let lines: Vec<&str> = text.split('\n').collect();
    let count: f32 = lines.len().lossy_convert();
    let height = count * pitch - scaled.line_gap();

    let mut baseline = middle.y - height / 2.0 + scaled.ascent();
    let mut glyphs = vec![];
    let mut width = 0.0_f32;
    for line in lines {
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(line);
        let shaped = shape(&face.shaping, &[], buffer);
        let line_width: f32 = shaped.glyph_positions().iter().map(|place| pixels(place.x_advance)).sum();
        let mut x = middle.x - line_width / 2.0;
        for (info, place) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
            glyphs.push(Glyph {
                id: GlyphId(u16::try_from(info.glyph_id).unwrap_or_default()),
                scale,
                position: point(x + pixels(place.x_offset), baseline - pixels(place.y_offset)),
            });
            x += pixels(place.x_advance);
        }
        width = width.max(line_width);
        baseline += pitch;
    }
    Laid {
        glyphs,
        size: Size::new(width, height),
    }
}

/// The box the lines of `text` take at `em` pixels per em: the widest line
/// and the height from the top of the first line to the bottom of the last.
pub(super) fn measure(face: &TextFace, text: &str, em: f32) -> Size {
    lay_out(face, text, em, Point::default()).size
}

/// The text as premultiplied pixels in `color`, with the middle of its
/// lines at `middle` of the frame. None for a text that draws no glyph.
pub(super) fn raster(face: &TextFace, text: &str, em: f32, middle: Point, color: Color) -> Option<Layer> {
    let outlines: Vec<OutlinedGlyph> = lay_out(face, text, em, middle)
        .glyphs
        .into_iter()
        .filter_map(|glyph| face.outlines.outline_glyph(glyph))
        .collect();

    let boxes = outlines.iter().map(OutlinedGlyph::px_bounds);
    let (left, top, right, bottom) = boxes.fold(None, |all: Option<(f32, f32, f32, f32)>, one| {
        Some(all.map_or((one.min.x, one.min.y, one.max.x, one.max.y), |all| {
            (
                all.0.min(one.min.x),
                all.1.min(one.min.y),
                all.2.max(one.max.x),
                all.3.max(one.max.y),
            )
        }))
    })?;
    let (left, top) = (left.floor(), top.floor());
    let width: usize = (right.ceil() - left).lossy_convert();
    let height: usize = (bottom.ceil() - top).lossy_convert();
    if width == 0 || height == 0 {
        return None;
    }

    let mut coverage = vec![0.0_f32; width * height];
    for outline in &outlines {
        let bounds = outline.px_bounds();
        let x0: usize = (bounds.min.x - left).lossy_convert();
        let y0: usize = (bounds.min.y - top).lossy_convert();
        outline.draw(|x, y, covered| {
            let at = (y0 + y as usize) * width + x0 + x as usize;
            if let Some(pixel) = coverage.get_mut(at) {
                // Glyphs that touch add up, like on the screen.
                *pixel = (*pixel + covered).min(1.0);
            }
        });
    }

    let channel = |value: f32| -> u8 { (value.clamp(0.0, 1.0) * 255.0).round().lossy_convert() };
    let mut rgba = Vec::with_capacity(coverage.len() * 4);
    for covered in coverage {
        let alpha = color.a * covered;
        rgba.extend_from_slice(&[
            channel(color.r * alpha),
            channel(color.g * alpha),
            channel(color.b * alpha),
            channel(alpha),
        ]);
    }
    Some(Layer {
        left: left.lossy_convert(),
        top: top.lossy_convert(),
        width,
        rgba,
    })
}
