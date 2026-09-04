//! Color glyphs. The emoji of a color font come as COLR layers or as PNG
//! strikes, and neither has a coverage mask for the text atlas. Such a
//! glyph rasterizes here into an RGBA image, drawn through the image
//! pipeline at the position the shaper gave it, while the plain glyphs
//! of the same font stay on the brush.

use log::error;
use rustybuzz::ttf_parser::{
    Face, GlyphId, NormalizedCoordinate, OutlineBuilder, RasterImageFormat, RgbaColor,
    Transform as FontTransform,
    colr::{ClipBox, ColorStop, CompositeMode, GradientExtend, Paint, Painter},
};
use tiny_skia::{
    BlendMode, Color, FillRule, GradientStop, LinearGradient, Mask, Paint as SkiaPaint, Path, PathBuilder,
    Pixmap, PixmapPaint, Point, RadialGradient, Rect, Shader, SpreadMode, SweepGradient, Transform,
};

use crate::{
    deps::refs::Weak,
    gm::{LossyConvert, flat::Size},
    window::image::{Image, Texture, TextureRawData},
};

/// A color glyph rasterized at one pixel size, ready to draw.
#[derive(Clone)]
pub(crate) struct ColorGlyph {
    pub image: Weak<Image>,
    /// The image's top left relative to the glyph origin on the
    /// baseline, in pixels, y down.
    pub left:  f32,
    pub top:   f32,
    /// The size the image draws at, in pixels.
    pub size:  Size,
}

/// The strike the bitmap tables are asked for. A bitmap font keeps a few
/// sizes and hands out the nearest one, the drawer scales it the rest of
/// the way.
fn strike_ppem(px_per_em: f32) -> u16 {
    let ppem: u32 = px_per_em.round().lossy_convert();
    u16::try_from(ppem).unwrap_or(u16::MAX).max(1)
}

/// Whether `glyph` draws through the color path at `px_per_em`. Must
/// agree with [`rasterize`] so the brush never draws what the image
/// pipeline draws too.
pub(crate) fn is_color_glyph(face: &Face, glyph: GlyphId, px_per_em: f32) -> bool {
    face.glyph_raster_image(glyph, strike_ppem(px_per_em))
        .is_some_and(|raster| raster.format == RasterImageFormat::PNG)
        || face.is_color_glyph(glyph)
}

/// A color glyph's pixels before the upload, straight alpha RGBA, with
/// the placement of the image against the glyph origin.
pub(crate) struct Raster {
    pub pixels: TextureRawData,
    pub left:   f32,
    pub top:    f32,
    pub size:   Size,
}

pub(crate) fn rasterize<'a>(
    face: &'a Face<'a>,
    font_name: &str,
    glyph: GlyphId,
    px_per_em: f32,
) -> Option<ColorGlyph> {
    let raster = raster(face, glyph, px_per_em).or_else(|| {
        error!(
            "Glyph {} of {font_name} has color data that did not render",
            glyph.0
        );
        None
    })?;
    let key = format!("color glyph {font_name} {} {px_per_em}", glyph.0);
    let TextureRawData { data, size, channels } = raster.pixels;
    let image = Image::from_raw_data(data, key, size, channels);
    Some(ColorGlyph {
        image,
        left: raster.left,
        top: raster.top,
        size: raster.size,
    })
}

