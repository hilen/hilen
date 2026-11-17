use crate::{
    self as hilen,
    deps::refs::{Weak, weak_from_ref},
    gm::{
        ToF32,
        color::{CLEAR, Color},
    },
    ui::{DynamicColor, Label, Setup, TextAlignment, ViewData, ViewFrame, ViewSubviews, view},
};

const TEXT: DynamicColor = DynamicColor::new(Color::hex("#1C1C1E"), Color::hex("#F2F2F4"));
const EMPTY: DynamicColor = DynamicColor::new(Color::hex("#F2F2F7"), Color::hex("#2C2C30"));
const LOW: Color = Color::hex("#E8F1FF");
const HIGH: Color = Color::hex("#0A4FBF");
const ROW: f32 = 32.0;
const GAP: f32 = 3.0;
const HEADER: f32 = 22.0;
const TEXT_SIZE: f32 = 12.0;

/// A grid of shaded cells with a text in each, rows of a name and columns
/// of a header, like the hours outside the target per zone and day. A cell
/// is shaded from a light color at the smallest value to a dark one at the
/// biggest, a cell with no value is plain. The height follows the rows, so
/// place it with a width and no height, or put it in a stack.
#[view]
pub struct HeatGrid {
    rows:    Vec<String>,
    columns: Vec<String>,
    values:  Vec<Vec<Option<f64>>>,
    texts:   Vec<Vec<String>>,
    colors:  Option<(Color, Color)>,
    views:   Vec<Weak<Label>>,
}

impl HeatGrid {
    /// Replaces the grid. `values[row][column]` shades a cell and
    /// `format` gives its text.
    pub fn set_cells(
        &self,
        rows: impl IntoIterator<Item = impl ToString>,
        columns: impl IntoIterator<Item = impl ToString>,
        values: Vec<Vec<Option<f64>>>,
        format: impl Fn(f64) -> String,
    ) -> &Self {
        let mut this = weak_from_ref(self);
        this.rows = rows.into_iter().map(|row| row.to_string()).collect();
        this.columns = columns.into_iter().map(|column| column.to_string()).collect();
        this.texts = values
            .iter()
            .map(|row| row.iter().map(|value| value.map_or_else(String::new, &format)).collect())
            .collect();
        this.values = values;
        for mut view in this.views.drain(..) {
            view.remove_from_superview();
        }
        let count = this.columns.len() + this.rows.len() * (this.columns.len() + 1);
        for _ in 0..count {
            let label = this.add_view::<Label>();
            label.set_text_size(TEXT_SIZE).set_text_inset(0.0).set_corner_radius(4);
            this.views.push(label);
        }
        self.relayout();
        self
    }

    /// The colors of the smallest and the biggest value.
    pub fn set_scale(&self, low: Color, high: Color) -> &Self {
        weak_from_ref(self).colors = Some((low, high));
        self.relayout();
        self
    }

    /// The color a cell of `value` gets.
    pub fn color_for(&self, value: f64) -> Color {
        let (low, high) = self.colors.unwrap_or((LOW, HIGH));
        let (min, max) = self.bounds();
        let share = if max > min {
            ((value - min) / (max - min)).clamp(0.0, 1.0)
        } else {
            0.5
        }
        .to_f32();
        Color::rgba(
            low.r + (high.r - low.r) * share,
            low.g + (high.g - low.g) * share,
            low.b + (high.b - low.b) * share,
            1.0,
        )
    }

    fn bounds(&self) -> (f64, f64) {
        self.values
            .iter()
            .flatten()
            .flatten()
            .fold((f64::MAX, f64::MIN), |(low, high), value| {
                (low.min(*value), high.max(*value))
            })
    }

    fn relayout(&self) {
        let columns = self.columns.len();
        if self.views.len() != columns + self.rows.len() * (columns + 1) {
            return;
        }
        let name_width = (self.width() * 0.22).clamp(60.0, 140.0);
        let cell = if columns > 0 {
            (self.width() - name_width - GAP * columns.to_f32()) / columns.to_f32()
        } else {
            0.0
        };
        let (_, high) = self.colors.unwrap_or((LOW, HIGH));
        let mut views = self.views.iter();
        for (index, column) in self.columns.iter().enumerate() {
            let label = views.next().copied().unwrap();
            label.set_text(column).set_color(CLEAR).set_text_color(TEXT);
            label.set_frame((
                name_width + GAP + index.to_f32() * (cell + GAP),
                0.0,
                cell,
                HEADER,
            ));
        }
        for (row_index, row) in self.rows.iter().enumerate() {
            let y = HEADER + GAP + row_index.to_f32() * (ROW + GAP);
            let name = views.next().copied().unwrap();
            name.set_text(row)
                .set_color(CLEAR)
                .set_text_color(TEXT)
                .set_alignment(TextAlignment::Left);
            name.set_frame((0.0, y, name_width, ROW));
            for column in 0..columns {
                let label = views.next().copied().unwrap();
                let value = self.values.get(row_index).and_then(|row| row.get(column)).copied().flatten();
                let text = self
                    .texts
                    .get(row_index)
                    .and_then(|row| row.get(column))
                    .cloned()
                    .unwrap_or_default();
                label.set_text(text);
                if let Some(value) = value {
                    let fill = self.color_for(value);
                    label.set_color(fill);
                    let dark = luma(fill) < luma(high).max(0.45);
                    label.set_text_color(if dark {
                        Color::hex("#FFFFFF")
                    } else {
                        Color::hex("#1C1C1E")
                    });
                } else {
                    label.set_color(EMPTY);
                    label.set_text_color(TEXT);
                }
                label.set_frame((name_width + GAP + column.to_f32() * (cell + GAP), y, cell, ROW));
            }
        }
        let rows = self.rows.len().to_f32();
        self.place().replace_h(HEADER + GAP + rows * (ROW + GAP));
    }
}

fn luma(color: Color) -> f32 {
    0.299 * color.r + 0.587 * color.g + 0.114 * color.b
}

impl Setup for HeatGrid {
    fn setup(self: Weak<Self>) {
        self.set_color(CLEAR);
        self.size_changed().sub(move || self.relayout());
    }
}
