use crate::{
    self as hilen,
    deps::refs::{Weak, weak_from_ref},
    gm::{
        ToF32,
        color::{CLEAR, Color},
    },
    ui::{
        Container, DynamicColor, Label, Setup, TextAlignment, UIColor, ViewData, ViewFrame, ViewSubviews,
        view,
    },
};

const TEXT: DynamicColor = DynamicColor::new(Color::hex("#1C1C1E"), Color::hex("#F2F2F4"));
const TRACK: DynamicColor = DynamicColor::new(Color::hex("#EFEFF4"), Color::hex("#2C2C30"));
const BAR: DynamicColor = DynamicColor::new(Color::hex("#0A84FF"), Color::hex("#409CFF"));
const ROW: f32 = 28.0;
const GAP: f32 = 8.0;
const TEXT_SIZE: f32 = 14.0;

struct Row {
    name:  Weak<Label>,
    track: Weak<Container>,
    bar:   Weak<Container>,
    value: Weak<Label>,
}

/// Horizontal bars, 1 per row: the name at the left, the bar, and the value
/// as text at the right, like the water use of each zone. The longest bar
/// is the biggest value, or `set_max` when the app fixes the scale. The
/// height follows the rows, 28 points each with 8 between, so place it with
/// a width and no height, or put it in a stack.
#[view]
pub struct BarChart {
    values:     Vec<f64>,
    max:        Option<f64>,
    name_width: f32,
    rows:       Vec<Row>,
    color:      Option<UIColor>,
}

impl BarChart {
    /// Replaces the rows: a name, a value and the text shown for the value.
    pub fn set_bars(&self, bars: impl IntoIterator<Item = (impl ToString, f64, impl ToString)>) -> &Self {
        let mut this = weak_from_ref(self);
        for row in this.rows.drain(..) {
            let (mut name, mut track, mut value) = (row.name, row.track, row.value);
            name.remove_from_superview();
            track.remove_from_superview();
            value.remove_from_superview();
        }
        this.values.clear();
        for (name, value, text) in bars {
            let name_label = this.add_view::<Label>();
            name_label
                .set_text(name.to_string())
                .set_text_size(TEXT_SIZE)
                .set_text_color(TEXT);
            name_label
                .set_alignment(TextAlignment::Left)
                .set_text_inset(0.0)
                .set_color(CLEAR);
            let track = this.add_view::<Container>();
            track.set_color(TRACK).set_corner_radius(4);
            let bar = track.add_view::<Container>();
            bar.set_color(this.color.unwrap_or(BAR.into())).set_corner_radius(4);
            let value_label = this.add_view::<Label>();
            value_label
                .set_text(text.to_string())
                .set_text_size(TEXT_SIZE)
                .set_text_color(TEXT);
            value_label
                .set_alignment(TextAlignment::Right)
                .set_text_inset(0.0)
                .set_color(CLEAR);
            this.values.push(value);
            this.rows.push(Row {
                name: name_label,
                track,
                bar,
                value: value_label,
            });
        }
        self.relayout();
        self
    }

    /// The value of a full bar, none for the biggest value of the rows.
    pub fn set_max(&self, max: Option<f64>) -> &Self {
        weak_from_ref(self).max = max;
        self.relayout();
        self
    }

    pub fn set_bar_color(&self, color: impl Into<UIColor>) -> &Self {
        let color = color.into();
        weak_from_ref(self).color = Some(color);
        for row in &self.rows {
            row.bar.set_color(color);
        }
        self
    }

    /// The width of the name column, 120 points by default.
    pub fn set_name_width(&self, width: f32) -> &Self {
        weak_from_ref(self).name_width = width;
        self.relayout();
        self
    }

    fn relayout(&self) {
        let width = self.width();
        let name_width = if self.name_width > 0.0 {
            self.name_width
        } else {
            120.0
        };
        let value_width = 72.0;
        let track_width = (width - name_width - value_width - GAP * 2.0).max(0.0);
        let max = self
            .max
            .unwrap_or_else(|| self.values.iter().copied().fold(0.0, f64::max))
            .max(f64::MIN_POSITIVE);
        for (index, (row, value)) in self.rows.iter().zip(&self.values).enumerate() {
            let y = index.to_f32() * (ROW + GAP);
            row.name.set_frame((0.0, y, name_width, ROW));
            row.track.set_frame((name_width + GAP, y + 6.0, track_width, ROW - 12.0));
            let share = (value / max).clamp(0.0, 1.0).to_f32();
            row.bar.set_frame((0.0, 0.0, track_width * share, ROW - 12.0));
            row.value.set_frame((width - value_width, y, value_width, ROW));
        }
        let rows = self.rows.len().to_f32();
        let height = if rows > 0.0 {
            rows * ROW + (rows - 1.0) * GAP
        } else {
            0.0
        };
        self.place().replace_h(height);
    }
}

impl Setup for BarChart {
    fn setup(self: Weak<Self>) {
        self.set_color(CLEAR);
        self.size_changed().sub(move || self.relayout());
    }
}
