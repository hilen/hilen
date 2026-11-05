/// The 3D side, it reads the fields of `GraphView`.
#[cfg(feature = "scene")]
#[path = "graph_view_3d.rs"]
mod graph_view_3d;

use super::{
    formula::{Expression, Kind},
    graph::Graph,
};
use crate::{
    refs::Weak,
    ui::{Color, DynamicColor, ImageView, PlotView, Setup, ViewData, view},
};
#[cfg(feature = "scene")]
use crate::{
    scene::SurfacePlotView,
    ui::{Image, UIEvents, ViewSubviews},
};

/// The colors of the curves of 1 graph, in the order of their formulas.
const CURVES: [DynamicColor; 5] = [
    DynamicColor::new(Color::hex("#C1121F"), Color::hex("#FF6B6B")),
    DynamicColor::new(Color::hex("#1F5FBF"), Color::hex("#6EA8FE")),
    DynamicColor::new(Color::hex("#2D8A3E"), Color::hex("#63C174")),
    DynamicColor::new(Color::hex("#C2610C"), Color::hex("#F5A65B")),
    DynamicColor::new(Color::hex("#7B3FA0"), Color::hex("#C49BE6")),
];

/// Draws 1 graph. Live, a drag pans or turns it and the wheel and a pinch
/// zoom it, and a 3D graph runs the 1 scene of the app. Still, for a list
/// or a chat, it takes no input, so the wheel and a tap go to the views
/// around it, and a 3D graph is a picture.
#[view]
pub struct GraphView {
    live:       bool,
    graph:      Option<Graph>,
    /// Made only for a live 3D graph, each one starts the scene.
    #[cfg(feature = "scene")]
    surface:    Weak<SurfacePlotView>,
    #[cfg(feature = "scene")]
    still:      Weak<Image>,
    /// The pixels the still picture has, to draw it again on a new size.
    #[cfg(feature = "scene")]
    still_size: (u32, u32),

    #[init]
    plot:    PlotView,
    picture: ImageView,
}

impl Setup for GraphView {
    fn clips_to_bounds(&self) -> bool {
        true
    }

    fn setup(self: Weak<Self>) {
        self.plot.place().back();
        self.plot.set_hidden(true);
        self.picture.place().back();
        self.picture.set_hidden(true);
        #[cfg(feature = "scene")]
        {
            self.size_changed().sub(move || self.draw_still());
            UIEvents::theme_changed().sub(self, move || self.redraw_still());
        }
    }
}

impl GraphView {
    /// Shows `graph`, live or still, in place of the one before.
    pub fn set_graph(mut self: Weak<Self>, graph: Graph, live: bool) {
        log::debug!("graph of {:?}, live {live}", graph.texts());
        self.live = live;
        self.plot.set_interactive(live);
        self.plot.remove_curves();
        self.picture.set_hidden(true);
        let kind = graph.kind();
        self.plot.set_hidden(kind != Kind::Curve);
        #[cfg(feature = "scene")]
        if kind != Kind::Curve && live {
            self.show_surface(&graph);
        } else if self.surface.is_ok() {
            self.surface.remove_from_superview();
            self.surface = Weak::default();
        }
        if kind == Kind::Curve {
            let (left, right) = graph.x_range();
            self.plot.set_x_range(left, right, 0.0);
            for (formula, color) in graph.formulas().iter().zip(CURVES.iter().cycle()) {
                let formula = formula.clone();
                self.plot.add_function(move |x| value(&formula, x, 0.0), *color);
            }
        }
        self.graph = Some(graph);
        #[cfg(feature = "scene")]
        self.redraw_still();
    }
}

/// A value the formula cannot compute leaves a gap in a curve or a hole
/// in a surface.
pub(super) fn value(formula: &Expression, x: f64, y: f64) -> f64 {
    formula.eval(x, y).unwrap_or(f64::NAN)
}
