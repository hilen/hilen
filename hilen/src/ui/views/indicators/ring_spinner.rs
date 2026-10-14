use std::f32::consts::TAU;

use ui_proc::view;

use crate::{
    deps::refs::Weak,
    gm::{
        ToF32,
        color::{Color, GRAY},
        flat::{LineCap, Size, StrokeStyle, VectorPath},
    },
    ui::{DrawingView, LoopAnimation, Setup, UIColor, ViewCallbacks, ViewData, ViewFrame},
};

/// An indeterminate progress ring. A circle stroke with a quarter gap that
/// turns, the browser `animate-spin` border ring. The ring fills the
/// view's frame, so size it like any view. The turn is a declared loop, see
/// `LoopAnimation`, so on Apple systems a layer of the system turns it and
/// the app draws no frame for it.
#[view]
pub struct RingSpinner {
    ring_color: UIColor,
    color:      Color,
    line_width: f32,
    /// Turns per second.
    speed:      f32,
    /// Position of the gap now, in turns.
    angle:      f32,
    /// Where the gap was when the turning started, in turns.
    start:      f32,
    /// The size the path was made for.
    drawn_size: Size,

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
        let angle = self.angle();
        self.speed = turns_per_second.to_f32();
        self.weak().turn(angle);
        self
    }

    /// Where the gap sits, in turns clockwise from the top. A turning ring
    /// turns on from there.
    pub fn set_angle(&mut self, turns: impl ToF32) -> &mut Self {
        self.angle = turns.to_f32().rem_euclid(1.0);
        self.redraw();
        self.weak().turn(self.angle);
        self
    }

    /// Where the gap is at the time of this frame, in turns. A ring on a
    /// layer of the system holds its path at the start of the turn, so a
    /// turning ring reads it from the loop.
    pub fn angle(&self) -> f32 {
        match self.loop_phase() {
            Some(phase) => (self.start + phase * self.speed.signum()).rem_euclid(1.0),
            None => self.angle,
        }
    }

    fn redraw(&mut self) {
        self.drawing.remove_all_paths();
        let size = self.size();
        self.drawn_size = size;
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

    /// A turning ring runs a loop of 1 turn from `start`, a ring with no
    /// speed runs none and holds the angle it has.
    fn turn(mut self: Weak<Self>, start: f32) {
        if self.speed == 0.0 {
            if self.angle.to_bits() != start.to_bits() {
                self.angle = start;
                self.redraw();
            }
            self.stop_loop();
            return;
        }
        self.start = start;
        let period = 1.0 / self.speed.abs();
        self.run_loop(LoopAnimation::new(period, move |phase| self.pose(phase)));
    }

    /// The ring at `phase` of a turn. The path is made again only when the
    /// angle or the size moved.
    fn pose(mut self: Weak<Self>, phase: f32) {
        let angle = (self.start + phase * self.speed.signum()).rem_euclid(1.0);
        if angle.to_bits() != self.angle.to_bits()
            || self.drawing.paths().is_empty()
            || self.size() != self.drawn_size
        {
            self.angle = angle;
            self.redraw();
        }
    }
}

impl Setup for RingSpinner {
    fn setup(mut self: Weak<Self>) {
        self.set_ring_color(GRAY);
        self.line_width = 2.0;
        self.set_size(12, 12);
        self.drawing.place().back();
        self.set_speed(1);
    }
}

impl ViewCallbacks for RingSpinner {
    fn update(&mut self) {
        if self.size() != self.drawn_size {
            self.redraw();
        }
    }

    fn theme_changed(&mut self) {
        self.color = self.ring_color.resolve();
        self.redraw();
    }
}
