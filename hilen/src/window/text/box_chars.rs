//! Box chars, U+2500 to U+257F, stretched over their whole cell the way a
//! terminal draws them, so the lines of a table drawn with them join.

use rustybuzz::Face;
use wgpu_text::glyph_brush::ab_glyph::{Font as AbGlyphFont, Point, PxScale, point};

use crate::window::Font;

/// How far a fitted glyph reaches past each edge of its cell, in pixels.
/// The edges are whole pixels, so 2 neighbors meet between 2 pixels.
/// The overlap covers the 0.05 pixel a glyph position moves to its
/// atlas step. It is kept small, the soft edge of a stroke is drawn
/// twice where 2 glyphs overlap and shows as a darker dot.
const OVERSHOOT: f32 = 0.25;

/// The layers the fitted glyphs of a label are drawn in, 1 for each
/// pair of the parity of the line and the parity of the place in it. 2
/// glyphs next to each other, also on a diagonal, are never in 1 layer.
pub(crate) const BOX_LAYERS: u8 = 4;

/// How a layout treats the box chars of a text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) enum BoxChars {
    /// As glyphs like any other.
    #[default]
    Glyphs,
    /// Each over its whole cell. The brush gets every glyph but these.
    Fitted,
    /// Each over its whole cell. The brush gets only the fitted glyphs of
    /// this layer, see `BOX_LAYERS`.
    Layer(u8),
}

pub(crate) fn is_box_char(char: char) -> bool {
    ('\u{2500}'..='\u{257F}').contains(&char)
}

/// The scale and the origin of the glyph of the char `rest` starts with
/// when it is a box char of `font` that fills its cell, the left edge,
/// the top of the line, the advance and the line pitch in pixels. `None`
/// for any other char.
pub(crate) fn fit_box(rest: &str, font: &Font, cell: (f32, f32, f32, f32)) -> Option<(PxScale, Point)> {
    let frame = font.box_frame()?;
    rest.chars().next().filter(|char| is_box_char(*char))?;
    Some(frame.fit(cell, font.ab().height_unscaled()))
}

/// The layer of the glyph at `place` on `line`.
pub(crate) fn box_layer(line: usize, place: usize) -> u8 {
    u8::from(line % 2 == 1) * 2 + u8::from(place % 2 == 1)
}

/// What the box chars of a font are drawn for, in font units: from the
/// left to the right end of `─`, from the bottom to the top of `│`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BoxFrame {
    left:   f32,
    right:  f32,
    bottom: f32,
    top:    f32,
}

impl BoxFrame {
    /// `None` for a font with no `─` or no `│`, its box chars stay as
    /// they are.
    pub(crate) fn of(face: &Face) -> Option<Self> {
        let across = face.glyph_bounding_box(face.glyph_index('─')?)?;
        let down = face.glyph_bounding_box(face.glyph_index('│')?)?;
        (across.x_max > across.x_min && down.y_max > down.y_min).then(|| Self {
            left:   across.x_min.into(),
            right:  across.x_max.into(),
            bottom: down.y_min.into(),
            top:    down.y_max.into(),
        })
    }

    /// The scale and the origin that lay this frame over the cell of a
    /// char, `OVERSHOOT` past each of its edges. The cell is in pixels,
    /// left and top edge, width and height, and its edges are rounded to
    /// whole pixels. The same edge of 2 neighbors rounds the same way.
    /// `units` is the ascent minus the descent of the font in font units,
    /// which `PxScale` counts in.
    pub(crate) fn fit(self, cell: (f32, f32, f32, f32), units: f32) -> (PxScale, Point) {
        let (left, top) = (cell.0.round(), cell.1.round());
        let width = (cell.0 + cell.2).round() - left;
        let height = (cell.1 + cell.3).round() - top;
        let x = (width + OVERSHOOT * 2.0) / (self.right - self.left);
        let y = (height + OVERSHOOT * 2.0) / (self.top - self.bottom);
        (
            PxScale {
                x: x * units,
                y: y * units,
            },
            point(left - OVERSHOOT - self.left * x, top - OVERSHOOT + self.top * y),
        )
    }
}
