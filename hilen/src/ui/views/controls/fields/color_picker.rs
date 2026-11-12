use crate::{
    self as hilen,
    deps::{
        refs::{Weak, weak_from_ref},
        vents::Event,
    },
    gm::{
        ToF32,
        color::{CLEAR, Color, U8Color},
    },
    ui::{
        Container, DynamicColor, Label, Setup, Slider, TextAlignment, TextField, ViewData, ViewFrame,
        ViewSubviews, view, view::ViewTouch,
    },
};

const BORDER: DynamicColor = DynamicColor::new(Color::hex("#D1D1D6"), Color::hex("#48484E"));
const TEXT: DynamicColor = DynamicColor::new(Color::hex("#1C1C1E"), Color::hex("#F2F2F4"));
const FILL: DynamicColor = DynamicColor::new(Color::hex("#FFFFFF"), Color::hex("#1C1C1E"));
const ROW: f32 = 32.0;
const SWATCH: f32 = 28.0;
/// The color picker is this tall with a row of presets.
const HEIGHT: f32 = 4.0 * ROW + 8.0 + SWATCH;

/// Picks a color: a sample, a red, a green and a blue slider, a hex field
/// and a row of preset colors, like the color of a light. `changed` fires
/// only for the user: a drag, a typed hex on Enter or when the field loses
/// the focus, or a tap on a preset. 168 points tall.
#[view]
pub struct ColorPicker {
    color:       Color,
    presets:     Vec<Weak<Container>>,
    /// The color after a change by the user.
    pub changed: Event<Color>,

    #[init]
    sample: Container,
    red:    Slider,
    green:  Slider,
    blue:   Slider,
    names:  Container,
    hex:    TextField,
}

impl ColorPicker {
    pub fn color(&self) -> Color {
        self.color
    }

    /// Shows a color with no `changed`.
    pub fn set_color_value(&self, color: Color) -> &Self {
        weak_from_ref(self).color = color.with_alpha(1.0);
        self.show();
        self
    }

    /// The preset colors under the sliders, a tap on one picks it.
    pub fn set_presets(&self, colors: impl IntoIterator<Item = Color>) -> &Self {
        let mut this = weak_from_ref(self);
        for mut preset in this.presets.drain(..) {
            preset.remove_from_superview();
        }
        for (index, color) in colors.into_iter().enumerate() {
            let preset = this.add_view::<Container>();
            preset
                .set_color(color)
                .set_corner_radius(SWATCH / 2.0)
                .set_border_width(1)
                .set_border_color(BORDER);
            let x = index.to_f32() * (SWATCH + 8.0);
            preset.place().size(SWATCH, SWATCH).l(x).t(4.0 * ROW + 8.0);
            preset.enable_touch();
            preset.touch().up_inside.sub(this, move || this.user_set(color));
            this.presets.push(preset);
        }
        self
    }

    fn show(&self) {
        let color = U8Color::from(self.color);
        self.sample.set_color(self.color);
        self.red.set_value(f32::from(color.r));
        self.green.set_value(f32::from(color.g));
        self.blue.set_value(f32::from(color.b));
        self.hex.set_text(format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b));
    }

    fn user_set(mut self: Weak<Self>, color: Color) {
        let color = color.with_alpha(1.0);
        let changed = U8Color::from(color) != U8Color::from(self.color);
        self.color = color;
        self.show();
        if changed {
            self.changed.trigger(color);
        }
    }

    fn slid(self: Weak<Self>) {
        let channel = |slider: Weak<Slider>| -> f32 { slider.value().round() / 255.0 };
        self.user_set(Color::rgb(
            channel(self.red),
            channel(self.green),
            channel(self.blue),
        ));
    }

    fn typed(self: Weak<Self>, text: &str) {
        match parse_hex(text) {
            Some(color) => self.user_set(color),
            None => self.show(),
        }
    }
}

impl Setup for ColorPicker {
    fn setup(self: Weak<Self>) {
        self.set_color(CLEAR);
        self.place().h(HEIGHT);

        self.sample.set_corner_radius(8).set_border_width(1).set_border_color(BORDER);
        self.sample.place().t(0).l(0).size(ROW * 3.0 - 8.0, ROW * 3.0 - 8.0);

        self.names.set_color(CLEAR);
        self.names.place().t(0).l(ROW * 3.0).w(20).h(ROW * 3.0);
        for (index, name) in ["R", "G", "B"].into_iter().enumerate() {
            let label = self.names.add_view::<Label>();
            label
                .set_text(name)
                .set_text_size(13)
                .set_text_color(TEXT)
                .set_color(CLEAR)
                .set_text_inset(0.0);
            label.set_frame((0.0, index.to_f32() * ROW, 20.0, ROW));
        }

        for (index, slider) in [self.red, self.green, self.blue].into_iter().enumerate() {
            slider.set_horizontal().set_range(0, 255);
            slider.place().l(ROW * 3.0 + 24.0).r(0).h(ROW).t(index.to_f32() * ROW);
            slider.on_change.sub(move || self.slid());
        }

        self.hex
            .set_text_size(14)
            .set_text_color(TEXT)
            .set_color(FILL)
            .set_corner_radius(8);
        self.hex
            .set_border_width(1)
            .set_border_color(BORDER)
            .set_alignment(TextAlignment::Center);
        self.hex.place().t(3.0 * ROW + 2.0).l(0).w(ROW * 3.0 - 8.0).h(ROW);
        self.hex.editing_ended.val(move |text| self.typed(&text));
        self.hex.submitted.val(move |text| self.typed(&text));

        self.set_color_value(Color::hex("#FFFFFF"));
    }
}

/// `#22C55E`, `22c55e` and `#2c5` are the same green.
pub(crate) fn parse_hex(text: &str) -> Option<Color> {
    let digits = text.trim().trim_start_matches('#');
    if !digits.chars().all(|char| char.is_ascii_hexdigit()) {
        return None;
    }
    let full: String = match digits.len() {
        3 => digits.chars().flat_map(|char| [char, char]).collect(),
        6 => digits.to_string(),
        _ => return None,
    };
    let value = u32::from_str_radix(&full, 16).ok()?;
    let channel = |shift: u32| ((value >> shift) & 0xFF).to_f32() / 255.0;
    Some(Color::rgb(channel(16), channel(8), channel(0)))
}

#[cfg(test)]
mod tests {
    use super::parse_hex;
    use crate::gm::color::Color;

    #[test]
    fn hex_in_3_spellings() {
        let green = Some(Color::hex("#22C55E"));
        assert_eq!(parse_hex("#22C55E"), green);
        assert_eq!(parse_hex(" 22c55e "), green);
        assert_eq!(parse_hex("#2c5"), Some(Color::hex("#22CC55")));
        assert_eq!(parse_hex("#22C55"), None);
        assert_eq!(parse_hex("zzzzzz"), None);
    }
}
