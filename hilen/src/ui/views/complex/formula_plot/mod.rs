//! Formulas typed as text, drawn as graphs: `y = sin(x)` a curve,
//! `z = x*y` a surface and `w = 1/z` a complex function as a surface of
//! its size colored by its angle. `graphs` reads a block of lines into
//! graphs, `GraphView` draws 1. A surface needs the `scene` feature, see
//! docs/formula-plot.md.

mod complex_math;
pub mod formula;
mod graph;
mod graph_view;
#[cfg(feature = "ui-tests")]
mod look_test;

pub use self::{
    graph::{Graph, graphs},
    graph_view::GraphView,
};
