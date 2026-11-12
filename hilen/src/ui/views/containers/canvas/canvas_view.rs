use crate::{
    self as hilen,
    deps::{
        refs::{Weak, weak_from_ref},
        vents::Event,
    },
    gm::{
        ToF32,
        flat::{Point, Rect},
    },
    ui::{Pinch, Scrollable, Setup, Touch, TouchStack, UIManager, View, ViewTouch, view},
};

/// One wheel notch is 60 pixels, that zooms by about an eighth.
const WHEEL_ZOOM: f32 = 0.002;

const MIN_ZOOM: f32 = 0.1;
const MAX_ZOOM: f32 = 10.0;

/// A view that pans and zooms what is inside it, for a map, a graph or a
/// board. Its subviews are laid out as in any view, in its own points at
/// zoom 1, and it draws them all bigger or smaller and moved. Text and
/// paths are drawn at the new size, not stretched, and a subview gets its
/// touches in its own points.
///
/// A drag on its empty area pans, the wheel zooms at the cursor and a
/// pinch zooms at the fingers. The view clips its subviews to its frame.
#[view]
pub struct CanvasView {
    zoom:      f32,
    min_zoom:  f32,
    max_zoom:  f32,
    offset:    Point,
    /// The part of the subviews that has to stay in view, see `set_bounds`.
    bounds:    Option<Rect>,
    /// Where the finger of a drag was last, in the points of this view.
    drag_from: Option<Point>,

    /// The zoom or the offset changed.
    pub on_change: Event,
}

impl CanvasView {
    /// How much bigger the subviews are drawn, 1 at the start.
    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    /// Where the zero point of the subviews sits inside this view.
    pub fn offset(&self) -> Point {
        self.offset
    }

    /// Sets the zoom and keeps the middle of the view where it is.
    pub fn set_zoom(&self, zoom: impl ToF32) -> &Self {
        let this = weak_from_ref(self);
        let middle = Point::new(
            this.__base_view().frame.size.width,
            this.__base_view().frame.size.height,
        ) / 2.0;
        let factor = zoom.to_f32() / this.zoom;
        this.zoom_at(middle, factor);
        self
    }

    pub fn set_offset(&self, offset: impl Into<Point>) -> &Self {
        let mut this = weak_from_ref(self);
        this.offset = offset.into();
        this.apply();
        self
    }

    /// Keeps `bounds`, a rect in the points of the subviews, on the
    /// screen, like a picture that must not be panned away. Across a side
    /// where it is drawn smaller than the view it stays in the middle,
    /// else its edges never come inside the view. None lets it go
    /// anywhere, the default.
    pub fn set_bounds(&self, bounds: Option<Rect>) -> &Self {
        let mut this = weak_from_ref(self);
        this.bounds = bounds;
        this.apply();
        self
    }

    /// The smallest and the biggest zoom, 0.1 and 10 at the start.
    pub fn set_zoom_limits(&self, min: impl ToF32, max: impl ToF32) -> &Self {
        let mut this = weak_from_ref(self);
        this.min_zoom = min.to_f32();
        this.max_zoom = max.to_f32();
        this.zoom_at(Point::default(), 1.0);
        self
    }

    /// Zooms by `factor` and keeps what is under `point` where it is.
    /// `point` is in the points of this view.
    pub fn zoom_at(&self, point: impl Into<Point>, factor: impl ToF32) -> &Self {
        let mut this = weak_from_ref(self);
        let point = point.into();
        let zoom = (this.zoom * factor.to_f32()).clamp(this.min_zoom, this.max_zoom);
        this.offset = point - (point - this.offset) * (zoom / this.zoom);
        this.zoom = zoom;
        this.apply();
        self
    }

    /// The point of the subviews that is drawn at `point` of this view.
    pub fn content_point(&self, point: impl Into<Point>) -> Point {
        (point.into() - self.offset) / self.zoom
    }

    fn apply(&mut self) {
        self.keep_bounds();
        let base = self.__base_view();
        base.content_scale = self.zoom;
        base.content_shift = self.offset;
        self.on_change.trigger(());
    }

    fn keep_bounds(&mut self) {
        let Some(bounds) = self.bounds else {
            return;
        };
        let size = self.__base_view().frame.size;
        // Not laid out yet, a later change keeps the bounds.
        if size.width <= 0.0 || size.height <= 0.0 {
            return;
        }
        let zoom = self.zoom;
        let axis = |offset: f32, start: f32, length: f32, room: f32| {
            let drawn = length * zoom;
            if drawn <= room {
                (room - drawn) / 2.0 - start * zoom
            } else {
                offset.clamp(room - (start + length) * zoom, -start * zoom)
            }
        };
        self.offset = Point::new(
            axis(self.offset.x, bounds.origin.x, bounds.size.width, size.width),
            axis(self.offset.y, bounds.origin.y, bounds.size.height, size.height),
        );
    }

    fn drag(&mut self, touch: Touch) {
        if touch.is_ended() {
            self.drag_from = None;
            return;
        }
        if let Some(from) = self.drag_from
            && touch.is_moved()
        {
            self.offset += touch.position - from;
            self.apply();
        }
        self.drag_from = Some(touch.position);
    }

    fn pinch(&mut self, pinch: Pinch) {
        self.offset += pinch.shift;
        self.zoom_at(pinch.center, pinch.scale);
    }
}

impl Setup for CanvasView {
    fn clips_to_bounds(&self) -> bool {
        true
    }

    fn setup(mut self: Weak<Self>) {
        // A control placed over the canvas covers what the canvas shows.
        self.__base_view().flat_depth = true;

        self.zoom = 1.0;
        self.min_zoom = MIN_ZOOM;
        self.max_zoom = MAX_ZOOM;

        // Low priority, a subview that takes touches is asked first
        // whenever it was added.
        self.enable_touch_low_priority();
        self.touch().all.val(move |touch| self.drag(touch));

        self.enable_pinch();
        self.touch().pinch.val(self, move |pinch| self.pinch(pinch));

        TouchStack::enable_scroll(self);
    }
}

impl Scrollable for CanvasView {
    fn __process_scroll_touch(&mut self, _touch: Touch) -> bool {
        false
    }

    fn __moves_sideways(&self) -> bool {
        true
    }

    fn __process_wheel_scroll(&mut self, delta: Point) {
        let cursor = self.__base_view().local_point(UIManager::cursor_position());
        self.zoom_at(cursor, (delta.y * WHEEL_ZOOM).exp());
    }
}
