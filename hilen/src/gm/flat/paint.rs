use crate::gm::{color::Color, flat::Point};

/// How many stops a paint can carry, the size of the uniform arrays in
/// `PathView` and `ui_path.wgsl`.
pub const MAX_STOPS: usize = 8;

/// The geometry of a paint's ramp. Points are in the owning view's
/// coordinate space, like the path points themselves.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ramp {
    Flat,
    /// 0 at `from`, 1 at `to`, clamped past both ends, CSS
    /// `linear-gradient`.
    Linear {
        from: Point,
        to:   Point,
    },
    /// 0 at `at`, 1 at `radius` away, clamped past the edge, the CSS
    /// `radial-gradient` circle shape.
    Radial {
        at:     Point,
        radius: f32,
    },
    /// Follows the angle around `at`, running 0 to 1 and back
    /// `repeats` times per turn, seamless at any count. This is the
    /// anisotropic sheen of turned metal, which no linear or radial
    /// ramp can fake.
    Conic {
        at:      Point,
        repeats: f32,
    },
}

/// What a `DrawingView` path is filled or stroked with: a ramp, up to
/// [`MAX_STOPS`] color stops along it, and an optional grain. A plain
/// `Color` converts into a flat paint, so simple paths keep passing a
/// color. Stops interpolate premultiplied, like CSS, so a ramp into a
/// transparent stop fades without sliding through black, which is also
/// what makes soft shadows and glows: a radial ramp from a color to
/// the same color with alpha zero.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Paint {
    pub(crate) ramp:  Ramp,
    pub(crate) stops: [(Color, f32); MAX_STOPS],
    pub(crate) count: usize,
    pub(crate) grain: f32,
}

impl Paint {
    fn new(ramp: Ramp, start: Color, end: Color) -> Self {
        let mut stops = [(Color::default(), 0.0); MAX_STOPS];
        stops[0] = (start, 0.0);
        stops[1] = (end, 1.0);
        Self {
            ramp,
            stops,
            count: 2,
            grain: 0.0,
        }
    }

    pub fn flat(color: Color) -> Self {
        let mut paint = Self::new(Ramp::Flat, color, color);
        paint.count = 1;
        paint
    }

    pub fn linear(from: impl Into<Point>, to: impl Into<Point>, start: Color, end: Color) -> Self {
        Self::new(
            Ramp::Linear {
                from: from.into(),
                to:   to.into(),
            },
            start,
            end,
        )
    }

    pub fn radial(at: impl Into<Point>, radius: f32, center: Color, edge: Color) -> Self {
        Self::new(
            Ramp::Radial {
                at: at.into(),
                radius,
            },
            center,
            edge,
        )
    }

    pub fn conic(at: impl Into<Point>, repeats: f32, start: Color, end: Color) -> Self {
        Self::new(
            Ramp::Conic {
                at: at.into(),
                repeats,
            },
            start,
            end,
        )
    }

    /// Inserts a stop at `position` along the ramp, keeping the stops
    /// sorted. The two constructor colors sit at 0 and 1.
    ///
    /// # Panics
    ///
    /// A paint holds at most [`MAX_STOPS`] stops.
    #[must_use]
    pub fn stop(mut self, color: Color, position: f32) -> Self {
        assert!(self.count < MAX_STOPS, "a paint holds at most {MAX_STOPS} stops");
        let at = self.stops[..self.count].partition_point(|(_, existing)| *existing <= position);
        self.stops[at..=self.count].rotate_right(1);
        self.stops[at] = (color, position);
        self.count += 1;
        self
    }

    /// Per pixel luminance noise scaled by `amount`, the brushed metal
    /// and plastic texture. On a conic ramp the noise follows the
    /// angle, so it streaks along the radius like machined metal.
    #[must_use]
    pub fn grain(mut self, amount: f32) -> Self {
        self.grain = amount;
        self
    }

    /// A paint whose every stop is fully transparent draws nothing and
    /// is skipped like a fully transparent flat path.
    pub(crate) fn visible(&self) -> bool {
        self.stops[..self.count].iter().any(|(color, _)| color.a >= 0.004)
    }
}

impl From<Color> for Paint {
    fn from(color: Color) -> Self {
        Self::flat(color)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::gm::color::{BLACK, BLUE, GREEN, RED, WHITE};

    #[test]
    fn stops_stay_sorted() {
        let paint = Paint::linear((0, 0), (1, 0), BLACK, WHITE)
            .stop(RED, 0.5)
            .stop(GREEN, 0.25)
            .stop(BLUE, 0.75);
        let positions: Vec<f32> = paint.stops[..paint.count].iter().map(|(_, p)| *p).collect();
        assert_eq!(positions, [0.0, 0.25, 0.5, 0.75, 1.0]);
        assert_eq!(paint.stops[1].0, GREEN);
        assert_eq!(paint.stops[2].0, RED);
        assert_eq!(paint.stops[3].0, BLUE);
    }

    #[test]
    fn flat_is_one_stop() {
        let paint = Paint::flat(RED);
        assert_eq!(paint.count, 1);
        assert!(paint.visible());
        assert!(!Paint::flat(RED.with_alpha(0.0)).visible());
    }
}
