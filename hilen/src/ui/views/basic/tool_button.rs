use std::fmt::Display;

use ui_proc::view;

use crate::{
    deps::refs::{Weak, weak_from_ref},
    gm::{
        ToF32,
        color::{CLEAR, Color},
    },
    ui::{
        DynamicColor, ImageView, Label, Setup, Tinted, ToLabel, Touch, UIColor, UIEvent, View, ViewCallbacks,
        ViewSubviews,
        view::{ViewData, ViewFrame, ViewTouch},
    },
    window::{
        Font,
        image::{Image, tint_svg},
    },
};

const TEXT: DynamicColor = DynamicColor::new(Color::hex("#1C1C1E"), Color::hex("#F2F2F4"));
const ON_TEXT: DynamicColor = DynamicColor::new(Color::hex("#0A6CFF"), Color::hex("#5AA5FF"));
const DISABLED_TEXT: DynamicColor = DynamicColor::new(Color::hex("#B4B4BC"), Color::hex("#5C5C64"));
const HOVER: DynamicColor = DynamicColor::new(Color::hex("#ECECF0"), Color::hex("#323236"));
const PRESSED: DynamicColor = DynamicColor::new(Color::hex("#D6D6DD"), Color::hex("#46464C"));
const ON: DynamicColor = DynamicColor::new(Color::hex("#E1ECFF"), Color::hex("#1F3352"));
const ON_HOVER: DynamicColor = DynamicColor::new(Color::hex("#D2E2FF"), Color::hex("#284068"));

/// How wide a `ToolButton` is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Width {
    /// As wide as its icon and text.
    #[default]
    Fit,
    /// The width its owner gives it.
    Fill,
}

const PADDING: f32 = 10.0;
const GAP: f32 = 6.0;
const ICON_SIZE: f32 = 16.0;
/// A picture like an avatar has to show a face, so it is larger than an
/// icon. Its center stays where the center of an icon is.
const PICTURE_SIZE: f32 = 24.0;
const HEIGHT: f32 = 32.0;
const TEXT_SIZE: f32 = 14.0;
const RADIUS: f32 = 6.0;

/// A button with an icon on the left and a text after it, as wide as
/// both need. The entry of a toolbar or a menu. The icon is an SVG with
/// black in it, tinted like `Tinted`, from `assets/images` or from bytes
/// the app holds. `set_on` gives the look of a toggle that is on. The fills
/// and the font follow the engine look unless the app gives its own.
#[view]
pub struct ToolButton {
    on_tap: UIEvent,

    icon_name: Option<String>,
    /// The SVG of an icon the app built in, none for an icon from a file.
    icon_data: Option<&'static [u8]>,
    /// A picture in place of the icon, drawn round in its own colors.
    picture:   Option<Weak<Image>>,

    #[educe(Default = true)]
    enabled: bool,
    on:      bool,
    pressed: bool,
    /// Fits its icon and text, or takes the width its owner gives it, see
    /// `set_fills_width`.
    width:   Width,

    tint:    Option<UIColor>,
    on_tint: Option<UIColor>,

    hover_color:    Option<UIColor>,
    pressed_color:  Option<UIColor>,
    on_color:       Option<UIColor>,
    on_hover_color: Option<UIColor>,

    label: Weak<Label>,

    #[init]
    icon: ImageView,
}

impl ToolButton {
    pub fn text(&self) -> &str {
        self.label.text()
    }

    pub fn set_text(&self, text: impl ToLabel) -> &Self {
        let text = text.to_label();
        self.label.set_hidden(text.is_empty());
        self.label.set_text(text);
        self.relayout();
        self
    }

    pub fn set_text_size(&self, size: impl ToF32) -> &Self {
        self.label.set_text_size(size);
        self.relayout();
        self
    }

    /// The name of an SVG in `assets/images` drawn in black, see Icons in
    /// the views chapter.
    pub fn set_icon(&self, name: impl Display) -> &Self {
        weak_from_ref(self).icon_name = Some(name.to_string());
        self.icon.set_hidden(false);
        self.relayout();
        self.refresh();
        self
    }

