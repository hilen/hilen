mod box_chars;
#[cfg(feature = "gpu")]
mod brushes;
mod color_check;
#[cfg(feature = "gpu")]
mod color_glyph;
mod fallback;
mod font;
mod measure_cache;
mod shape_cache;
mod shaped_layout;
mod system_emoji;
#[cfg(not_wasm)]
mod system_fallback;
mod text_layout;

#[cfg(feature = "gpu")]
pub(crate) use box_chars::{BOX_LAYERS, is_box_char};
pub(crate) use box_chars::{BoxChars, BoxFrame, box_layer, fit_box};
#[cfg(feature = "gpu")]
pub(crate) use brushes::TextBrushes;
pub(crate) use fallback::runs_with_fallbacks;
pub use font::*;
pub(crate) use measure_cache::*;
pub(crate) use shape_cache::*;
pub(crate) use shaped_layout::*;
pub use text_layout::TextLayout;
pub(crate) use text_layout::TextLine;
