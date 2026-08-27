use std::f32::consts::TAU;

use ui_proc::view;
use web_time::Instant;

use crate::{
    deps::refs::Weak,
    gm::{
        ToF32,
        color::{Color, GRAY},
        flat::{LineCap, StrokeStyle, VectorPath},
    },
    ui::{DrawingView, Setup, UIAnimation, UIColor, ViewCallbacks, ViewData, ViewFrame},
};

/// An indeterminate progress ring. A circle stroke with a quarter gap that
/// turns while the view is visible, the browser `animate-spin` border ring.
/// The ring fills the view's frame, so size it like any view.
#[view]
pub struct RingSpinner {
    ring_color:    UIColor,
    color:         Color,
    line_width:    f32,
    /// Turns per second.
    speed:         f32,
    /// Position of the gap, in turns.
    angle:         f32,
    last_tick:     Option<Instant>,
    keeping_alive: bool,

    #[init]
    drawing: DrawingView,
}

impl RingSpinner {
    /// Not `set_color`, which is the `ViewData` background setter.
    pub fn set_ring_color(&mut self, color: impl Into<UIColor>) -> &mut Self {
        let color = color.into();
        self.ring_color = color;
        self.color = color.resolve();
        self.redraw();
        self
    }

    pub fn set_line_width(&mut self, width: impl ToF32) -> &mut Self {
        self.line_width = width.to_f32();
        self.redraw();
        self
    }

    /// Turns per second. Zero holds the ring still at its current angle.
    pub fn set_speed(&mut self, turns_per_second: impl ToF32) -> &mut Self {
        self.speed = turns_per_second.to_f32();
        self
    }

    /// Where the gap sits, in turns clockwise from the top.
    pub fn set_angle(&mut self, turns: impl ToF32) -> &mut Self {
        self.angle = turns.to_f32().rem_euclid(1.0);
        self.redraw();
        self
    }

    pub fn angle(&self) -> f32 {
        self.angle
    }

    fn redraw(&mut self) {
        self.drawing.remove_all_paths();
        let size = self.size();
        let radius = (size.smallest_side() - self.line_width) / 2.0;
        if radius <= 0.0 {
            return;
        }
        // The gap is the top quarter at angle zero, screen angles start
        // at the right, so the stroke starts a quarter turn before the top.
        let start = (self.angle - 0.125) * TAU;
        let path = VectorPath::arc(size.center(), radius, start, TAU * 0.75);
        self.drawing.add_stroke(
            &path,
            self.color,
            StrokeStyle::width(self.line_width).cap(LineCap::Butt),
        );
    }

    /// Render on demand sleeps the loop unless continuous work is live. A
    /// live animation is that work, so an empty one runs while the ring is
    /// visible and ends on its own when it hides or dies.
    fn keep_frames_coming(mut self: Weak<Self>) {
        if self.keeping_alive {
            return;
        }
        self.keeping_alive = true;
        let anim =
            UIAnimation::new(|_, _| {}).finish_condition(move || self.is_null() || self.is_hidden_in_tree());
        anim.on_finish.sub(move || {
            if self.is_ok() {
                self.keeping_alive = false;
                self.last_tick = None;
            }
        });
        self.add_animation(anim);
    }
}

impl Setup for RingSpinner {
    fn setup(mut self: Weak<Self>) {
        self.set_ring_color(GRAY);
        self.line_width = 2.0;
        self.speed = 1.0;
        self.set_size(12, 12);
        self.drawing.place().back();
    }
}

impl ViewCallbacks for RingSpinner {
    fn update(&mut self) {
        if self.is_hidden_in_tree() {
            return;
        }
        self.weak().keep_frames_coming();

        let now = Instant::now();
        let passed = self.last_tick.map_or(0.0, |tick| (now - tick).as_secs_f32());
        self.last_tick = Some(now);
        let angle = (self.speed * passed + self.angle).rem_euclid(1.0);
        if angle.to_bits() != self.angle.to_bits() || self.drawing.paths().is_empty() {
            self.angle = angle;
            self.redraw();
        }
    }

    fn theme_changed(&mut self) {
        self.color = self.ring_color.resolve();
        self.redraw();
    }
}
