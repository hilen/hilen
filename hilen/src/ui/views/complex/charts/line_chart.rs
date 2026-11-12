use std::sync::Arc;

use crate::{
    self as hilen,
    deps::refs::{Weak, weak_from_ref},
    gm::{
        LossyConvert, ToF32,
        color::{CLEAR, Color},
        flat::{Point, StrokeStyle, VectorPath},
    },
    ui::{
        DrawingView, DynamicColor, FillRule, Label, Setup, TextAlignment, UIColor, UIEvents, ViewData,
        ViewFrame, ViewSubviews, view,
    },
};

const GRID: DynamicColor = DynamicColor::new(Color::hex("#E5E5EA"), Color::hex("#3A3A3C"));
const AXIS_TEXT: DynamicColor = DynamicColor::new(Color::hex("#6E6E73"), Color::hex("#A1A1A6"));
const LEFT: f32 = 48.0;
const BOTTOM: f32 = 24.0;
const TEXT_SIZE: f32 = 12.0;

type Format = Arc<dyn Fn(f64) -> String + Send + Sync>;

struct Series {
    points: Vec<(f64, f64)>,
    color:  UIColor,
    width:  f32,
}

/// A line chart of values over x, like readings over time: lines, bands of
/// y between 2 values, like a target range, and lines at an x, like the time
/// of an alert. A line of many more points than the chart is wide is drawn
/// from the mean of each pixel column, so 90 days of minutes draw at once.
/// The axes get 5 steps of round numbers, `set_x_format` and `set_y_format`
/// turn a value into its text, like a time. The ranges follow the data
/// unless `set_x_range` or `set_y_range` fix them.
#[view]
pub struct LineChart {
    series:   Vec<Series>,
    bands:    Vec<(f64, f64, UIColor)>,
    markers:  Vec<(f64, UIColor)>,
    x_range:  Option<(f64, f64)>,
    y_range:  Option<(f64, f64)>,
    x_format: Option<Format>,
    y_format: Option<Format>,
    labels:   Vec<Weak<Label>>,

    #[init]
    plot: DrawingView,
}

impl LineChart {
    /// Adds a line through `points`, sorted by x, 2 points wide.
    pub fn add_line(&self, points: impl IntoIterator<Item = (f64, f64)>, color: impl Into<UIColor>) -> &Self {
        weak_from_ref(self).series.push(Series {
            points: points.into_iter().collect(),
            color:  color.into(),
            width:  2.0,
        });
        self.redraw();
        self
    }

    /// Fills the y range from `low` to `high` across the whole chart, under
    /// the lines, like the target range of a reading.
    pub fn add_band(&self, low: f64, high: f64, color: impl Into<UIColor>) -> &Self {
        weak_from_ref(self).bands.push((low.min(high), low.max(high), color.into()));
        self.redraw();
        self
    }

    /// A vertical line at `x` over the whole height, like the time of an
    /// event.
    pub fn add_marker(&self, x: f64, color: impl Into<UIColor>) -> &Self {
        weak_from_ref(self).markers.push((x, color.into()));
        self.redraw();
        self
    }

    /// Removes every line, band and marker. The ranges and formats stay.
    pub fn clear(&self) -> &Self {
        let mut this = weak_from_ref(self);
        this.series.clear();
        this.bands.clear();
        this.markers.clear();
        self.redraw();
        self
    }

    pub fn set_x_range(&self, range: Option<(f64, f64)>) -> &Self {
        weak_from_ref(self).x_range = range;
        self.redraw();
        self
    }

    pub fn set_y_range(&self, range: Option<(f64, f64)>) -> &Self {
        weak_from_ref(self).y_range = range;
        self.redraw();
        self
    }

    pub fn set_x_format(&self, format: impl Fn(f64) -> String + Send + Sync + 'static) -> &Self {
        weak_from_ref(self).x_format = Some(Arc::new(format));
        self.redraw();
        self
    }

    pub fn set_y_format(&self, format: impl Fn(f64) -> String + Send + Sync + 'static) -> &Self {
        weak_from_ref(self).y_format = Some(Arc::new(format));
        self.redraw();
        self
    }

    /// The x and y ranges the chart draws now.
    pub fn ranges(&self) -> ((f64, f64), (f64, f64)) {
        (self.x_bounds(), self.y_bounds())
    }

