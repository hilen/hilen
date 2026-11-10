use ui_proc::view;

use crate::{
    deps::{
        refs::{Weak, weak_from_ref},
        vents::Event,
    },
    gm::color::{Color, LIGHT_GRAY},
    image::tint_svg,
    ui::{
        Container, DynamicColor, ImageView, Setup, UIColor, ViewCallbacks, ViewFrame,
        view::{ViewData, ViewTouch},
    },
};

const CHECK: &[u8] = include_bytes!("../../images/check.svg");

const FILL: Color = Color::hex("#273549");
const BORDER: Color = Color::hex("#464E61");
const DOT: Color = Color::hex("#5894F2");

const DISABLED_FILL: DynamicColor = DynamicColor::new(Color::hex("#F2F2F4"), Color::hex("#2A2A2E"));
const DISABLED_BORDER: DynamicColor = DynamicColor::new(Color::hex("#D1D1D6"), Color::hex("#48484E"));
const DISABLED_MARK: DynamicColor = DynamicColor::new(Color::hex("#B4B4BC"), Color::hex("#5C5C64"));

/// What a `CheckBox` draws while it is on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CheckBoxMark {
    /// A small round dot in the middle.
    #[default]
    Dot,
    /// The Lucide check, <https://lucide.dev>, in the mark color.
    Check,
}

#[view]
pub struct CheckBox {
    on:              bool,
    disabled:        bool,
    mark:            CheckBoxMark,
    colors:          Option<(UIColor, UIColor, UIColor)>,
    disabled_colors: Option<(UIColor, UIColor, UIColor)>,

    pub selected: Event<bool>,

    #[init]
    dot:   Container,
    check: ImageView,
}

impl CheckBox {
    pub fn on(&self) -> bool {
        self.on
    }

    pub fn on_change<Ret>(
        self: Weak<Self>,
        mut callback: impl FnMut(bool) -> Ret + Send + 'static,
    ) -> Weak<Self> {
        self.selected.val(move |val| {
            callback(val);
        });
        self
    }

    /// The box fill, its border and the mark drawn while it is on.
    pub fn set_colors(
        &self,
        fill: impl Into<UIColor>,
        border: impl Into<UIColor>,
        mark: impl Into<UIColor>,
    ) -> &Self {
        weak_from_ref(self).colors = Some((fill.into(), border.into(), mark.into()));
        self.refresh();
        self
    }

    /// The fill, the border and the mark of a disabled box. Without them a
    /// gray set that fits both themes is used.
    pub fn set_disabled_colors(
        &self,
        fill: impl Into<UIColor>,
        border: impl Into<UIColor>,
        mark: impl Into<UIColor>,
    ) -> &Self {
        weak_from_ref(self).disabled_colors = Some((fill.into(), border.into(), mark.into()));
        self.refresh();
        self
    }

    pub fn set_mark(&self, mark: CheckBoxMark) -> &Self {
        weak_from_ref(self).mark = mark;
        self.refresh();
        self
    }

    pub fn set_on(&mut self, on: bool) {
        self.on = on;
        self.refresh();
    }

    pub fn is_enabled(&self) -> bool {
        !self.disabled
    }

    /// A disabled box draws in faint colors and a tap does not change it. It
    /// still takes touches, so what is behind it gets none.
    pub fn set_enabled(&self, enabled: bool) -> &Self {
        weak_from_ref(self).disabled = !enabled;
        self.refresh();
        self
    }
}

impl CheckBox {
    fn refresh(&self) {
        let (fill, border, mark) = if self.disabled {
            self.disabled_colors.unwrap_or((
                DISABLED_FILL.into(),
                DISABLED_BORDER.into(),
                DISABLED_MARK.into(),
            ))
        } else {
            self.colors.unwrap_or((FILL.into(), BORDER.into(), DOT.into()))
        };
        self.set_color(fill).set_border_color(border);
        self.dot.set_color(mark);
        self.dot.set_hidden(!self.on || self.mark != CheckBoxMark::Dot);
        self.check.set_hidden(!self.on || self.mark != CheckBoxMark::Check);
        if self.mark == CheckBoxMark::Check {
            self.check.set_image(tint_svg(CHECK, "hilen_check.svg", mark.resolve()));
        }
    }
}

impl ViewCallbacks for CheckBox {
    /// The check is an SVG with the color written in, a theme change has to
    /// tint it again.
    fn theme_changed(&mut self) {
        self.refresh();
    }
}

impl Setup for CheckBox {
    fn setup(mut self: Weak<Self>) {
        self.enable_touch();

        self.set_border_width(2).set_corner_radius(10);

        self.dot.set_corner_radius(4).set_border_width(1.3).set_border_color(LIGHT_GRAY);
        self.dot
            .place()
            .custom(move |f| {
                let side = self.size().smallest_side() * 0.42;
                f.size = (side, side).into();
            })
            .center();

        self.check
            .place()
            .custom(move |f| {
                let side = self.size().smallest_side() * 0.6;
                f.size = (side, side).into();
            })
            .center();

        self.set_on(false);

        self.touch().up_inside.sub(self, move || {
            if self.disabled {
                return;
            }
            let on = !self.on;
            self.set_on(on);
            self.selected.trigger(on);
        });
    }
}
