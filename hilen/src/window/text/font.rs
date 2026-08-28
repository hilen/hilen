use anyhow::{Result, anyhow, bail};
use log::error;
use rustybuzz::{Face, ttf_parser::Tag};
use wgpu::MultisampleState;
use wgpu_text::{
    BrushBuilder, Section, Text, TextBrush,
    glyph_brush::ab_glyph::{Font as AbGlyphFont, FontArc, FontRef, PxScale, VariableFont},
};

use crate::{
    deps::refs::{
        Weak,
        main_lock::MainLock,
        manage::{DataManager, ResourceLoader},
        weak_from_ref,
    },
    filesystem::read_bytes as read,
    gm::{LossyConvert, ToF32, flat::Size},
    managed,
    render::depth_stencil_state,
    window::{
        msaa_sample_count, surface_texture_format,
        text::{FontRun, ShapeCache, ShapedLayout, ShapedParams, TextLayout, VerticalAlign},
        window::Window,
    },
};

pub struct Font {
    pub name:    String,
    pub brush:   TextBrush,
    /// The same font the brush rasterizes with, kept for measuring while
    /// the brush is busy.
    ab:          FontArc,
    face:        Face<'static>,
    /// `ab_glyph` `PxScale` is ascent minus descent in pixels, while text
    /// sizes everywhere else, CSS included, mean pixels per em. This
    /// factor converts an em size into the `PxScale` that renders it.
    em_scale:    f32,
    shape_cache: MainLock<ShapeCache>,
}

impl Font {
    fn new(name: impl ToString, data: &[u8]) -> Result<Self> {
        Self::new_with_variations(name, data, &[], 0.0)
    }

    fn new_with_variations(
        name: impl ToString,
        data: &[u8],
        variations: &[([u8; 4], f32)],
        stem_darkening: f32,
    ) -> Result<Self> {
        let window = Window::current();

        let render_size = Window::render_size();

        // Managed fonts live until process exit, leaking gives the raster
        // font and the shaping face one shared 'static copy of the data.
        let data: &'static [u8] = Vec::leak(data.to_vec());

        let mut font = FontRef::try_from_slice(data)?;
        let mut face = Face::from_slice(data, 0)
            .ok_or_else(|| anyhow!("Failed to parse font '{}' for shaping", name.to_string()))?;

        for (tag, value) in variations {
            let axis = String::from_utf8_lossy(tag);
            if !font.set_variation(tag, *value) {
                bail!("Font '{}' has no {axis} axis", name.to_string());
            }
            face.set_variation(Tag::from_bytes(tag), *value)
                .ok_or_else(|| anyhow!("Shaping face of '{}' rejected {axis} axis", name.to_string()))?;
        }

        let font = FontArc::new(font);

        let units_per_em = font
            .units_per_em()
            .ok_or_else(|| anyhow!("Font '{}' has no units per em", name.to_string()))?;
        let em_scale = font.height_unscaled() / units_per_em;

        let brush = BrushBuilder::using_font(font.clone())
            .with_depth_stencil(depth_stencil_state().into())
            .with_multisample(MultisampleState {
                count:                     msaa_sample_count(),
                mask:                      !0,
                alpha_to_coverage_enabled: false,
            })
            .with_stem_darkening(stem_darkening)
            /* .initial_cache_size((16_384, 16_384))) */ // use this to avoid resizing cache texture
            .build(&window.device, render_size.width.lossy_convert(), render_size.height.lossy_convert(), surface_texture_format());
        Ok(Self {
            name: name.to_string(),
            brush,
            ab: font,
            face,
            em_scale,
            shape_cache: MainLock::new(),
        })
    }

    /// Converts a pixels per em text size into the `ab_glyph` `PxScale`
    /// that renders it.
    pub(crate) fn em_scale(&self) -> f32 {
        self.em_scale
    }

