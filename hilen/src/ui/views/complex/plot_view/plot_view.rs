use std::sync::atomic::{AtomicU64, Ordering};

use super::grid::{label, lines, nice_step};
use crate::{
    self as hilen,
    deps::{
        refs::{Weak, manage::DataManager, weak_from_ref},
        vents::Event,
    },
    gm::{
        LossyConvert, ToF32,
        color::{Color, U8Color},
        flat::{LineCap, LineJoin, Point, Size, StrokeStyle, VectorPath},
    },
    ui::{
        DrawingView, DynamicColor, Image, ImageView, Label, Pinch, Scrollable, Setup, TextAlignment, Touch,
        TouchStack, UIColor, UIEvents, UIManager, View, ViewData, ViewFrame, ViewSubviews, ViewTouch, view,
    },
};

/// One wheel notch is 60 pixels, that zooms by about an eighth, the same as
/// `CanvasView`.
const WHEEL_ZOOM: f64 = 0.002;
/// About how many points lie between 2 grid lines.
const GRID_SPACING: f64 = 90.0;
/// A function is sampled every this many points along x.
const SAMPLE_STEP: f64 = 2.0;
/// A sample this many view heights above or below the view ends a piece
/// of the curve, so `1/x` at 0 draws no line from the top to the bottom.
const BREAK_HEIGHTS: f64 = 4.0;
const MIN_SPAN: f64 = 1e-9;
const MAX_SPAN: f64 = 1e9;
const LABEL_WIDTH: f32 = 64.0;
const LABEL_HEIGHT: f32 = 18.0;
const LABEL_GAP: f32 = 4.0;

/// The color field is sampled every this many points and stretched.
const FIELD_STEP: f32 = 2.0;

/// Each picture of the color field needs a name of its own, the store
/// returns the old picture for a name it already holds.
static FIELD_PICTURES: AtomicU64 = AtomicU64::new(0);

const GRID: DynamicColor = DynamicColor::new(Color::hex("#E3E6EC"), Color::hex("#2A303C"));
const AXIS: DynamicColor = DynamicColor::new(Color::hex("#5B6472"), Color::hex("#9AA3B2"));

enum Source {
    Function(Box<dyn Fn(f64) -> f64>),
    Points(Vec<(f64, f64)>),
}

struct Curve {
    source: Source,
    color:  UIColor,
}

/// A 2D graph of functions `y = f(x)` and of point lists, with a grid,
/// axes and number labels, like a graphing calculator. A drag pans, the
/// wheel zooms at the cursor and a pinch zooms at the fingers. Every
/// change samples the functions again for the range on screen, so a
/// curve stays sharp at any zoom.
#[view]
pub struct PlotView {
    center_x:      f64,
    center_y:      f64,
    /// How wide the visible range of x is. The range of y follows from
    /// the shape of the view, a unit is as long on both axes.
    x_span:        f64,
    curves:        Vec<Curve>,
    line_width:    f32,
    grid_color:    UIColor,
    axis_color:    UIColor,
    labels:        Vec<Weak<Label>>,
    drag_from:     Option<Point>,
    color_field:   Option<Box<dyn Fn(f64, f64) -> Color>>,
    field_picture: Weak<Image>,

    /// The visible range changed by a pan, a zoom or a call.
    pub on_change: Event,

    #[init]
    field_view: ImageView,
    drawing:    DrawingView,
}

impl PlotView {
    /// Adds a curve of `y = function(x)`. A value that is not finite,
    /// like `ln(x)` below 0, leaves a gap.
    pub fn add_function(&self, function: impl Fn(f64) -> f64 + 'static, color: impl Into<UIColor>) -> &Self {
        let mut this = weak_from_ref(self);
        this.curves.push(Curve {
            source: Source::Function(Box::new(function)),
            color:  color.into(),
        });
        this.redraw();
        self
    }

    /// Adds a line through the points in their order.
    pub fn add_points(
        &self,
        points: impl IntoIterator<Item = (f64, f64)>,
        color: impl Into<UIColor>,
    ) -> &Self {
        let mut this = weak_from_ref(self);
        this.curves.push(Curve {
            source: Source::Points(points.into_iter().collect()),
            color:  color.into(),
        });
        this.redraw();
        self
    }

