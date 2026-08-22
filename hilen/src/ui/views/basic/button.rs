use std::{fmt::Display, ops::DerefMut};

use ui_proc::view;

use crate::{
    deps::refs::{Weak, weak_from_ref},
    gm::{
        ToF32,
        color::{CLEAR, Color, WHITE},
    },
    ui::{
        DynamicColor, ImageView, Label, Setup, Style, ToLabel, UIColor, UIEvent, UIManager, View,
        ViewSubviews, ViewTransition,
        view::{ViewData, ViewTouch},
    },
    window::{Font, image::ToImage},
};

/// The look a disabled button falls back to when the app sets no override.
/// Light and dark grays that read as inert against either background.
const DISABLED_COLOR: DynamicColor =
    DynamicColor::new(Color::rgb(0.839, 0.839, 0.847), Color::rgb(0.173, 0.173, 0.18));

const DISABLED_TEXT_COLOR: DynamicColor =
    DynamicColor::new(Color::rgb(0.557, 0.557, 0.576), Color::rgb(0.443, 0.443, 0.467));

#[view]
pub struct Button {
    on_tap: UIEvent,

    #[educe(Default = true)]
    enabled: bool,

    /// The colors the button had when it was disabled, put back when it is
    /// enabled again. Stored as `UIColor` so a theme pair survives the trip.
    enabled_color:      Option<UIColor>,
    enabled_text_color: Option<UIColor>,

    disabled_color:      Option<UIColor>,
    disabled_text_color: Option<UIColor>,

    label: Weak<Label>,

    #[init]
    image: ImageView,
}

impl Button {
    pub fn text(&self) -> &str {
        self.label.text()
    }

    pub fn set_text(&self, text: impl ToLabel) -> &Self {
        self.label.set_hidden(false);
        self.label.set_text(text);
        self
    }

    pub fn text_color(&self) -> &Color {
        self.label.text_color()
    }

    pub fn set_text_color(&self, color: impl Into<UIColor>) -> &Self {
        self.label.set_text_color(color);
        self
    }

    pub fn text_size(&self) -> f32 {
        self.label.text_size()
    }

    pub fn set_text_size(&self, size: impl ToF32) -> &Self {
        weak_from_ref(self).label.set_text_size(size);
        self
    }

    pub fn set_letter_spacing(&self, spacing: impl ToF32) -> &Self {
        weak_from_ref(self).label.set_letter_spacing(spacing);
        self
    }

    pub fn set_font(&self, font: Weak<Font>) -> &Self {
        weak_from_ref(self).label.set_font(font);
        self
    }
}

impl Button {
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// A disabled button takes on the disabled colors and stops firing
    /// `on_tap`. It still receives touches, so it does not become a hole
    /// that whatever sits behind it starts catching.
    ///
    /// The colors it had are put back when it is enabled again. Set the
    /// normal colors while the button is enabled, a `set_color` call made
    /// while it is disabled is overwritten by that restore.
    pub fn set_enabled(&mut self, enabled: bool) -> &mut Self {
        if self.enabled == enabled {
            return self;
        }

        self.enabled = enabled;

        if enabled {
            if let Some(color) = self.enabled_color.take() {
                self.set_color(color);
            }
            if let Some(color) = self.enabled_text_color.take() {
                self.set_text_color(color);
            }
        } else {
            self.enabled_color = Some(self.ui_color());
            self.enabled_text_color = Some(self.label.ui_text_color());
            self.set_color(self.disabled_color.unwrap_or(DISABLED_COLOR.into()));
            self.set_text_color(self.disabled_text_color.unwrap_or(DISABLED_TEXT_COLOR.into()));
        }

        self
    }

    pub fn set_disabled_color(&mut self, color: impl Into<UIColor>) -> &mut Self {
        self.disabled_color = Some(color.into());
        if !self.enabled {
            self.set_color(self.disabled_color.unwrap());
        }
        self
    }

    pub fn set_disabled_text_color(&mut self, color: impl Into<UIColor>) -> &mut Self {
        self.disabled_text_color = Some(color.into());
        if !self.enabled {
            self.set_text_color(self.disabled_text_color.unwrap());
        }
        self
    }

    pub fn on_tap<R>(&self, mut action: impl FnMut() -> R + Send + 'static) -> &Self {
        self.enable_touch();
        self.on_tap.sub(self.weak(), move || {
            action();
        });
        self
    }

    pub fn set_image(&self, image: impl ToImage) -> &Self {
        self.image.set_hidden(false);
        self.image.set_image(image);
        self
    }

    pub fn set_resizing_image(&mut self, name: impl Display) -> &mut Self {
        self.set_color(CLEAR);
        self.image.set_hidden(false);
        self.image.set_resizing_image(name);
        self
    }
}

impl Button {
    pub fn add_transition<From: View + 'static, To: View + Default + 'static>(
        self: Weak<Self>,
    ) -> Weak<Self> {
        self.on_tap(move || {
            let from = self.find_superview::<From>();
            let mut to = To::new();
            from.transition_to(to.deref_mut());
            UIManager::set_view(to);
        });
        self
    }
}

impl Setup for Button {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);

        let label = Label::new();

        label.__base_view().is_system = true;
        label.__base_view().ignore_global_style = true;

        self.label = self.add_subview(label).downcast_view().unwrap();

        self.label.place().back();
        self.label.set_color(CLEAR);
        self.label.set_hidden(true);

        self.image.place().back();
        self.image.set_hidden(true);
        self.image.__base_view().is_system = true;

        self.touch().up_inside.sub(self, move || {
            if self.enabled {
                self.on_tap.trigger(());
            }
        });

        Style::apply_global(self);
    }
}

#[macro_export]
macro_rules! link_button {
    ($self:ident, $($button:ident).+, $method:ident) => {{
        use hilen::ui::AlertErr;
        $self.$($button).+.on_tap(move || { $self.$method().alert_err(); });
    }}
}

#[macro_export]
macro_rules! async_call {
    ($self:ident, $method:ident) => {
        tokio::spawn(async move {
            $self.$method().await.alert_err();
        });
    };
}