    /// An icon from SVG bytes the app holds, like one built in with
    /// `include_bytes`, drawn in black and tinted like a named icon. The
    /// name keeps its tinted pictures apart from other icons.
    pub fn set_icon_data(&self, data: &'static [u8], name: impl Display) -> &Self {
        let mut this = weak_from_ref(self);
        this.icon_data = Some(data);
        this.icon_name = Some(name.to_string());
        self.icon.set_hidden(false);
        self.relayout();
        self.refresh();
        self
    }

    pub fn set_font(&self, font: Weak<Font>) -> &Self {
        self.label.set_font(font);
        self.relayout();
        self
    }

    /// The fill under the pointer. The default is the hover gray of the theme.
    pub fn set_hover_color(&self, color: impl Into<UIColor>) -> &Self {
        weak_from_ref(self).hover_color = Some(color.into());
        self.refresh();
        self
    }

    /// The fill while it is pressed.
    pub fn set_pressed_color(&self, color: impl Into<UIColor>) -> &Self {
        weak_from_ref(self).pressed_color = Some(color.into());
        self.refresh();
        self
    }

    /// The fill while the button is on.
    pub fn set_on_color(&self, color: impl Into<UIColor>) -> &Self {
        weak_from_ref(self).on_color = Some(color.into());
        self.refresh();
        self
    }

    /// The fill of a button that is on while the pointer is over it, so a
    /// picked entry of a switch stays marked under the pointer. Without it
    /// an app's own on fill stays as it is, the engine on fill gets a step
    /// darker.
    pub fn set_on_hover_color(&self, color: impl Into<UIColor>) -> &Self {
        weak_from_ref(self).on_hover_color = Some(color.into());
        self.refresh();
        self
    }

    /// The color of the icon and the text. A `DynamicColor` follows the
    /// theme. The default is the text color of the theme.
    pub fn set_tint(&self, tint: impl Into<UIColor>) -> &Self {
        weak_from_ref(self).tint = Some(tint.into());
        self.refresh();
        self
    }

    /// A picture in place of the icon, like the avatar of the user, drawn
    /// round and in its own colors, never tinted, 24 points where an icon is
    /// 16. `None` brings the icon back.
    pub fn set_picture(&self, picture: Option<Weak<Image>>) -> &Self {
        weak_from_ref(self).picture = picture;
        let radius = if picture.is_some() {
            PICTURE_SIZE / 2.0
        } else {
            0.0
        };
        self.icon.set_corner_radius(radius);
        self.icon.set_hidden(picture.is_none() && self.icon_name.is_none());
        self.relayout();
        self.refresh();
        self
    }

    /// The color of the icon and the text while the button is on.
    pub fn set_on_tint(&self, tint: impl Into<UIColor>) -> &Self {
        weak_from_ref(self).on_tint = Some(tint.into());
        self.refresh();
        self
    }

    /// A button that takes the width its owner gives it, like a row of a
    /// menu, with its icon and text at the left. Its fill, hover and on look
    /// cover the whole row. Off by default, the button is as wide as its
    /// icon and text.
    pub fn set_fills_width(&self, fills: bool) -> &Self {
        weak_from_ref(self).width = if fills { Width::Fill } else { Width::Fit };
        if fills {
            self.place().drop_width();
        }
        self.relayout();
        self
    }

    pub fn is_on(&self) -> bool {
        self.on
    }

    /// The look of a toggle that is on. A tap does not change it, the
    /// owner calls this from `on_tap`.
    pub fn set_on(&self, on: bool) -> &Self {
        weak_from_ref(self).on = on;
        self.refresh();
        self
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// A disabled button draws in a faint color and fires no `on_tap`. It
    /// still takes touches, so what is behind it gets none.
    pub fn set_enabled(&self, enabled: bool) -> &Self {
        let mut this = weak_from_ref(self);
        this.enabled = enabled;
        this.pressed = false;
        self.refresh();
        self
    }

    pub fn on_tap<R>(&self, mut action: impl FnMut() -> R + Send + 'static) -> &Self {
        self.on_tap.sub(self.weak(), move || {
            action();
        });
        self
    }
}

impl ToolButton {
    fn tint(&self) -> UIColor {
        if !self.enabled {
            DISABLED_TEXT.into()
        } else if self.on {
            self.on_tint.unwrap_or(ON_TEXT.into())
        } else {
            self.tint.unwrap_or(TEXT.into())
        }
    }