    /// Removes every curve, the grid and the axes stay.
    pub fn remove_curves(&self) -> &Self {
        let mut this = weak_from_ref(self);
        this.curves.clear();
        this.redraw();
        self
    }

    /// Fills the plot under the grid with the color of `field(x, y)` at
    /// each point, like a heat map or the domain coloring of a complex
    /// function. It is sampled every 2 points again on each pan and zoom.
    pub fn set_color_field(&self, field: impl Fn(f64, f64) -> Color + 'static) -> &Self {
        let mut this = weak_from_ref(self);
        this.color_field = Some(Box::new(field));
        this.redraw();
        self
    }

    pub fn remove_color_field(&self) -> &Self {
        let mut this = weak_from_ref(self);
        this.color_field = None;
        this.redraw();
        self
    }

    /// Shows x from `min` to `max` around the middle `y`.
    pub fn set_x_range(&self, min: f64, max: f64, y: f64) -> &Self {
        let mut this = weak_from_ref(self);
        this.center_x = f64::midpoint(min, max);
        this.center_y = y;
        this.x_span = (max - min).clamp(MIN_SPAN, MAX_SPAN);
        this.changed();
        self
    }

    pub fn set_line_width(&self, width: impl ToF32) -> &Self {
        let mut this = weak_from_ref(self);
        this.line_width = width.to_f32();
        this.redraw();
        self
    }

    pub fn set_grid_color(&self, color: impl Into<UIColor>) -> &Self {
        let mut this = weak_from_ref(self);
        this.grid_color = color.into();
        this.redraw();
        self
    }

    pub fn set_axis_color(&self, color: impl Into<UIColor>) -> &Self {
        let mut this = weak_from_ref(self);
        this.axis_color = color.into();
        this.redraw();
        self
    }

    /// On by default: a drag pans, the wheel and a pinch zoom. Off, the
    /// plot takes no touch, pinch or wheel, they go on to the views around
    /// it. For a plot in a list or a chat, where the wheel scrolls the list
    /// and a tap belongs to the cell.
    pub fn set_interactive(self: Weak<Self>, on: bool) -> Weak<Self> {
        if on {
            self.enable_input();
        } else {
            TouchStack::disable_input(self.weak_view());
        }
        self
    }

    fn enable_input(self: Weak<Self>) {
        // Low priority, a subview that takes touches is asked first.
        self.enable_touch_low_priority();
        self.enable_pinch();
        TouchStack::enable_scroll(self);
    }

    /// The visible x from the left edge to the right edge.
    pub fn x_range(&self) -> (f64, f64) {
        let half = self.x_span / 2.0;
        (self.center_x - half, self.center_x + half)
    }

    /// The visible y from the bottom edge to the top edge.
    pub fn y_range(&self) -> (f64, f64) {
        let half = self.units_per_point() * f64::from(self.height()) / 2.0;
        (self.center_y - half, self.center_y + half)
    }

    /// The graph point drawn at `point` of this view.
    pub fn graph_point(&self, point: impl Into<Point>) -> (f64, f64) {
        let point = point.into();
        let units = self.units_per_point();
        (
            self.center_x + (f64::from(point.x) - f64::from(self.width()) / 2.0) * units,
            self.center_y - (f64::from(point.y) - f64::from(self.height()) / 2.0) * units,
        )
    }

    /// Zooms by `factor`, more than 1 zooms in, and keeps the graph point
    /// under `point` where it is. `point` is in the points of this view.
    pub fn zoom_at(&self, point: impl Into<Point>, factor: f64) -> &Self {
        let mut this = weak_from_ref(self);
        let point = point.into();
        let (x, y) = this.graph_point(point);
        this.x_span = (this.x_span / factor).clamp(MIN_SPAN, MAX_SPAN);
        let (moved_x, moved_y) = this.graph_point(point);
        this.center_x += x - moved_x;
        this.center_y += y - moved_y;
        this.changed();
        self
    }

    fn units_per_point(&self) -> f64 {
        let width = f64::from(self.width());
        if width > 0.0 { self.x_span / width } else { 0.0 }
    }

    fn view_point(&self, x: f64, y: f64) -> Point {
        let units = self.units_per_point();
        let (left, _) = self.x_range();
        let (_, top) = self.y_range();
        Point::new(((x - left) / units).to_f32(), ((top - y) / units).to_f32())
    }