/// The pixels of `glyph` at `px_per_em`, no GPU involved. A PNG strike
/// decodes as is and scales on draw, a COLR graph paints at the size.
pub(crate) fn raster<'a>(face: &'a Face<'a>, glyph: GlyphId, px_per_em: f32) -> Option<Raster> {
    if let Some(strike) = face.glyph_raster_image(glyph, strike_ppem(px_per_em))
        && strike.format == RasterImageFormat::PNG
    {
        let pixels = match Texture::parse_file_from_bytes(strike.data) {
            Ok(decoded) => decoded,
            Err(err) => {
                error!("Glyph {} has a broken PNG strike: {err}", glyph.0);
                return None;
            }
        };
        let scale = px_per_em / f32::from(strike.pixels_per_em);
        let size = Size::new(
            pixels.size.width.lossy_convert(),
            pixels.size.height.lossy_convert(),
        ) * scale;
        return Some(Raster {
            pixels,
            left: f32::from(strike.x) * scale,
            // The strike's y is its bottom edge above the baseline.
            top: -(f32::from(strike.y) + f32::from(strike.height)) * scale,
            size,
        });
    }

    if !face.is_color_glyph(glyph) {
        return None;
    }

    let px_per_unit = px_per_em / f32::from(face.units_per_em());

    // The paint graph is walked twice. The first walk only measures, so
    // the pixmap is exactly the glyph's box and its offset is known.
    let mut bounds = BoundsPainter {
        face,
        transforms: vec![Transform::identity()],
        bounds: None,
        clip_box: None,
        started: false,
    };
    face.paint_color_glyph(glyph, 0, FOREGROUND, &mut bounds)?;
    let rect = bounds.clip_box.or(bounds.bounds)?;

    let left = (rect.x_min * px_per_unit).floor();
    let right = (rect.x_max * px_per_unit).ceil();
    let top = (-rect.y_max * px_per_unit).floor();
    let bottom = (-rect.y_min * px_per_unit).ceil();
    let width: u32 = (right - left).max(0.0).lossy_convert();
    let height: u32 = (bottom - top).max(0.0).lossy_convert();
    let pixmap = Pixmap::new(width, height)?;

    // Font units are y up, the pixmap is y down.
    let base = Transform::from_row(px_per_unit, 0.0, 0.0, -px_per_unit, -left, -top);
    let mut painter = GlyphPainter {
        face,
        layers: vec![Layer {
            pixmap,
            mode: BlendMode::SourceOver,
        }],
        transforms: vec![base],
        clips: vec![],
        path: None,
    };
    face.paint_color_glyph(glyph, 0, FOREGROUND, &mut painter)?;
    let pixmap = painter.layers.pop()?.pixmap;

    Some(Raster {
        pixels: TextureRawData {
            data:     hilen_pixels::demultiply_rgba(pixmap.pixels()),
            size:     Size::new(width, height),
            channels: 4,
        },
        left,
        top,
        size: Size::new(width.lossy_convert(), height.lossy_convert()),
    })
}

/// What a palette entry of `0xFFFF` paints with, the text color in a
/// browser. Emoji fonts do not use it, so the raster is cached without
/// the label color in its key.
const FOREGROUND: RgbaColor = RgbaColor {
    red:   0,
    green: 0,
    blue:  0,
    alpha: 255,
};

fn transform(t: FontTransform) -> Transform {
    Transform::from_row(t.a, t.b, t.c, t.d, t.e, t.f)
}

fn color(color: RgbaColor) -> Color {
    Color::from_rgba8(color.red, color.green, color.blue, color.alpha)
}

fn spread(extend: GradientExtend) -> SpreadMode {
    match extend {
        GradientExtend::Pad => SpreadMode::Pad,
        GradientExtend::Repeat => SpreadMode::Repeat,
        GradientExtend::Reflect => SpreadMode::Reflect,
    }
}

/// Sorted stops squeezed into `0..=1`, with the range they covered. A
/// color line may put stops outside the unit range, the caller moves
/// the gradient geometry by the range so the colors land where the font
/// put them.
struct UnitStops {
    /// The offsets of the first and the last stop on the color line.
    min:   f32,
    max:   f32,
    stops: Vec<(f32, Color)>,
}

fn unit_stops(stops: impl Iterator<Item = ColorStop>) -> Option<UnitStops> {
    let mut stops: Vec<ColorStop> = stops.collect();
    stops.sort_by(|a, b| a.stop_offset.total_cmp(&b.stop_offset));
    let min = stops.first()?.stop_offset;
    let max = stops.last()?.stop_offset;
    let span = (max - min).max(f32::EPSILON);
    let stops = stops
        .into_iter()
        .map(|stop| {
            (
                ((stop.stop_offset - min) / span).clamp(0.0, 1.0),
                color(stop.color),
            )
        })
        .collect();
    Some(UnitStops { min, max, stops })
}

fn gradient_stops(stops: Vec<(f32, Color)>) -> Vec<GradientStop> {
    stops
        .into_iter()
        .map(|(position, color)| GradientStop::new(position, color))
        .collect()
}

fn point(x: f32, y: f32) -> Point {
    Point::from_xy(x, y)
}

