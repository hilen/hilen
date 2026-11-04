use anyhow::Result;
use rustybuzz::ttf_parser::{Face, GlyphId};
use ui_proc::view;
use wgpu_text::glyph_brush::ab_glyph::{Font as _, PxScale, ScaleFont};

use crate::{
    deps::refs::Weak,
    gm::flat::Size,
    ui::{
        Container, Label, RunStyle, TextAlignment, UIColor, UIManager, VerticalAlignment, ViewCallbacks,
        ViewFrame, ViewSubviews,
        view::ViewData,
        views::complex::math::{
            layout::{Item, MathBox, lay_out},
            parse::parse,
        },
    },
    window::{Font, MATH_FONT_DATA},
};

/// A LaTeX formula drawn like a LaTeX document, with Latin Modern Math.
/// Powers and indices, fractions, roots, sums and integrals with their
/// limits, brackets that grow, accents, the math alphabets like `\mathbb`
/// and the matrix and cases environments.
///
/// The view does not size itself. `set_latex` gives the size the formula
/// takes and `baseline` where its baseline is from the top, the owner
/// gives it that frame.
#[view]
pub struct MathView {
    laid:  MathBox,
    color: UIColor,
    /// The math font, [`Font::math`] when not set.
    font:  Option<Weak<Font>>,
}

impl ViewCallbacks for MathView {}

impl MathView {
    /// Lays out `latex` at `size` points per em, as a display formula or
    /// as one inside a line, and draws it. A formula that does not parse
    /// leaves the view empty and gives the error.
    pub fn set_latex(self: Weak<Self>, latex: &str, display: bool, size: f32) -> Result<Size> {
        let laid = lay_out(&parse(latex)?, size, display)?;
        Ok(self.set_box(laid))
    }

    /// The color of the formula, black when not set.
    pub fn set_color(mut self: Weak<Self>, color: impl Into<UIColor>) -> Weak<Self> {
        self.color = color.into();
        self.build();
        self
    }

    /// A copy of the math font to draw with, like one of
    /// [`Font::math_darkened`]. The layout stays the one of the font file.
    pub fn set_font(mut self: Weak<Self>, font: Weak<Font>) -> Weak<Self> {
        self.font = Some(font);
        self.build();
        self
    }

    /// How far the baseline is from the top of the formula.
    pub fn baseline(&self) -> f32 {
        self.laid.ascent
    }

    pub fn formula_size(&self) -> Size {
        Size::new(self.laid.width, self.laid.ascent + self.laid.depth)
    }

    pub(crate) fn set_box(mut self: Weak<Self>, laid: MathBox) -> Size {
        self.laid = laid;
        self.build();
        self.formula_size()
    }

    fn build(mut self: Weak<Self>) {
        self.remove_all_subviews();
        let Ok(face) = Face::parse(MATH_FONT_DATA, 0) else {
            return;
        };
        let upem = f32::from(face.units_per_em());
        let top = self.laid.ascent;
        let color = self.color;
        let items = std::mem::take(&mut self.laid.items);
        for item in &items {
            match *item {
                Item::Glyph { id, ch, x, y, size } => {
                    let glyph = GlyphId(id);
                    let advance = face
                        .glyph_hor_advance(glyph)
                        .map_or(size * 0.6, |units| f32::from(units) * size / upem);
                    // How far the ink reaches from the baseline, a big
                    // bracket reaches far past the line of the font.
                    let reach = face
                        .glyph_bounding_box(glyph)
                        .map_or(size, |rect| f32::from(rect.y_max.max(-rect.y_min)) * size / upem);
                    self.glyph(GlyphAt {
                        id,
                        ch,
                        x,
                        baseline: top - y,
                        size,
                        advance,
                        reach,
                    });
                }
                Item::Rule { x, y, width, height } => {
                    // A rule thinner than a pixel covers no pixel center on
                    // some rows, and a draw with 1 sample, like WebGL, then
                    // leaves out the bar of a fraction. 1 pixel always
                    // covers 1 center.
                    let thick = height.max(1.0 / UIManager::scale());
                    let rule = self.add_view::<Container>();
                    rule.set_color(color);
                    rule.set_frame((x, top - y - height - (thick - height) / 2.0, width, thick));
                }
            }
        }
        self.laid.items = items;
    }

    /// 1 glyph as a label. A glyph no char maps to, a bigger variant of a
    /// bracket or a part of a tall one, is picked by its id.
    fn glyph(self: Weak<Self>, at: GlyphAt) {
        let font = self.font.unwrap_or_else(Font::math);
        let scaled = font.ab().as_scaled(PxScale::from(at.size * font.em_scale()));
        let label = self.add_view::<Label>();
        // Centered both ways in a frame far bigger than the glyph, since a
        // label cuts the ink that leaves its frame on the side its text
        // starts from, the lean of an italic letter or the top of a tall
        // integral.
        label
            .set_alignment(TextAlignment::Center)
            .set_vertical_alignment(VerticalAlignment::Center)
            .set_text_size(at.size);
        label.set_font(font);
        match (at.id, at.ch) {
            (0, Some(ch)) => {
                label.set_text(ch.to_string());
            }
            (id, _) => {
                label.set_text("x");
                label.set_font_runs([(0..1, RunStyle::glyph(id, at.advance))]);
            }
        }
        label.set_text_color(self.color);
        let pad = at.size;
        let height = (at.reach + pad) * 2.0;
        // A centered line has its baseline this far under the middle of
        // the frame, see `ShapedLayout::first_baseline`.
        let drop = f32::midpoint(scaled.ascent(), scaled.descent());
        let center_y = at.baseline - drop;
        label.set_frame((
            at.x - pad,
            center_y - height / 2.0,
            at.advance + pad * 2.0,
            height,
        ));
    }
}

/// Where 1 glyph goes in the view, y down.
struct GlyphAt {
    id:       u16,
    ch:       Option<char>,
    x:        f32,
    baseline: f32,
    size:     f32,
    advance:  f32,
    /// How far its ink reaches above or below the baseline.
    reach:    f32,
}
