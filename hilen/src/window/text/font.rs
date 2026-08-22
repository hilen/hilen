use anyhow::{Result, anyhow, bail};
use log::error;
use rustybuzz::{Face, ttf_parser::Tag};
use wgpu::{
    CompareFunction, DepthBiasState, DepthStencilState, MultisampleState, StencilState, TextureFormat,
};
use wgpu_text::{
    BrushBuilder, Section, Text, TextBrush,
    glyph_brush::ab_glyph::{Font as AbGlyphFont, FontArc, FontRef, VariableFont},
};

use crate::{
    deps::refs::{
        Weak,
        main_lock::MainLock,
        manage::{DataManager, ResourceLoader},
    },
    filesystem::read_bytes as read,
    gm::{LossyConvert, ToF32, flat::Size},
    managed,
    window::{
        msaa_sample_count, surface_texture_format,
        text::{ShapedLayout, ShapedParams},
        window::Window,
    },
};

pub struct Font {
    pub name:  String,
    pub brush: TextBrush,
    face:      Face<'static>,
    /// `ab_glyph` `PxScale` is ascent minus descent in pixels, while text
    /// sizes everywhere else, CSS included, mean pixels per em. This
    /// factor converts an em size into the `PxScale` that renders it.
    em_scale:  f32,
}

impl Font {
    fn new(name: impl ToString, data: &[u8]) -> Result<Self> {
        Self::new_with_variations(name, data, &[])
    }

    fn new_with_variations(name: impl ToString, data: &[u8], variations: &[([u8; 4], f32)]) -> Result<Self> {
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

        let brush = BrushBuilder::using_font(font).with_depth_stencil( DepthStencilState {
            format:              TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare:       Some(CompareFunction::Less),
            stencil:             StencilState::default(),
            bias:                DepthBiasState::default(),
        }.into())
            .with_multisample(MultisampleState {
                count:                     msaa_sample_count(),
                mask:                      !0,
                alpha_to_coverage_enabled: false,
            })
            /* .initial_cache_size((16_384, 16_384))) */ // use this to avoid resizing cache texture
            .build(&window.device, render_size.width.lossy_convert(), render_size.height.lossy_convert(), surface_texture_format());
        Ok(Self {
            name: name.to_string(),
            brush,
            face,
            em_scale,
        })
    }

    /// Converts a pixels per em text size into the `ab_glyph` `PxScale`
    /// that renders it.
    pub(crate) fn em_scale(&self) -> f32 {
        self.em_scale
    }

    /// Size the text takes when drawn at `size`. `width` bounds wrapping,
    /// `None` measures a single unbounded line. Layout params must mirror
    /// `draw_label` or measured sizes will not match rendering.
    pub(crate) fn measure(
        &mut self,
        text: &str,
        size: impl ToF32,
        width: Option<f32>,
        tracking: f32,
    ) -> Size {
        if text.is_empty() {
            return Size::default();
        }

        let section = Section::new()
            .add_text(Text::new(text).with_scale(size.to_f32() * self.em_scale))
            .with_bounds((width.unwrap_or(f32::INFINITY), f32::INFINITY));

        let layout = ShapedLayout {
            face:      &self.face,
            font_name: &self.name,
            params:    ShapedParams {
                tracking,
                multiline: width.is_some(),
                h_align: wgpu_text::glyph_brush::HorizontalAlign::Left,
            },
        };

        let Some(bounds) = self.brush.glyph_bounds_with_layout(section, &layout) else {
            return Size::default();
        };

        Size::new(bounds.width(), bounds.height())
    }

    /// Queues a section shaped with this font's face. Call
    /// [`Font::process_queued`] once per frame after all sections.
    pub(crate) fn queue_shaped(&mut self, section: Section, params: ShapedParams) {
        let layout = ShapedLayout {
            face: &self.face,
            font_name: &self.name,
            params,
        };
        self.brush.queue_section_with_layout(section, &layout);
    }

    pub(crate) fn process_queued(&mut self) -> Result<()> {
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
        Self::store_with_name(name, || Self::new_with_variations(name, data, variations))
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