fn shader<'a>(paint: &Paint<'a>, coords: &'a [NormalizedCoordinate]) -> Option<Shader<'static>> {
    match paint {
        Paint::Solid(solid) => Some(Shader::SolidColor(color(*solid))),
        Paint::LinearGradient(gradient) => {
            let UnitStops { min, max, stops } = unit_stops(gradient.stops(0, coords))?;
            let (x0, y0) = (gradient.x0, gradient.y0);
            // The color lines run parallel to p0 p2, so the gradient
            // vector is p1 projected onto the perpendicular of p0 p2.
            let (vx, vy) = (gradient.x2 - x0, gradient.y2 - y0);
            let (mut x1, mut y1) = (gradient.x1, gradient.y1);
            let length = vx.hypot(vy);
            if length > 0.0 {
                let (nx, ny) = (-vy / length, vx / length);
                let t = (x1 - x0) * nx + (y1 - y0) * ny;
                x1 = x0 + nx * t;
                y1 = y0 + ny * t;
            }
            let (dx, dy) = (x1 - x0, y1 - y0);
            LinearGradient::new(
                point(x0 + dx * min, y0 + dy * min),
                point(x0 + dx * max, y0 + dy * max),
                gradient_stops(stops),
                spread(gradient.extend),
                Transform::identity(),
            )
        }
        Paint::RadialGradient(gradient) => {
            let UnitStops { min, max, stops } = unit_stops(gradient.stops(0, coords))?;
            let (dx, dy, dr) = (
                gradient.x1 - gradient.x0,
                gradient.y1 - gradient.y0,
                gradient.r1 - gradient.r0,
            );
            RadialGradient::new(
                point(gradient.x0 + dx * min, gradient.y0 + dy * min),
                gradient.r0 + dr * min,
                point(gradient.x0 + dx * max, gradient.y0 + dy * max),
                gradient.r0 + dr * max,
                gradient_stops(stops),
                spread(gradient.extend),
                Transform::identity(),
            )
        }
        Paint::SweepGradient(gradient) => {
            let UnitStops { min, max, mut stops } = unit_stops(gradient.stops(0, coords))?;
            // Angles come in units of 180 degrees, counter clockwise.
            let span = (gradient.end_angle - gradient.start_angle) * 180.0;
            let mut start = gradient.start_angle * 180.0 + span * min;
            let mut end = gradient.start_angle * 180.0 + span * max;
            if start > end {
                (start, end) = (end, start);
                stops.reverse();
                for (position, _) in &mut stops {
                    *position = 1.0 - *position;
                }
            }
            SweepGradient::new(
                point(gradient.center_x, gradient.center_y),
                start,
                end,
                gradient_stops(stops),
                spread(gradient.extend),
                Transform::identity(),
            )
        }
    }
}

fn blend_mode(mode: CompositeMode) -> BlendMode {
    match mode {
        CompositeMode::Clear => BlendMode::Clear,
        CompositeMode::Source => BlendMode::Source,
        CompositeMode::Destination => BlendMode::Destination,
        CompositeMode::SourceOver => BlendMode::SourceOver,
        CompositeMode::DestinationOver => BlendMode::DestinationOver,
        CompositeMode::SourceIn => BlendMode::SourceIn,
        CompositeMode::DestinationIn => BlendMode::DestinationIn,
        CompositeMode::SourceOut => BlendMode::SourceOut,
        CompositeMode::DestinationOut => BlendMode::DestinationOut,
        CompositeMode::SourceAtop => BlendMode::SourceAtop,
        CompositeMode::DestinationAtop => BlendMode::DestinationAtop,
        CompositeMode::Xor => BlendMode::Xor,
        CompositeMode::Plus => BlendMode::Plus,
        CompositeMode::Screen => BlendMode::Screen,
        CompositeMode::Overlay => BlendMode::Overlay,
        CompositeMode::Darken => BlendMode::Darken,
        CompositeMode::Lighten => BlendMode::Lighten,
        CompositeMode::ColorDodge => BlendMode::ColorDodge,
        CompositeMode::ColorBurn => BlendMode::ColorBurn,
        CompositeMode::HardLight => BlendMode::HardLight,
        CompositeMode::SoftLight => BlendMode::SoftLight,
        CompositeMode::Difference => BlendMode::Difference,
        CompositeMode::Exclusion => BlendMode::Exclusion,
        CompositeMode::Multiply => BlendMode::Multiply,
        CompositeMode::Hue => BlendMode::Hue,
        CompositeMode::Saturation => BlendMode::Saturation,
        CompositeMode::Color => BlendMode::Color,
        CompositeMode::Luminosity => BlendMode::Luminosity,
    }
}

