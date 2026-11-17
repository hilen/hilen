//! Which glyphs of a color font draw as color pictures. The shaper asks it
//! for every glyph, the rasters themselves are in `color_glyph.rs`.

use rustybuzz::ttf_parser::{Face, GlyphId, RasterImageFormat};

use crate::gm::LossyConvert;

/// The strike the bitmap tables are asked for. A bitmap font keeps a few
/// sizes and hands out the nearest one, the drawer scales it the rest of
/// the way.
pub(super) fn strike_ppem(px_per_em: f32) -> u16 {
    let ppem: u32 = px_per_em.round().lossy_convert();
    u16::try_from(ppem).unwrap_or(u16::MAX).max(1)
}

/// Whether `glyph` draws through the color path at `px_per_em`. Must
/// agree with `color_glyph::rasterize` so the brush never draws what the
/// image pipeline draws too.
pub(crate) fn is_color_glyph(face: &Face, glyph: GlyphId, px_per_em: f32) -> bool {
    face.glyph_raster_image(glyph, strike_ppem(px_per_em))
        .is_some_and(|raster| raster.format == RasterImageFormat::PNG)
        || face.is_color_glyph(glyph)
}
