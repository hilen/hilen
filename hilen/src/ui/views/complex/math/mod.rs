//! LaTeX math: a parser, a layout on the MATH table of Latin Modern Math,
//! and `MathView` that draws the result.

mod layout;
mod parse;
mod symbols;
mod view;

pub(crate) use layout::{MathBox, lay_out};
pub(crate) use parse::parse;
pub use view::MathView;