    fn background(&self) -> UIColor {
        if !self.enabled {
            CLEAR.into()
        } else if self.pressed {
            self.pressed_color.unwrap_or(PRESSED.into())
        } else if self.on && self.is_hovered() {
            self.on_hover_color.or(self.on_color).unwrap_or(ON_HOVER.into())
        } else if self.on {
            self.on_color.unwrap_or(ON.into())
        } else if self.is_hovered() {
            self.hover_color.unwrap_or(HOVER.into())
        } else {
            CLEAR.into()
        }
    }

    fn refresh(&self) {
        let tint = self.tint();
        self.label.set_text_color(tint);
        if let Some(picture) = self.picture {
            self.icon.set_image(picture);
            self.set_color(self.background());
            return;
        }
        match (self.icon_data, &self.icon_name) {
            (Some(data), Some(name)) => {
                self.icon.set_image(tint_svg(data, name, tint.resolve()));
            }
            (None, Some(name)) => {
                self.icon.set_image(Tinted {
                    tint: tint.resolve(),
                    name: name.clone(),
                });
            }
            _ => {}
        }
        self.set_color(self.background());
    }

    /// The width the icon and the text take with their padding, the width
    /// the button gives itself. For an owner that sets frames by hand.
    pub fn fit_width(&self) -> f32 {
        let (icon, gap, text) = self.parts();
        PADDING + icon + gap + text + PADDING
    }

    /// The left edge of the icon or the picture and its side.
    fn icon_frame(&self) -> (f32, f32) {
        if self.picture.is_some() {
            (PADDING - (PICTURE_SIZE - ICON_SIZE) / 2.0, PICTURE_SIZE)
        } else {
            (PADDING, ICON_SIZE)
        }
    }

    /// The space the icon takes from the left padding on, the width of the
    /// text and the gap between them.
    fn parts(&self) -> (f32, f32, f32) {
        let icon = if self.icon_name.is_some() || self.picture.is_some() {
            let (left, side) = self.icon_frame();
            left + side - PADDING
        } else {
            0.0
        };
        let text = if self.label.text().is_empty() {
            0.0
        } else {
            self.label.content_size().width
        };
        let gap = if icon > 0.0 && text > 0.0 { GAP } else { 0.0 };
        (icon, gap, text)
    }

    /// The width follows the icon and the text. The height stays what the
    /// owner placed, 32 points by default.
    fn relayout(&self) {
        let (icon, gap, text) = self.parts();
        let (left, side) = self.icon_frame();
        self.icon.place().clear().l(left).center_y().size(side, side);
        self.label.place().clear().l(PADDING + icon + gap).tb(0).w(text);
        if self.width == Width::Fit {
            self.place().w(self.fit_width());
        }
    }

    fn touched(&self, touch: Touch) {
        if !self.enabled {
            return;
        }
        let size = self.size();
        let inside = touch.position.x >= 0.0
            && touch.position.y >= 0.0
            && touch.position.x <= size.width
            && touch.position.y <= size.height;
        let pressed = !touch.is_ended() && inside;
        if pressed != self.pressed {
            weak_from_ref(self).pressed = pressed;
            self.refresh();
        }
    }
}

impl ViewCallbacks for ToolButton {
    /// The icon is an SVG with the color written in, a theme change has to
    /// tint it again.
    fn theme_changed(&mut self) {
        self.refresh();
    }
}

impl Setup for ToolButton {
    fn setup(mut self: Weak<Self>) {
        self.set_corner_radius(RADIUS);
        self.place().h(HEIGHT);

        let label = Label::new();
        label.__base_view().is_system = true;
        label.__base_view().ignore_global_style = true;
        self.label = self.add_subview(label).downcast_view().unwrap();
        self.label.set_color(CLEAR).set_text_size(TEXT_SIZE).set_hidden(true);

        self.icon.set_hidden(true);
        self.icon.__base_view().is_system = true;

        self.enable_touch();
        self.enable_hover();
        self.touch().hovered.sub(self, move || self.refresh());
        self.touch().all.val(move |touch| self.touched(touch));
        self.touch().up_inside.sub(self, move || {
            if self.enabled {
                self.on_tap.trigger(());
            }
        });

        self.relayout();
        self.refresh();
    }
}