struct OutlinePath(PathBuilder);

impl OutlineBuilder for OutlinePath {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to(x, y);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0.quad_to(x1, y1, x, y);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.0.cubic_to(x1, y1, x2, y2, x, y);
    }

    fn close(&mut self) {
        self.0.close();
    }
}

fn outline(face: &Face, glyph: GlyphId) -> Option<Path> {
    let mut builder = OutlinePath(PathBuilder::new());
    face.outline_glyph(glyph, &mut builder)?;
    builder.0.finish()
}

fn clip_box_path(clip_box: ClipBox) -> Option<Path> {
    let rect = Rect::from_ltrb(clip_box.x_min, clip_box.y_min, clip_box.x_max, clip_box.y_max)?;
    Some(PathBuilder::from_rect(rect))
}

/// The measuring walk. Unions the box of every outline the graph paints,
/// under the transform it paints it with. A top level clip box bounds
/// the whole glyph, so it wins when there is one.
struct BoundsPainter<'a> {
    face:       &'a Face<'a>,
    transforms: Vec<Transform>,
    bounds:     Option<ClipBox>,
    clip_box:   Option<ClipBox>,
    started:    bool,
}

impl BoundsPainter<'_> {
    fn current(&self) -> Transform {
        *self.transforms.last().expect("the transform stack keeps its base")
    }
}

impl<'a> Painter<'a> for BoundsPainter<'_> {
    fn outline_glyph(&mut self, glyph_id: GlyphId) {
        self.started = true;
        let Some(rect) = self.face.glyph_bounding_box(glyph_id) else {
            return;
        };
        let mut corners = [
            point(f32::from(rect.x_min), f32::from(rect.y_min)),
            point(f32::from(rect.x_max), f32::from(rect.y_min)),
            point(f32::from(rect.x_min), f32::from(rect.y_max)),
            point(f32::from(rect.x_max), f32::from(rect.y_max)),
        ];
        self.current().map_points(&mut corners);
        for corner in corners {
            let bounds = self.bounds.get_or_insert(ClipBox {
                x_min: corner.x,
                y_min: corner.y,
                x_max: corner.x,
                y_max: corner.y,
            });
            bounds.x_min = bounds.x_min.min(corner.x);
            bounds.y_min = bounds.y_min.min(corner.y);
            bounds.x_max = bounds.x_max.max(corner.x);
            bounds.y_max = bounds.y_max.max(corner.y);
        }
    }

    fn paint(&mut self, _: Paint<'a>) {
        self.started = true;
    }

    fn push_clip(&mut self) {
        self.started = true;
    }

    fn push_clip_box(&mut self, clipbox: ClipBox) {
        if !self.started {
            self.clip_box = Some(clipbox);
        }
        self.started = true;
    }

    fn pop_clip(&mut self) {}

    fn push_layer(&mut self, _: CompositeMode) {
        self.started = true;
    }

    fn pop_layer(&mut self) {}

    fn push_transform(&mut self, transform: FontTransform) {
        self.started = true;
        self.transforms.push(self.current().pre_concat(self::transform(transform)));
    }

    fn pop_transform(&mut self) {
        self.transforms.pop();
    }
}

struct Layer {
    pixmap: Pixmap,
    /// How the layer composites onto the one below when it pops.
    mode:   BlendMode,
}

/// The painting walk over the same graph, into a pixmap the measured
/// size. Every fill goes through the current transform and the current
/// clip, a composite paints its two sides into layers of their own.
struct GlyphPainter<'a> {
    face:       &'a Face<'a>,
    layers:     Vec<Layer>,
    transforms: Vec<Transform>,
    clips:      Vec<Mask>,
    /// The outline a paint fills, already in pixmap space. It is set
    /// under the transform of its own `PaintGlyph`, while a paint below
    /// a `PaintTransform` sees a different current transform, which then
    /// applies to the shader alone.
    path:       Option<Path>,
}

impl GlyphPainter<'_> {
    fn current(&self) -> Transform {
        *self.transforms.last().expect("the transform stack keeps its base")
    }

    fn size(&self) -> (u32, u32) {
        let pixmap = &self.layers.first().expect("the layer stack keeps its base").pixmap;
        (pixmap.width(), pixmap.height())
    }

    fn push_clip_path(&mut self, path: Option<Path>) {
        let (width, height) = self.size();
        let mut mask = Mask::new(width, height).expect("the glyph pixmap has an area");
        if let Some(path) = path {
            match self.clips.last() {
                Some(parent) => {
                    mask = parent.clone();
                    mask.intersect_path(&path, FillRule::Winding, true, Transform::identity());
                }
                None => mask.fill_path(&path, FillRule::Winding, true, Transform::identity()),
            }
        }
        self.clips.push(mask);
    }
}