    fn x_bounds(&self) -> (f64, f64) {
        if let Some(range) = self.x_range {
            return range;
        }
        let xs = self.series.iter().flat_map(|series| series.points.iter().map(|point| point.0));
        widen(min_max(xs).unwrap_or((0.0, 1.0)), 0.0)
    }

    fn y_bounds(&self) -> (f64, f64) {
        if let Some(range) = self.y_range {
            return range;
        }
        let ys = self
            .series
            .iter()
            .flat_map(|series| series.points.iter().map(|point| point.1))
            .chain(self.bands.iter().flat_map(|band| [band.0, band.1]));
        widen(min_max(ys).unwrap_or((0.0, 1.0)), 0.06)
    }

    fn redraw(&self) {
        let this = weak_from_ref(self);
        let size = this.plot.size();
        this.plot.remove_all_paths();
        if size.width <= 0.0 || size.height <= 0.0 {
            return;
        }
        let (x_range, y_range) = this.ranges();
        let w = f64::from(size.width);
        let h = f64::from(size.height);
        let to_x = move |x: f64| ((x - x_range.0) / (x_range.1 - x_range.0) * w).to_f32();
        let to_y = move |y: f64| (h - (y - y_range.0) / (y_range.1 - y_range.0) * h).to_f32();

        for (low, high, color) in &this.bands {
            let (top, bottom) = (to_y(*high).max(0.0), to_y(*low).min(size.height));
            if bottom > top {
                let rect = [(0.0, top), (size.width, top), (size.width, bottom), (0.0, bottom)];
                this.plot
                    .add_fill(&VectorPath::polygon(rect), color.resolve(), FillRule::NonZero);
            }
        }

        let y_ticks = ticks(y_range);
        let x_ticks = ticks(x_range);
        let grid = StrokeStyle {
            width: 1.0,
            ..StrokeStyle::default()
        };
        for y in &y_ticks {
            let py = to_y(*y).round() + 0.5;
            this.plot.add_stroke(
                &VectorPath::polyline([(0.0, py), (size.width, py)]),
                GRID.resolve(),
                grid,
            );
        }

        for series in &this.series {
            let points = column_means(&series.points, x_range, size.width.lossy_convert());
            let path: Vec<Point> = points.iter().map(|(x, y)| Point::new(to_x(*x), to_y(*y))).collect();
            if path.len() > 1 {
                let style = StrokeStyle {
                    width: series.width,
                    ..StrokeStyle::default()
                };
                this.plot.add_stroke(&VectorPath::polyline(path), series.color.resolve(), style);
            }
        }

        for (x, color) in &this.markers {
            let px = to_x(*x).round() + 0.5;
            if (0.0..=size.width).contains(&px) {
                let style = StrokeStyle {
                    width: 2.0,
                    ..StrokeStyle::default()
                };
                this.plot.add_stroke(
                    &VectorPath::polyline([(px, 0.0), (px, size.height)]),
                    color.resolve(),
                    style,
                );
            }
        }

        this.show_labels(&x_ticks, &y_ticks, &to_x, &to_y);
    }

    fn show_labels(
        &self,
        x_ticks: &[f64],
        y_ticks: &[f64],
        to_x: &dyn Fn(f64) -> f32,
        to_y: &dyn Fn(f64) -> f32,
    ) {
        let mut this = weak_from_ref(self);
        let needed = x_ticks.len() + y_ticks.len();
        while this.labels.len() < needed {
            let label = this.add_view::<Label>();
            label.set_text_size(TEXT_SIZE).set_color(CLEAR).set_text_inset(0.0);
            this.labels.push(label);
        }
        for (index, label) in this.labels.iter().enumerate() {
            label.set_hidden(index >= needed);
        }
        let plot = *this.plot.frame();
        for (index, y) in y_ticks.iter().enumerate() {
            let label = this.labels[index];
            label.set_text(format_value(this.y_format.as_ref(), *y, y_ticks));
            label.set_alignment(TextAlignment::Right).set_text_color(AXIS_TEXT);
            label.set_frame((0.0, plot.y() + to_y(*y) - 8.0, LEFT - 6.0, 16.0));
        }
        for (index, x) in x_ticks.iter().enumerate() {
            let label = this.labels[y_ticks.len() + index];
            label.set_text(format_value(this.x_format.as_ref(), *x, x_ticks));
            label.set_alignment(TextAlignment::Center).set_text_color(AXIS_TEXT);
            label.set_frame((plot.x() + to_x(*x) - 40.0, plot.max_y() + 4.0, 80.0, 16.0));
        }
    }
}

