use crate::{
    self as hilen,
    deps::{
        refs::{Weak, weak_from_ref},
        vents::Event,
    },
    gm::color::{CLEAR, Color},
    ui::{Anchor, DynamicColor, Label, Setup, TextAlignment, TextField, ToolButton, ViewData, view},
};

const BORDER: DynamicColor = DynamicColor::new(Color::hex("#D1D1D6"), Color::hex("#48484E"));
const FILL: DynamicColor = DynamicColor::new(Color::hex("#FFFFFF"), Color::hex("#1C1C1E"));
const TEXT: DynamicColor = DynamicColor::new(Color::hex("#1C1C1E"), Color::hex("#F2F2F4"));
const UNIT: DynamicColor = DynamicColor::new(Color::hex("#6E6E73"), Color::hex("#A1A1A6"));
const HEIGHT: f32 = 36.0;
const STEP_WIDTH: f32 = 32.0;

/// A number with its unit and a minus and a plus button, like a price per
/// kWh or a target heat. A typed value is kept when the field loses the
/// focus or on Enter, clamped to the range and rounded to the step. A
/// value that is no number goes back to the last good one. `changed` fires
/// only for the user, a typed value or a press of a step button. 36 points
/// tall.
#[view]
pub struct NumberField {
    value:       f64,
    min:         f64,
    max:         f64,
    step:        f64,
    decimals:    usize,
    /// The value after a change by the user.
    pub changed: Event<f64>,

    #[init]
    minus: ToolButton,
    field: TextField,
    unit:  Label,
    plus:  ToolButton,
}

impl NumberField {
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Shows a value, clamped and rounded, with no `changed`.
    pub fn set_value(&self, value: f64) -> &Self {
        let value = self.fit(value);
        weak_from_ref(self).value = value;
        self.show();
        self
    }

    /// The smallest and the biggest value, none and none by default.
    pub fn set_range(&self, min: f64, max: f64) -> &Self {
        let mut this = weak_from_ref(self);
        this.min = min.min(max);
        this.max = max.max(min);
        self.set_value(self.value)
    }

    /// What a step button adds, and the step a typed value is rounded to.
    /// 1 by default.
    pub fn set_step(&self, step: f64) -> &Self {
        let mut this = weak_from_ref(self);
        this.step = step.abs();
        this.decimals = decimals_of(step);
        self.set_value(self.value)
    }

    /// The text after the number, like `°C` or `EUR/kWh`.
    pub fn set_unit(&self, unit: impl ToString) -> &Self {
        self.unit.set_text(unit.to_string());
        self.unit.place().replace_w(self.unit.content_size().width + 4.0);
        self
    }

    fn fit(&self, value: f64) -> f64 {
        let value = if self.step > 0.0 {
            (value / self.step).round() * self.step
        } else {
            value
        };
        value.clamp(self.min, self.max)
    }

    fn show(&self) {
        let decimals = self.decimals;
        self.field.set_text(format!("{:.decimals$}", self.value));
        self.minus.set_enabled(self.value > self.min);
        self.plus.set_enabled(self.value < self.max);
    }

    fn user_set(mut self: Weak<Self>, value: f64) {
        let value = self.fit(value);
        let changed = (value - self.value).abs() > f64::EPSILON;
        self.value = value;
        self.show();
        if changed {
            self.changed.trigger(value);
        }
    }

    fn typed(self: Weak<Self>, text: &str) {
        match text.trim().replace(',', ".").parse::<f64>() {
            Ok(value) if value.is_finite() => self.user_set(value),
            _ => self.show(),
        }
    }
}

impl Setup for NumberField {
    fn setup(mut self: Weak<Self>) {
        self.min = f64::MIN;
        self.max = f64::MAX;
        self.step = 1.0;
        self.set_color(FILL)
            .set_corner_radius(8)
            .set_border_width(1)
            .set_border_color(BORDER);
        self.place().h(HEIGHT);

        self.minus.set_text("-").set_fills_width(true).set_centered(true);
        self.minus.place().l(0).tb(0).w(STEP_WIDTH);
        self.minus.on_tap(move || self.user_set(self.value - self.step));

        self.plus.set_text("+").set_fills_width(true).set_centered(true);
        self.plus.place().r(0).tb(0).w(STEP_WIDTH);
        self.plus.on_tap(move || self.user_set(self.value + self.step));

        self.unit
            .set_color(CLEAR)
            .set_text_color(UNIT)
            .set_text_size(14)
            .set_text_inset(0.0);
        self.unit.set_alignment(TextAlignment::Left);
        self.unit.place().tb(0).w(0).anchor(Anchor::Right, self.plus, 4);

        self.field.float_only().set_text_size(15).set_text_color(TEXT).set_color(CLEAR);
        self.field.set_alignment(TextAlignment::Right);
        self.field
            .place()
            .tb(0)
            .anchor(Anchor::Left, self.minus, 0)
            .anchor(Anchor::Right, self.unit, 4);
        self.field.editing_ended.val(move |text| self.typed(&text));
        self.field.submitted.val(move |text| self.typed(&text));

        self.show();
    }
}

fn decimals_of(step: f64) -> usize {
    let mut decimals = 0;
    let mut scaled = step.abs();
    while decimals < 6 && (scaled - scaled.round()).abs() > 1e-9 {
        scaled *= 10.0;
        decimals += 1;
    }
    decimals
}

#[cfg(test)]
mod tests {
    use super::decimals_of;

    #[test]
    fn decimals_follow_the_step() {
        assert_eq!(decimals_of(1.0), 0);
        assert_eq!(decimals_of(0.5), 1);
        assert_eq!(decimals_of(0.01), 2);
        assert_eq!(decimals_of(0.25), 2);
    }
}
