//! The outline and the soft shadow of a label. Each is the coverage of the
//! label's glyphs, spread on the CPU in 2 passes and kept as an image, so a
//! frame draws it as 1 quad and a wide spread costs nothing per frame. The
//! image is made again only when the text, its layout or the effect changes.

use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
};

use hilen_pixels::{blur_coverage, widen_coverage};
use wgpu_text::glyph_brush::ab_glyph::{Font as AbGlyphFont, OutlinedGlyph};

use crate::{
    deps::refs::{Weak, main_lock::MainLock, manage::DataManager},
    gm::{
        LossyConvert,
        color::{CLEAR, Color},
        flat::{CornerRadii, Point, Rect, Size},
    },
    render::data::UIImageInstance,
    ui::{
        Label, UIManager, ViewFrame,
        label_drawer::{LAYER_STEP, Under, under_layers},
        ui_drawer::{IMAGE_RECT_DRAWER, UIDrawer},
    },
    window::{ShapedLayout, ShapedParams, Window, image::Image},
};

/// Frames an effect image stays after its last draw. A label that is hidden
/// for a moment keeps its image, a line of subtitles that is gone frees it.
const KEPT_FRAMES: u64 = 120;

/// One spread copy of a text, ready to draw.
#[derive(Clone, Copy)]
struct Effect {
    image: Weak<Image>,
    /// The top left of the image against the label's whole pixel origin.
    left:  f32,
    top:   f32,
    size:  Size,
}

/// By the hash of everything the image depends on, with the frame it was
/// drawn in last. None for a text with no glyph to spread, so it is not
/// rastered again every frame.
static EFFECTS: MainLock<HashMap<u64, (Option<Effect>, u64)>> = MainLock::new();

/// What one effect image is made from. The place is the label's place
/// against its own whole pixel origin, so a label that moves by whole
/// pixels keeps its image.
struct Recipe<'a> {
    text:     &'a str,
    params:   ShapedParams,
    scale_px: f32,
    position: (f32, f32),
    bounds:   (f32, f32),
    color:    Color,
    spread:   f32,
    soft:     bool,
}

impl Recipe<'_> {
    fn key(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.text.hash(&mut hasher);
        ShapedLayout {
            emit:   "",
            params: self.params.clone(),
        }
        .hash(&mut hasher);
        for part in [
            self.scale_px,
            self.position.0,
            self.position.1,
            self.bounds.0,
            self.bounds.1,
            self.color.r,
            self.color.g,
            self.color.b,
            self.color.a,
            self.spread,
        ] {
            part.to_bits().hash(&mut hasher);
        }
        self.soft.hash(&mut hasher);
        hasher.finish()
    }

    /// Rasters the glyphs into one coverage map, spreads it and uploads it
    /// in the color of the effect. None when the text draws no glyph.
    fn make(&self, key: u64) -> Option<Effect> {
        let layout = ShapedLayout {
            emit:   "",
            params: self.params.clone(),
        };
        let outlines: Vec<_> = layout
            .plain_glyphs(self.scale_px, self.text, self.position, self.bounds.0)
            .into_iter()
            .filter_map(|(font, glyph)| font.ab().outline_glyph(glyph))
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

        // Room for the spread and 1 pixel more, the rim of it is soft.
        let pad = self.spread.ceil() + 1.0;
        let (left, top) = (left.floor() - pad, top.floor() - pad);
        let width: usize = (right.ceil() + pad - left).lossy_convert();
        let height: usize = (bottom.ceil() + pad - top).lossy_convert();
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

        let spread = if self.soft {
            blur_coverage(&coverage, width, self.spread)
        } else {
            widen_coverage(&coverage, width, self.spread)
        };

        let channel = |value: f32| -> u8 { (value.clamp(0.0, 1.0) * 255.0).round().lossy_convert() };
        let (red, green, blue) = (
            channel(self.color.r),
            channel(self.color.g),
            channel(self.color.b),
        );
        let mut pixels = Vec::with_capacity(spread.len() * 4);
        for covered in spread {
            pixels.extend_from_slice(&[red, green, blue, channel(self.color.a * covered)]);
        }

        let size = Size::<u32>::new(
            u32::try_from(width).expect("an effect image is narrower than u32"),
            u32::try_from(height).expect("an effect image is lower than u32"),
        );
        let image = Image::from_raw_data(pixels, format!("text effect {key:016x}"), size, 4);
        Some(Effect {
            image,
            left,
            top,
            size: Size::new(size.width.lossy_convert(), size.height.lossy_convert()),
        })
    }
}

impl UIDrawer {
    /// The outline and the soft shadow of a label as images under its text.
    /// A hard shadow is a plain copy of the text, the brush draws it.
    pub(super) fn draw_text_effects(frame: &Rect, label: &Label, scale: f32, opacity: f32) {
        let layers = under_layers(label, scale);
        if !layers.iter().any(|(_, _, kind)| !matches!(kind, Under::Plain)) {
            return;
        }

        let text = label.display_text(frame.size.width);
        let params = Self::label_params(label, text, scale);
        let scale_px = label.text_size() * scale * params.base.em_scale();
        let z = label.z_position() - UIManager::additional_z_offset();

        // The image is made for the label at its whole pixel origin and
        // drawn pixel for pixel, a blurred copy of a blur would be softer
        // than asked.
        let frame = frame * scale;
        let origin = Point::new(frame.x().floor(), frame.y().floor());
        let local: Rect = (
            frame.x() - origin.x,
            frame.y() - origin.y,
            frame.width(),
            frame.height(),
        )
            .into();
        let (position, bounds) = Self::label_geometry(&local, label);

        let now = Window::render_frame();
        for (index, (color, offset, kind)) in layers.into_iter().enumerate() {
            let (spread, soft) = match kind {
                Under::Plain => continue,
                Under::Wider(width) => (width, false),
                Under::Blurred(radius) => (radius, true),
            };
            let recipe = Recipe {
                text,
                params: params.clone(),
                scale_px,
                position,
                bounds,
                color,
                spread,
                soft,
            };
            let key = recipe.key();
            let effects = EFFECTS.get_mut();
            let (effect, used) = effects.entry(key).or_insert_with(|| (recipe.make(key), now));
            *used = now;
            let Some(effect) = *effect else {
                continue;
            };

            let forward: f32 = index.lossy_convert();
            let rect: Rect = (
                (origin.x + effect.left + offset.x.round()) / scale,
                (origin.y + effect.top + offset.y.round()) / scale,
                effect.size.width / scale,
                effect.size.height / scale,
            )
                .into();
            IMAGE_RECT_DRAWER.get_mut().add_with_image(
                UIImageInstance::new(
                    rect,
                    (0, 0, 1, 1).into(),
                    CLEAR,
                    0.0,
                    CornerRadii::default(),
                    z - forward * LAYER_STEP,
                    false,
                    false,
                    scale,
                )
                .with_opacity(opacity),
                effect.image,
            );
        }
    }

    /// Frees the effect images no label has drawn for a while. Called once
    /// the frame's image draws are flushed.
    pub(super) fn drop_stale_text_effects() {
        let now = Window::render_frame();
        EFFECTS.get_mut().retain(|_, (effect, used)| {
            if *used + KEPT_FRAMES >= now {
                return true;
            }
            if let Some(effect) = effect {
                effect.image.free();
            }
            false
        });
    }
}
