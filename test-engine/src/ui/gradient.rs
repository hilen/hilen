use std::f32::consts::SQRT_2;

use crate::{
    gm::{
        ToF32,
        color::Color,
        flat::{CornerRadii, Point, Rect, Size},
    },
    render::data::UIGradientInstance,
};

/// A gradient fill for a view, following CSS. `Gradient::vertical` is the
/// plain top to bottom ramp `set_gradient` has always drawn, the same thing
/// CSS calls `linear-gradient(180deg, start, end)`.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Gradient {
    pub start: Color,
    pub end:   Color,
    pub kind:  GradientKind,

    /// Where the end color lands on the gradient ray, the position of the
    /// last CSS color stop. `transparent 60%` is 0.6, and everything past
    /// it keeps the end color.
    pub end_stop: f32,
}

/// The shape of a gradient, a CSS `linear-gradient` or `radial-gradient`.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum GradientKind {
    Linear {
        /// Degrees, the CSS angle. Zero points up, 90 points right, and
        /// the angle grows clockwise, so 180 is top to bottom.
        angle: f32,
    },
    Radial {
        /// A fraction of the box, so the middle is 0.5 and 0.5, and the
        /// CSS `at top` is 0.5 and 0.0.
        center: Point,
    },
}

impl Gradient {
    /// CSS `linear-gradient(180deg, start, end)`, top to bottom.
    pub fn vertical(start: impl Into<Color>, end: impl Into<Color>) -> Self {
        Self::linear(180, start, end)
    }

    /// CSS `linear-gradient(<angle>deg, start, end)`.
    pub fn linear(angle: impl ToF32, start: impl Into<Color>, end: impl Into<Color>) -> Self {
        Self {
            start:    start.into(),
            end:      end.into(),
            kind:     GradientKind::Linear {
                angle: angle.to_f32(),
            },
            end_stop: 1.0,
        }
    }

    /// CSS `radial-gradient(ellipse, start, end)`, centered on the box.
    pub fn radial(start: impl Into<Color>, end: impl Into<Color>) -> Self {
        Self::radial_at((0.5, 0.5), start, end)
    }

    /// CSS `radial-gradient(ellipse at <x> <y>, start, end)`. The ending
    /// shape is the CSS default, the ellipse through the farthest corner.
    pub fn radial_at(center: impl Into<Point>, start: impl Into<Color>, end: impl Into<Color>) -> Self {
        Self {
            start:    start.into(),
            end:      end.into(),
            kind:     GradientKind::Radial {
                center: center.into(),
            },
            end_stop: 1.0,
        }
    }

    /// Moves the last color stop. CSS `transparent 60%` is 0.6.
    pub fn with_end_stop(mut self, end_stop: impl ToF32) -> Self {
        self.end_stop = end_stop.to_f32();
        self
    }

    /// All of the CSS math happens here, so the fragment stage is left with
    /// one dot product or one length. The shader works in uv space, where
    /// the box spans -0.5 to 0.5 on both axes and y grows downwards.
    pub(crate) fn instance(
        &self,
        frame: Rect,
        corner_radii: CornerRadii,
        border_color: Color,
        border_width: f32,
        z_position: f32,
        scale: f32,
    ) -> UIGradientInstance {
        // A ramp of no length has nothing to interpolate over and would
        // divide by zero.
        let end_stop = self.end_stop.max(0.001);

        let mut instance = UIGradientInstance {
            position: frame.origin,
            size: frame.size,
            start_color: self.start,
            end_color: self.end,
            corner_radii,
            z_position,
            scale,
            linear_axis: Point::default(),
            radial_center: Point::default(),
            radial_radii: Size::default(),
            linear_bias: 0.0,
            kind: UIGradientInstance::LINEAR,
            border_width,
            padding: [0.0; 1],
            border_color,
        };

        match self.kind {
            GradientKind::Linear { angle } => {
                let (sin, cos) = angle.to_radians().sin_cos();

                // The CSS gradient line length for the box.
                let length = (frame.width() * sin).abs() + (frame.height() * cos).abs();
                let ramp = length * end_stop;

                // The box size folds into the axis so the fragment can dot
                // it straight against uv. Cosine is negated because a CSS
                // angle grows clockwise from up while y grows downwards.
                instance.linear_axis = Point::new(frame.width() * sin / ramp, -frame.height() * cos / ramp);
                instance.linear_bias = 0.5 / end_stop;
            }
            GradientKind::Radial { center } => {
                // The farthest corner sits exactly at the farthest side
                // distance on both axes, and a farthest corner ellipse keeps
                // the farthest side aspect ratio, so both of its radii are
                // that distance times the square root of two.
                instance.kind = UIGradientInstance::RADIAL;
                instance.radial_center = Point::new(center.x - 0.5, center.y - 0.5);
                instance.radial_radii = Size::new(
                    SQRT_2 * center.x.max(1.0 - center.x) * end_stop,
                    SQRT_2 * center.y.max(1.0 - center.y) * end_stop,
                );
            }
        }

        instance
    }
}
