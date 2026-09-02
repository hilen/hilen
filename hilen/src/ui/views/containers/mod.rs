mod scrolling;
mod table_view;

pub use scrolling::*;
pub use table_view::*;
mod movable_view;
mod split_view;
/// Hover needs a pointer, and there is no such thing on a touch screen.
#[cfg(all(feature = "ui-tests", any(desktop, wasm)))]
mod split_view_hover_test;
#[cfg(feature = "ui-tests")]
mod split_view_test;
mod view_gallery;

pub use movable_view::*;
pub use split_view::*;
pub use view_gallery::*;