    fn changed(&mut self) {
        self.redraw();
        self.on_change.trigger(());
    }

    fn redraw(&mut self) {
        self.drawing.remove_all_paths();
        for mut label in self.labels.drain(..) {
            label.remove_from_superview();
        }
        if self.width() <= 0.0 || self.height() <= 0.0 {
            return;
        }

        self.draw_color_field();
        let step = nice_step(GRID_SPACING * self.units_per_point());
        self.draw_grid(step);
        self.draw_axes();
        self.draw_labels(step);
        for index in 0..self.curves.len() {
            self.draw_curve(index);
        }
    }

    fn draw_color_field(&mut self) {
        if self.field_picture.is_ok() {
            self.field_picture.free();
            self.field_picture = Weak::default();
        }
        let Some(field) = &self.color_field else {
            self.field_view.set_hidden(true);
            return;
        };

        let columns: u32 = (self.width() / FIELD_STEP).ceil().lossy_convert();
        let rows: u32 = (self.height() / FIELD_STEP).ceil().lossy_convert();
        let step = f64::from(FIELD_STEP) * self.units_per_point();
        let (left, _) = self.x_range();
        let (_, top) = self.y_range();
        let mut rgba = Vec::new();
        for row in 0..rows {
            let y = top - (f64::from(row) + 0.5) * step;
            for column in 0..columns {
                let x = left + (f64::from(column) + 0.5) * step;
                let color: U8Color = field(x, y).into();
                rgba.extend([color.r, color.g, color.b, color.a]);
            }
        }

        let name = format!("plot-field-{}", FIELD_PICTURES.fetch_add(1, Ordering::Relaxed));
        self.field_picture = Image::from_raw_data(rgba, name, Size::new(columns, rows), 4);
        self.field_view.set_image(self.field_picture);
        self.field_view.set_hidden(false);
    }

    fn draw_grid(&mut self, step: f64) {
        let (left, right) = self.x_range();
        let (bottom, top) = self.y_range();
        let (width, height) = (self.width(), self.height());
        let color = self.grid_color.resolve();
        let style = StrokeStyle::width(1);

        for x in lines(left, right, step) {
            let at = self.view_point(x, 0.0).x;
            self.drawing
                .add_stroke(&VectorPath::polyline([(at, 0.0), (at, height)]), color, style);
        }
        for y in lines(bottom, top, step) {
            let at = self.view_point(0.0, y).y;
            self.drawing
                .add_stroke(&VectorPath::polyline([(0.0, at), (width, at)]), color, style);
        }
    }

    fn draw_axes(&mut self) {
        let origin = self.view_point(0.0, 0.0);
        let (width, height) = (self.width(), self.height());
        let color = self.axis_color.resolve();
        let style = StrokeStyle::width(2);

        if (0.0..=height).contains(&origin.y) {
            self.drawing.add_stroke(
                &VectorPath::polyline([(0.0, origin.y), (width, origin.y)]),
                color,
                style,
            );
        }
        if (0.0..=width).contains(&origin.x) {
            self.drawing.add_stroke(
                &VectorPath::polyline([(origin.x, 0.0), (origin.x, height)]),
                color,
                style,
            );
        }
    }

    /// Numbers under the x axis and right of the y axis. When an axis is
    /// off screen its numbers stay on the nearest edge. A number that would
    /// not fit whole inside the view is left out.
    fn draw_labels(&mut self, step: f64) {
        let (left, right) = self.x_range();
        let (bottom, top) = self.y_range();
        let origin = self.view_point(0.0, 0.0);
        let (width, height) = (self.width(), self.height());
        let color = self.axis_color;

        let below_axis = (origin.y + LABEL_GAP).clamp(0.0, height - LABEL_HEIGHT);
        for x in lines(left, right, step) {
            if x == 0.0 {
                continue;
            }
            let at = self.view_point(x, 0.0).x;
            // A number cut by the edge reads as another number, -10 as 0.
            if at < LABEL_WIDTH / 2.0 || at > width - LABEL_WIDTH / 2.0 {
                continue;
            }
            self.add_label(
                &label(x, step),
                at - LABEL_WIDTH / 2.0,
                below_axis,
                TextAlignment::Center,
                color,
            );
        }

        let right_of_axis = (origin.x + LABEL_GAP).clamp(0.0, width - LABEL_WIDTH);
        for y in lines(bottom, top, step) {
            if y == 0.0 {
                continue;
            }
            let at = self.view_point(0.0, y).y;
            if at < LABEL_HEIGHT / 2.0 || at > height - LABEL_HEIGHT / 2.0 {
                continue;
            }
            self.add_label(
                &label(y, step),
                right_of_axis,
                at - LABEL_HEIGHT / 2.0,
                TextAlignment::Left,
                color,
            );
        }
    }

