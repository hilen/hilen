//! The 3D side of `GraphView`: a live surface in a scene, and a still
//! picture of it for a list or a chat. Only with the `scene` feature.

use std::sync::atomic::{AtomicU64, Ordering};

use num_complex::Complex64;

use super::{
    super::{
        complex_math::domain_color,
        formula::{Expression, Kind},
        graph::Graph,
    },
    GraphView, value,
};
use crate::{
    gm::LossyConvert,
    refs::{Weak, manage::DataManager},
    scene::SurfacePlotView,
    ui::{Color, ViewData, ViewFrame, ViewSubviews},
};

/// A still picture has this many pixels for each point, sharp on a screen
/// with 2 pixels per point.
const STILL_SCALE: f32 = 2.0;
/// The largest side a GPU draws.
const MAX_PIXELS: f32 = 16384.0;

/// Each still picture needs a name of its own, the store returns the old
/// picture for a name it already holds.
static STILLS: AtomicU64 = AtomicU64::new(0);

impl GraphView {
    /// A new view every time: one made before may have lost its scene to
    /// another view that started the 1 scene of the app.
    pub(super) fn show_surface(mut self: Weak<Self>, graph: &Graph) {
        if self.surface.is_ok() {
            self.surface.remove_from_superview();
        }
        self.surface = self.add_view::<SurfacePlotView>();
        self.surface.place().back();
        let mut surface = self.surface;
        surface.set_range(graph.half());
        let Some(formula) = graph.formulas().first().cloned() else {
            return;
        };
        if graph.kind() == Kind::Complex {
            surface.set_colored_function(move |x, y| complex_point(&formula, x, y));
        } else {
            surface.set_function(move |x, y| value(&formula, x, y));
        }
    }

    /// Draws the still picture again whatever size it had, for a new
    /// graph or a new theme.
    pub(super) fn redraw_still(mut self: Weak<Self>) {
        self.still_size = (0, 0);
        self.draw_still();
    }

    /// The picture of a still 3D graph, drawn again when the size or the
    /// theme changes.
    pub(super) fn draw_still(mut self: Weak<Self>) {
        let Some(graph) = &self.graph else {
            return;
        };
        if self.live || graph.kind() == Kind::Curve {
            return;
        }
        let Some(formula) = graph.formulas().first().cloned() else {
            return;
        };
        let size = (pixels(self.width()), pixels(self.height()));
        if size.0 == 0 || size.1 == 0 || size == self.still_size {
            return;
        }
        let complex = graph.kind() == Kind::Complex;
        let half = graph.half();
        let name = format!("formula-plot-still-{}", STILLS.fetch_add(1, Ordering::Relaxed));
        let still = SurfacePlotView::picture(
            move |x, y| {
                if complex {
                    let (height, color) = complex_point(&formula, x, y);
                    (height, Some(color))
                } else {
                    (value(&formula, x, y), None)
                }
            },
            half,
            &name,
            size,
        );
        if self.still.is_ok() {
            self.still.free();
        }
        self.still = still;
        self.still_size = size;
        self.picture.set_image(still);
        self.picture.set_hidden(false);
    }
}

fn pixels(points: f32) -> u32 {
    let pixels = (points * STILL_SCALE).round();
    if (1.0..MAX_PIXELS).contains(&pixels) {
        pixels.lossy_convert()
    } else {
        0
    }
}

/// The modulus surface of a complex function: the height is the size of
/// its value, the color its angle.
fn complex_point(formula: &Expression, x: f64, y: f64) -> (f64, Color) {
    let value = formula
        .eval_complex(Complex64::new(x, y))
        .unwrap_or(Complex64::new(f64::NAN, f64::NAN));
    (value.norm(), domain_color(value))
}
