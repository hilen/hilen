use std::{fmt::Display, sync::atomic::Ordering};

use atomic_float::AtomicF32;
use refs::{Weak, weak_from_ref};
use ui_proc::view;

use crate::{
    gm::{
        ToF32,
        color::{BLACK, Color},
        flat::Size,
    },
    ui::{
        DynamicColor, ImageView, Setup, Style, ToLabel, UIColor, UIManager, View, ViewCallbacks, ViewFrame,
        view::{ViewData, ViewSubviews},
    },
    window::{Font, image::ToImage},
};

static DEFAULT_TEXT_SIZE: AtomicF32 = AtomicF32::new(16.0);

#[derive(Debug, Default)]
pub enum TextAlignment {
    Left,
    #[default]
    Center,
    Right,
}

impl TextAlignment {
    pub fn center(&self) -> bool {
        matches!(self, Self::Center)
    }
}

#[view]
pub struct Label {
    pub alignment: TextAlignment,

    pub text: String,

    multiline: bool,

    #[educe(Default = BLACK)]
    text_color: Color,

    dynamic_text_color: Option<DynamicColor>,

    /// The bottom of the glyph ramp, `None` for flat text. The top is
    /// `text_color`, so a gradient and a plain color cannot both be set.
    text_end_color: Option<Color>,

    dynamic_text_end_color: Option<DynamicColor>,

    #[educe(Default = DEFAULT_TEXT_SIZE.load(Ordering::Relaxed))]
    text_size: f32,

    /// Extra points between glyphs, CoreText style tracking. Negative
    /// tightens. Needed to match fonts whose tracking the platform
    /// applies from the trak table, like SF Pro on macOS.
    letter_spacing: f32,

    font: Weak<Font>,
}

impl Label {
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn set_text(&self, text: impl ToLabel) -> &Self {
        weak_from_ref(self).text = text.to_label();
        self
    }

    pub fn text_color(&self) -> &Color {
        &self.text_color
    }

    pub fn set_text_color(&self, color: impl Into<UIColor>) -> &Self {
        let mut this = weak_from_ref(self);
        match color.into() {
            UIColor::Plain(color) => {
                this.text_color = color;
                this.dynamic_text_color = None;
            }
            UIColor::Dynamic(color) => {
                this.text_color = color.resolve();
                this.dynamic_text_color = Some(color);
            }
        }
        this.text_end_color = None;
        this.dynamic_text_end_color = None;
        self
    }

    pub(crate) fn text_end_color(&self) -> Option<Color> {
        self.text_end_color
    }

    /// Fades the glyphs from `start` at the top of the label frame to `end` at
    /// its bottom. This is what CSS paints with a linear gradient and
    /// `background-clip: text`. [`Label::set_text_color`] clears it.
    pub fn set_text_gradient(&self, start: impl Into<UIColor>, end: impl Into<UIColor>) -> &Self {
        self.set_text_color(start);

        let mut this = weak_from_ref(self);
        match end.into() {
            UIColor::Plain(color) => {
                this.text_end_color = Some(color);
                this.dynamic_text_end_color = None;
            }
            UIColor::Dynamic(color) => {
                this.text_end_color = Some(color.resolve());
                this.dynamic_text_end_color = Some(color);
            }
        }
        self
    }

    pub(crate) fn text_size(&self) -> f32 {
        self.text_size
    }

    pub fn set_text_size(&self, size: impl ToF32) -> &Self {
        weak_from_ref(self).text_size = size.to_f32();
        self
    }

    pub(crate) fn letter_spacing(&self) -> f32 {
        self.letter_spacing
    }

    pub fn set_letter_spacing(&self, spacing: impl ToF32) -> &Self {
        weak_from_ref(self).letter_spacing = spacing.to_f32();
        self
    }

    pub(crate) fn font(&self) -> Weak<Font> {
        if self.font.is_ok() {
            self.font
        } else {
            Font::default()
        }
    }

    pub fn set_font(&self, font: Weak<Font>) -> &Self {
        weak_from_ref(self).font = font;
        self
    }

    /// Size the label's frame needs to show the current text. Multiline
    /// wraps at the current frame width.
    pub fn content_size(&self) -> Size {
        self.size_for_width(self.width())
    }

    /// Size the label's frame needs at the given frame width. For multiline
    /// this is how auto-height panels measure before layout.
    pub fn size_for_width(&self, width: f32) -> Size {
        let margin = self.alignment_margin();
        let bound = self.multiline.then_some(width - margin);
        let measured = self.font().measure(&self.text, self.text_size, bound, self.letter_spacing);

        if measured.has_no_area() {
            return measured;
        }

        Size::new(measured.width + margin, measured.height)
    }

    // The drawer indents left and right aligned text by 16 physical pixels,
    // so the fitted frame must include it or the text clips.
    fn alignment_margin(&self) -> f32 {
        if self.alignment.center() {
            0.0
        } else {
            16.0 / UIManager::scale()
        }
    }
}

impl Label {
    pub fn set_alignment(&self, alignment: TextAlignment) -> &Self {
        weak_from_ref(self).alignment = alignment;
        self
    }

    pub(crate) fn is_multiline(&self) -> bool {
        self.multiline
    }

    pub fn set_multiline(&self, multiline: bool) -> &Self {
        weak_from_ref(self).multiline = multiline;
        self
    }

    pub fn set_image(&self, image: impl ToImage) -> &Self {
        self.remove_all_subviews();
        let image_view = self.add_view::<ImageView>();
        image_view.place().back();
        image_view.set_image(image);
        image_view.__base_view().z_position = self.z_position();

        self
    }

    pub fn set_resizing_image(&mut self, name: impl Display) -> &mut Self {
        self.remove_all_subviews();
        let mut image_view = self.add_view::<ImageView>();
        image_view.place().back();
        image_view.set_resizing_image(name);
        image_view.__base_view().z_position = self.z_position();
        image_view.subviews_weak().iter_mut().for_each(|v| {
            v.__base_view().z_position = self.z_position();
            v.subviews_weak().iter_mut().for_each(|v| {
                v.__base_view().z_position = self.z_position();
            });
        });

        self
    }
}

impl Label {
    pub fn set_default_text_size(size: impl ToF32) {
        DEFAULT_TEXT_SIZE.store(size.to_f32(), Ordering::Relaxed);
    }

    /// The test harness forces its own size and has to put this back, or every
    /// label in the app keeps the harness size after a run.
    pub(crate) fn default_text_size() -> f32 {
        DEFAULT_TEXT_SIZE.load(Ordering::Relaxed)
    }
}

impl Setup for Label {
    fn setup(self: Weak<Self>) {
        Style::apply_global(self);
    }
}

impl ViewCallbacks for Label {
    fn theme_changed(&mut self) {
        if let Some(color) = self.dynamic_text_color {
            self.text_color = color.resolve();
        }
        if let Some(color) = self.dynamic_text_end_color {
            self.text_end_color = Some(color.resolve());
        }
    }
}

pub trait AddLabel {
    fn add_label(&self, text: impl ToLabel) -> &Self;
}

impl<T: ?Sized + View> AddLabel for T {
    fn add_label(&self, text: impl ToLabel) -> &Self {
        let mut label = self.add_view::<Label>();
        label.place().center().h(20).lr(0);
        label.text = text.to_label();
        self
    }
}