    fn add_label(&mut self, text: &str, left: f32, top: f32, alignment: TextAlignment, color: UIColor) {
        let label = self.add_view::<Label>();
        label
            .set_text(text)
            .set_text_size(13)
            .set_text_color(color)
            .set_alignment(alignment)
            .set_text_inset(0);
        label.place().l(left).t(top).size(LABEL_WIDTH, LABEL_HEIGHT);
        self.labels.push(label);
    }

    fn draw_curve(&mut self, index: usize) {
        let height = f64::from(self.height());
        let limit = height * BREAK_HEIGHTS;
        let mut pieces: Vec<Vec<Point>> = vec![Vec::new()];

        let points: Vec<(f64, f64)> = match &self.curves[index].source {
            Source::Function(function) => {
                let (left, right) = self.x_range();
                let step = SAMPLE_STEP * self.units_per_point();
                let mut samples = Vec::new();
                let mut index = 0.0;
                loop {
                    let x = left + index * step;
                    if x > right + step {
                        break samples;
                    }
                    samples.push((x, function(x)));
                    index += 1.0;
                }
            }
            Source::Points(points) => points.clone(),
        };

        let units = self.units_per_point();
        let (_, top) = self.y_range();
        for (x, y) in points {
            // Checked in f64 before the f32 point exists, a value like
            // x^100 is far past what an f32 holds and its convert panics.
            let view_y = (top - y) / units;
            if !view_y.is_finite() || view_y < -limit || view_y > height + limit {
                pieces.push(Vec::new());
                continue;
            }
            if let Some(piece) = pieces.last_mut() {
                piece.push(self.view_point(x, y));
            }
        }

        let color = self.curves[index].color.resolve();
        let style = StrokeStyle::width(self.line_width).cap(LineCap::Round).join(LineJoin::Round);
        for piece in pieces.into_iter().filter(|piece| piece.len() > 1) {
            self.drawing.add_stroke(&VectorPath::polyline(piece), color, style);
        }
    }

    fn drag(&mut self, touch: Touch) {
        if touch.is_ended() {
            self.drag_from = None;
            return;
        }
        if let Some(from) = self.drag_from
            && touch.is_moved()
        {
            let units = self.units_per_point();
            self.center_x -= f64::from(touch.position.x - from.x) * units;
            self.center_y += f64::from(touch.position.y - from.y) * units;
            self.changed();
        }
        self.drag_from = Some(touch.position);
    }

    fn pinch(&mut self, pinch: Pinch) {
        let units = self.units_per_point();
        self.center_x -= f64::from(pinch.shift.x) * units;
        self.center_y += f64::from(pinch.shift.y) * units;
        self.zoom_at(pinch.center, f64::from(pinch.scale));
    }
}

impl Setup for PlotView {
    fn clips_to_bounds(&self) -> bool {
        true
    }

    fn setup(mut self: Weak<Self>) {
        self.x_span = 20.0;
        self.line_width = 3.0;
        self.grid_color = GRID.into();
        self.axis_color = AXIS.into();

        self.field_view.place().back();
        self.field_view.set_hidden(true);
        self.drawing.place().back();

        self.enable_input();
        self.touch().all.val(move |touch| self.drag(touch));
        self.touch().pinch.val(self, move |pinch| self.pinch(pinch));

        self.size_changed().sub(move || self.redraw());
        UIEvents::theme_changed().sub(self, move || self.redraw());
    }
}

impl Scrollable for PlotView {
    fn __process_scroll_touch(&mut self, _touch: Touch) -> bool {
        false
    }

    fn __moves_sideways(&self) -> bool {
        true
    }

    fn __process_wheel_scroll(&mut self, delta: Point) {
        let cursor = self.__base_view().local_point(UIManager::cursor_position());
        self.zoom_at(cursor, (f64::from(delta.y) * WHEEL_ZOOM).exp());
    }
}