impl<'a> Painter<'a> for GlyphPainter<'a> {
    fn outline_glyph(&mut self, glyph_id: GlyphId) {
        self.path = outline(self.face, glyph_id).and_then(|path| path.transform(self.current()));
    }

    fn paint(&mut self, paint: Paint<'a>) {
        let Some(path) = &self.path else {
            return;
        };
        let Some(mut shader) = shader(&paint, self.face.variation_coordinates()) else {
            return;
        };
        shader.transform(self.current());
        let skia_paint = SkiaPaint {
            shader,
            anti_alias: true,
            ..SkiaPaint::default()
        };
        let layer = self.layers.last_mut().expect("the layer stack keeps its base");
        layer.pixmap.fill_path(
            path,
            &skia_paint,
            FillRule::Winding,
            Transform::identity(),
            self.clips.last(),
        );
    }

    fn push_clip(&mut self) {
        self.push_clip_path(self.path.clone());
    }

    fn push_clip_box(&mut self, clipbox: ClipBox) {
        self.push_clip_path(clip_box_path(clipbox).and_then(|path| path.transform(self.current())));
    }

    fn pop_clip(&mut self) {
        self.clips.pop();
    }

    fn push_layer(&mut self, mode: CompositeMode) {
        let (width, height) = self.size();
        self.layers.push(Layer {
            pixmap: Pixmap::new(width, height).expect("the glyph pixmap has an area"),
            mode:   blend_mode(mode),
        });
    }

    fn pop_layer(&mut self) {
        // The base layer never pops, a graph pops what it pushed.
        if self.layers.len() < 2 {
            return;
        }
        let Some(layer) = self.layers.pop() else {
            return;
        };
        let below = self.layers.last_mut().expect("a layer below remains");
        below.pixmap.draw_pixmap(
            0,
            0,
            layer.pixmap.as_ref(),
            &PixmapPaint {
                blend_mode: layer.mode,
                ..PixmapPaint::default()
            },
            Transform::identity(),
            None,
        );
    }

    fn push_transform(&mut self, transform: FontTransform) {
        self.transforms.push(self.current().pre_concat(self::transform(transform)));
    }

    fn pop_transform(&mut self) {
        if self.transforms.len() > 1 {
            self.transforms.pop();
        }
    }
}

#[cfg(test)]
mod test {
    use std::fs::read;

    use rustybuzz::ttf_parser::Face;

    use super::{is_color_glyph, raster};

    const FONTS: [&str; 3] = [
        "TwemojiColr0.ttf",
        "NotoColorEmojiColr1.ttf",
        "NotoColorEmojiCbdt.ttf",
    ];

    /// Every emoji the subset fonts carry renders with ink at a small, a
    /// UI and a large size, in every color format.
    #[test]
    fn every_emoji_renders_at_every_size() {
        for font in FONTS {
            let data = read(format!("{}/../assets/fonts/{font}", env!("CARGO_MANIFEST_DIR"))).unwrap();
            let face = Face::parse(&data, 0).unwrap();
            for emoji in "😀🐶🍕🚀🎉👍🌈🔥".chars() {
                let glyph = face.glyph_index(emoji).unwrap_or_else(|| panic!("{font} lacks {emoji}"));
                for px in [16.0, 40.0, 96.0] {
                    assert!(
                        is_color_glyph(&face, glyph, px),
                        "{font} {emoji} at {px} px is not a color glyph"
                    );
                    let raster = raster(&face, glyph, px)
                        .unwrap_or_else(|| panic!("{font} {emoji} at {px} px did not render"));
                    let opaque = raster.pixels.data.chunks(4).filter(|pixel| pixel[3] > 128).count();
                    assert!(opaque > 0, "{font} {emoji} at {px} px rendered no ink");
                    assert!(
                        raster.size.width > px * 0.3 && raster.size.width < px * 2.0,
                        "{font} {emoji} at {px} px is {} px wide",
                        raster.size.width
                    );
                }
            }
        }
    }
}
