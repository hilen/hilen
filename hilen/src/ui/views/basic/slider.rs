use ui_proc::view;

use crate::{
    deps::{
        refs::{Weak, weak_from_ref},
        vents::Event,
    },
    gm::{ToF32, color::WHITE, converter::Converter},
    ui::{
        CircleView, Color, Container, Point, Setup, Shadow, Touch, UIColor, ViewCallbacks, ViewData,
        view::{ViewFrame, ViewTouch},
    },
};

const TRACK_THICKNESS: f32 = 8.0;
const THUMB_RADIUS: f32 = 14.0;
const TRACK_COLOR: Color = Color::hex("#d1d1d6");
const FILL_COLOR: Color = Color::hex("#0a84ff");
const THUMB_SHADOW: Shadow = Shadow {
    offset: Point { x: 0.0, y: 3.0 },
    radius: 8.0,
    color:  Color::rgba(0.0, 0.0, 0.0, 0.35),
};

/// A thin rounded track with the part up to the thumb filled in an
/// accent color and a round thumb over it. Vertical by default, the
/// bottom is the minimum. `set_horizontal` lays it on its side with
/// the minimum at the left. The whole view takes touches, so a thin
/// track stays easy to hit.
#[view]
pub struct Slider {
    raw_value:  f32,
    horizontal: bool,

    track_thickness: f32,
    thumb_radius:    f32,

    converter: Converter,

    /// A touch holds the thumb now.
    dragging: bool,

    pub on_change:  Event<f32>,
    /// A touch took the thumb, before the first `on_change` of the drag.
    pub drag_began: Event,
    /// The touch that held the thumb let go, after the last `on_change` of
    /// the drag. An owner that keeps 1 undo step per drag closes it here.
    pub drag_ended: Event,

    #[init]
    track:  Container,
    fill:   Container,
    circle: CircleView,
}

impl Slider {
    /// A touch holds the thumb now, between `drag_began` and `drag_ended`.
    pub fn is_dragging(&self) -> bool {
        self.dragging
    }

    pub fn value(&self) -> f32 {
        self.converter.convert(self.raw_value)
    }

    /// Moves the thumb. `on_change` fires only for the user, never for this
    /// call, so code that shows a value does not run the handler of a drag.
    pub fn set_value(&self, val: impl ToF32) -> &Self {
        let mut this = weak_from_ref(self);
        this.raw_value = this.converter.reverse_convert(val);
        this.layout_parts();
        self
    }

    pub fn set_horizontal(&self) -> &Self {
        let mut this = weak_from_ref(self);
        this.horizontal = true;
        this.layout_parts();
        self
    }

    pub fn set_track_thickness(&self, thickness: impl ToF32) -> &Self {
        let mut this = weak_from_ref(self);
        this.track_thickness = thickness.to_f32();
        this.track.set_corner_radius(this.track_thickness / 2.0);
        this.fill.set_corner_radius(this.track_thickness / 2.0);
        this.layout_parts();
        self
    }

    pub fn set_thumb_radius(&self, radius: impl ToF32) -> &Self {
        let mut this = weak_from_ref(self);
        this.thumb_radius = radius.to_f32();
        this.layout_parts();
        self
    }

    pub fn set_track_color(&self, color: impl Into<UIColor>) -> &Self {
        let this = weak_from_ref(self);
        this.track.set_color(color);
        self
    }

    pub fn set_fill_color(&self, color: impl Into<UIColor>) -> &Self {
        let this = weak_from_ref(self);
        this.fill.set_color(color);
        self
    }

    pub fn set_thumb_color(&self, color: Color) -> &Self {
        let this = weak_from_ref(self);
        this.circle.set_circle_color(color);
        self
    }

    pub fn set_thumb_shadow(&self, shadow: impl Into<Option<Shadow>>) -> &Self {
        let this = weak_from_ref(self);
        this.circle.set_shadow(shadow);
        self
    }

    pub fn set_thumb_border(&self, width: impl ToF32, color: impl Into<UIColor>) -> &Self {
        let this = weak_from_ref(self);
        this.circle.set_border_width(width).set_border_color(color);
        self
    }

    fn layout_parts(&mut self) {
        let r = self.thumb_radius;
        let t = self.track_thickness;
        let (width, height) = (self.width(), self.height());
        self.circle.set_radius(r);
        self.circle.set_corner_radius(r);
        if self.horizontal {
            let length = width - 2.0 * r;
            let y = (height - t) / 2.0;
            self.track.set_frame((r, y, length, t));
            self.fill.set_frame((r, y, length * self.raw_value, t));
            self.circle.set_x(length * self.raw_value);
            self.circle.set_y(height / 2.0 - r);
        } else {
            let length = height - 2.0 * r;
            let x = (width - t) / 2.0;
            let top = r + length * (1.0 - self.raw_value);
            self.track.set_frame((x, r, t, length));
            self.fill.set_frame((x, top, t, length * self.raw_value));
            self.circle.set_x(width / 2.0 - r);
            self.circle.set_y(top - r);
        }
    }

    /// The indicator center along the slider axis, y when vertical and
    /// x when horizontal.
    pub fn indicator_position(&self) -> f32 {
        let center = self.circle.frame().center();
        if self.horizontal { center.x } else { center.y }
    }

    pub fn set_range(&self, min: impl ToF32, max: impl ToF32) -> &Self {
        let this = weak_from_ref(self);
        this.set_min(min).set_max(max);
        self
    }

    pub fn set_min(&self, min: impl ToF32) -> &Self {
        let mut this = weak_from_ref(self);
        this.converter.set_min(min);
        self
    }

    pub(crate) fn set_max(&self, max: impl ToF32) -> &Self {
        let mut this = weak_from_ref(self);
        this.converter.set_max(max);
        self
    }

    fn value_changed(&self) {
        self.on_change.trigger(self.value());
    }
}

impl Setup for Slider {
    fn setup(mut self: Weak<Self>) {
        self.enable_touch();
        self.touch().all.val(move |touch| {
            self.on_touch(&touch);
        });

        self.track_thickness = TRACK_THICKNESS;
        self.thumb_radius = THUMB_RADIUS;
        self.track.set_color(TRACK_COLOR).set_corner_radius(TRACK_THICKNESS / 2.0);
        self.fill.set_color(FILL_COLOR).set_corner_radius(TRACK_THICKNESS / 2.0);
        self.circle.set_circle_color(WHITE);
        self.circle.set_shadow(THUMB_SHADOW);
    }
}

impl ViewCallbacks for Slider {
    fn update(&mut self) {
        self.layout_parts();
    }
}

impl Slider {
    fn on_touch(&mut self, touch: &Touch) {
        if touch.is_ended() {
            if self.dragging {
                self.dragging = false;
                self.drag_ended.trigger(());
            }
            return;
        }
        if !self.dragging {
            self.dragging = true;
            self.drag_began.trigger(());
        }

        let r = self.thumb_radius;
        if self.horizontal {
            let x_pos = touch.position.x.clamp(r, self.width() - r);
            self.raw_value = (x_pos - r) / (self.width() - 2.0 * r);
        } else {
            let y_pos = touch.position.y.clamp(r, self.height() - r);
            self.raw_value = 1.0 - (y_pos - r) / (self.height() - 2.0 * r);
        }
        self.layout_parts();

        self.value_changed();
    }
}