impl Setup for LineChart {
    fn setup(self: Weak<Self>) {
        self.set_color(CLEAR);
        self.plot.set_color(CLEAR);
        self.plot.place().t(8).l(LEFT).r(12).b(BOTTOM);
        self.plot.size_changed().sub(move || self.redraw());
        UIEvents::theme_changed().sub(self, move || self.redraw());
    }
}

fn min_max(values: impl Iterator<Item = f64>) -> Option<(f64, f64)> {
    values.filter(|value| value.is_finite()).fold(None, |range, value| match range {
        None => Some((value, value)),
        Some((low, high)) => Some((low.min(value), high.max(value))),
    })
}

/// The range with a share of its span added on each side, and never empty.
fn widen((low, high): (f64, f64), share: f64) -> (f64, f64) {
    let span = high - low;
    if span <= f64::EPSILON {
        return (low - 1.0, high + 1.0);
    }
    (low - span * share, high + span * share)
}

/// About 5 round values inside the range, steps of 1, 2 or 5 times a power
/// of 10.
pub(crate) fn ticks((low, high): (f64, f64)) -> Vec<f64> {
    let span = high - low;
    if !span.is_finite() || span <= 0.0 {
        return vec![];
    }
    let raw = span / 5.0;
    let power = 10f64.powf(raw.log10().floor());
    let step = [1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .map(|m| m * power)
        .find(|step| *step >= raw)
        .unwrap_or(raw);
    let mut value = (low / step).ceil() * step;
    let mut out = vec![];
    while value <= high + step * 1e-9 && out.len() < 20 {
        out.push(if value.abs() < step * 1e-9 { 0.0 } else { value });
        value += step;
    }
    out
}

fn format_value(format: Option<&Format>, value: f64, all: &[f64]) -> String {
    if let Some(format) = format {
        return format(value);
    }
    let step = all.windows(2).map(|pair| pair[1] - pair[0]).next().unwrap_or(1.0);
    let decimals = if step >= 1.0 {
        0
    } else {
        (-step.log10()).ceil().lossy_convert()
    };
    format!("{value:.decimals$}")
}

/// The points to draw: all of them when they fit, else the mean of the
/// points in each pixel column, so the line keeps its shape.
fn column_means(points: &[(f64, f64)], (low, high): (f64, f64), columns: usize) -> Vec<(f64, f64)> {
    let inside: Vec<(f64, f64)> = points
        .iter()
        .copied()
        .filter(|(x, y)| x.is_finite() && y.is_finite() && *x >= low && *x <= high)
        .collect();
    if inside.len() <= columns.max(1) * 2 {
        return inside;
    }
    let width = (high - low) / f64::from(columns.to_f32());
    let mut sums = vec![(0.0, 0.0, 0usize); columns];
    for (x, y) in inside {
        let column: usize = ((x - low) / width).lossy_convert();
        let column = column.min(columns - 1);
        let slot = &mut sums[column];
        slot.0 += x;
        slot.1 += y;
        slot.2 += 1;
    }
    sums.into_iter()
        .filter(|slot| slot.2 > 0)
        .map(|(x, y, count)| {
            let count = f64::from(count.to_f32());
            (x / count, y / count)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{column_means, ticks};

    #[test]
    fn ticks_are_round_steps_inside_the_range() {
        assert_eq!(ticks((0.0, 10.0)), vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0]);
        assert_eq!(ticks((17.3, 26.1)), vec![18.0, 20.0, 22.0, 24.0, 26.0]);
        assert_eq!(ticks((0.0, 0.0)), Vec::<f64>::new());
    }

    #[test]
    fn many_points_become_1_mean_per_column() {
        let points: Vec<(f64, f64)> = (0..10_000).map(|i| (f64::from(i), f64::from(i % 2))).collect();
        let means = column_means(&points, (0.0, 10_000.0), 100);
        assert_eq!(means.len(), 100);
        assert!(means.iter().all(|(_, y)| (*y - 0.5).abs() < 0.01));
        let few = column_means(&points[..50], (0.0, 10_000.0), 100);
        assert_eq!(few.len(), 50);
    }
}