    pub(crate) fn face(&self) -> &Face<'static> {
        &self.face
    }

    pub(crate) fn ab(&self) -> &FontArc {
        &self.ab
    }

    pub(crate) fn shape_cache(&self) -> &MainLock<ShapeCache> {
        &self.shape_cache
    }

    fn params(
        &self,
        tracking: f32,
        width: Option<f32>,
        runs: Vec<FontRun>,
        line_height: Option<f32>,
    ) -> ShapedParams {
        ShapedParams {
            tracking,
            multiline: width.is_some(),
            h_align: wgpu_text::glyph_brush::HorizontalAlign::Left,
            v_align: VerticalAlign::Center,
            line_height,
            base: weak_from_ref(self),
            runs,
        }
    }

    /// Size the text takes when drawn at `size`. `width` bounds wrapping,
    /// `None` measures a single unbounded line. `runs` are the byte
    /// ranges drawn with other fonts. Layout params must mirror
    /// `draw_label` or measured sizes will not match rendering.
    pub(crate) fn measure(
        &mut self,
        text: &str,
        size: impl ToF32,
        width: Option<f32>,
        tracking: f32,
        runs: Vec<FontRun>,
        line_height: Option<f32>,
    ) -> Size {
        if text.is_empty() {
            return Size::default();
        }

        let px_scale = size.to_f32() * self.em_scale;
        let section = Section::new()
            .add_text(Text::new(text).with_scale(px_scale))
            .with_bounds((width.unwrap_or(f32::INFINITY), f32::INFINITY));

        let layout = ShapedLayout {
            emit:   &self.name,
            params: self.params(tracking, width, runs, line_height),
        };

        let Some(bounds) = self.brush.glyph_bounds_with_layout(section, &layout) else {
            return Size::default();
        };

        // With a custom line box the height is count boxes. The glyph
        // bounds cover ascent to descent, one `px_scale`, so swapping
        // that for one box turns baseline span into box count times
        // the box.
        let height = match line_height {
            Some(line_height) => bounds.height() - px_scale + line_height,
            None => bounds.height(),
        };

        Size::new(bounds.width(), height)
    }

    /// Line and caret positions of `text` drawn at `size`, in the same
    /// pixels `measure` reports. `width` bounds wrapping like in `measure`.
    pub(crate) fn text_layout(
        &self,
        text: &str,
        size: impl ToF32,
        width: Option<f32>,
        tracking: f32,
        runs: Vec<FontRun>,
    ) -> TextLayout {
        let layout = ShapedLayout {
            emit:   &self.name,
            params: self.params(tracking, width, runs, None),
        };

        let scale = PxScale::from(size.to_f32() * self.em_scale);
        layout.text_layout(scale, text, width.unwrap_or(f32::INFINITY))
    }

    /// Queues a section laid out with the base font of `params` and
    /// drawn with this font, which keeps only its own glyphs. Call
    /// [`Font::process_queued`] once per frame after all sections.
    pub(crate) fn queue_shaped(&mut self, section: Section, params: ShapedParams) {
        let layout = ShapedLayout {
            emit: &self.name,
            params,
        };
        self.brush.queue_section_with_layout(section, &layout);
    }

    pub(crate) fn process_queued(&mut self) -> Result<()> {
        self.shape_cache.get_mut().sweep();
        self.brush.process_queued(&Window::current().device, Window::queue())?;
        Ok(())
    }
}

static DEFAULT_FONT: MainLock<Option<Weak<Font>>> = MainLock::new();

impl Font {
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Weak<Font> {
        if let Some(font) = *DEFAULT_FONT
            && font.is_ok()
        {
            return font;
        }
        Self::roboto()
    }

    pub fn set_default(font: Weak<Font>) {
        *DEFAULT_FONT.get_mut() = Some(font);
    }

    pub fn reset_default() {
        *DEFAULT_FONT.get_mut() = None;
    }

    /// Loads a variable font with the given axis values, for example
    /// weight `(*b"wght", 600.0)`, optical size `(*b"opsz", 17.0)` or
    /// grade `(*b"GRAD", 430.0)`. Each combination is a separate managed
    /// instance, cache it under a name that includes the values.
    pub fn with_variations(name: &str, data: &[u8], variations: &[([u8; 4], f32)]) -> Result<Weak<Font>> {
        Self::store_with_name(name, || Self::new_with_variations(name, data, variations, 0.0))
    }

    /// Like [`Font::with_variations`], with the glyph coverage boosted to
    /// approximate the stem darkening platform rasterizers like `CoreText`
    /// apply. Ports matching browser text on macOS need it, plain engine
    /// text does not.
    pub fn with_variations_darkened(
        name: &str,
        data: &[u8],
        variations: &[([u8; 4], f32)],
        darkening: f32,
    ) -> Result<Weak<Font>> {
        Self::store_with_name(name, || {
            Self::new_with_variations(name, data, variations, darkening)
        })
    }

    pub fn roboto() -> Weak<Font> {
        Self::store_with_name("Roboto-Regular.ttf", || {
            Self::new("Roboto-Regular.ttf", include_bytes!("fonts/Roboto-Regular.ttf"))
        })
        .expect("Failed to load Roboto font")
    }
}

managed!(Font);

static DEFAULT_FONT_DATA: &[u8] = include_bytes!("fonts/Roboto-Regular.ttf");

impl ResourceLoader for Font {
    fn load_path(path: &std::path::Path) -> Self {
        let data = read(path);

        let data = data
            .as_ref()
            .map(Vec::as_slice)
            .inspect_err(|err| {
                error!(
                    "Failed to read font file: {}. Error: {err} Returning default font",
                    path.display()
                );
            })
            .unwrap_or(DEFAULT_FONT_DATA);

        Self::load_data(data, path.display())
    }

    fn load_data(data: &[u8], name: impl ToString) -> Self {
        let name = name.to_string();

        // Bad bytes must degrade like an unreadable file does above. A
        // corrupt download would otherwise kill a browser session.
        match Font::new(&name, data) {
            Ok(font) => font,
            Err(err) => {
                error!("Failed to load font {name}: {err}. Returning default font");
                Font::new(name, DEFAULT_FONT_DATA).expect("Failed to load the built in default font")
            }
        }
    }
}
