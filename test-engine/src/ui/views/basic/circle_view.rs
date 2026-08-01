use refs::Weak;
use ui_proc::view;

use crate::{
    gm::{
        ToF32,
        color::Color,
        flat::{FillRule, VectorPath},
    },
    ui::{
        DrawingView, Setup,
        view::{ViewData, ViewFrame},
    },
};

#[view]
pub struct CircleView {
    radius: f32,
    color:  Color,

    #[init]
    drawing: DrawingView,
}

impl CircleView {
    pub fn set_radius(&mut self, radius: impl ToF32) -> &mut Self {
        let radius = radius.to_f32();

        if (radius - self.radius).abs() < f32::EPSILON {
            return self;
        }

        self.radius = radius;

        let diameter = radius.to_f32() * 2.0;
        self.set_size(diameter, diameter);
        self.redraw();
        self
    }

    /// Not `set_color`, which would collide with the `ViewData` method
    /// for the background. A `&self` trait method wins resolution over a
    /// `&mut self` inherent one, so a same named method here would be
    /// silently unreachable.
    pub fn set_circle_color(&mut self, color: Color) {
        self.color = color;
        self.redraw();
    }

    fn redraw(&mut self) {
        self.drawing.remove_all_paths();
        let frame = self.frame().with_zero_origin();
        self.drawing.add_fill(
            &VectorPath::circle(frame.size.center(), frame.size.width / 2.0),
            self.color,
            FillRule::NonZero,
        );
    }
}

impl Setup for CircleView {
    fn setup(self: Weak<Self>) {
        self.set_size(10, 10);
        self.drawing.place().back();
    }
}
