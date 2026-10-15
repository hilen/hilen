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
};

const TEXT: DynamicColor = DynamicColor::new(Color::hex("#1C1C1E"), Color::hex("#F2F2F4"));
const ON_TEXT: DynamicColor = DynamicColor::new(Color::hex("#0A6CFF"), Color::hex("#5AA5FF"));
const DISABLED_TEXT: DynamicColor = DynamicColor::new(Color::hex("#B4B4BC"), Color::hex("#5C5C64"));
const HOVER: DynamicColor = DynamicColor::new(Color::hex("#ECECF0"), Color::hex("#323236"));
const PRESSED: DynamicColor = DynamicColor::new(Color::hex("#D6D6DD"), Color::hex("#46464C"));
const ON: DynamicColor = DynamicColor::new(Color::hex("#E1ECFF"), Color::hex("#1F3352"));

const PADDING: f32 = 10.0;
const GAP: f32 = 6.0;
const ICON_SIZE: f32 = 16.0;
const HEIGHT: f32 = 32.0;
const TEXT_SIZE: f32 = 14.0;
const RADIUS: f32 = 6.0;

/// A button with an icon on the left and a text after it, as wide as
/// both need. The entry of a toolbar or a menu. The icon is an SVG with
/// black in it, tinted like `Tinted`. `set_on` gives the look of a toggle
/// that is on.
#[view]
pub struct ToolButton {
    on_tap: UIEvent,

    icon_name: Option<String>,

    #[educe(Default = true)]
    enabled: bool,
    on:      bool,
    pressed: bool,

    tint:    Option<UIColor>,
    on_tint: Option<UIColor>,

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

    /// The color of the icon and the text. A `DynamicColor` follows the
    /// theme. The default is the text color of the theme.
    pub fn set_tint(&self, tint: impl Into<UIColor>) -> &Self {
        weak_from_ref(self).tint = Some(tint.into());
        self.refresh();
        self
    }

    /// The color of the icon and the text while the button is on.
    pub fn set_on_tint(&self, tint: impl Into<UIColor>) -> &Self {
        weak_from_ref(self).on_tint = Some(tint.into());
        self.refresh();
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
            PRESSED.into()
        } else if self.is_hovered() {
            HOVER.into()
        } else if self.on {
            ON.into()
        } else {
            CLEAR.into()
        }
    }

    fn refresh(&self) {
        let tint = self.tint();
        self.label.set_text_color(tint);
        if let Some(name) = &self.icon_name {
            self.icon.set_image(Tinted {
                tint: tint.resolve(),
                name: name.clone(),
            });
        }
        self.set_color(self.background());
    }

    /// The width follows the icon and the text. The height stays what the
    /// owner placed, 32 points by default.
    fn relayout(&self) {
        let icon = if self.icon_name.is_some() { ICON_SIZE } else { 0.0 };
        let text = if self.label.text().is_empty() {
            0.0
        } else {
            self.label.content_size().width
        };
        let gap = if icon > 0.0 && text > 0.0 { GAP } else { 0.0 };

        self.icon.place().clear().l(PADDING).center_y().size(ICON_SIZE, ICON_SIZE);
        self.label.place().clear().l(PADDING + icon + gap).tb(0).w(text);
        self.place().w(PADDING + icon + gap + text + PADDING);
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
