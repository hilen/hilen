//! The diff of 1 file, split in 2 columns or unified in 1, with syntax
//! colors, a text selection, the 2 sides of an image and buttons that
//! discard a hunk or a line. The app it lives in loads the diffs and runs
//! the discards through `DiffHost`. The data is the `filediff` crate, see
//! docs/diff-view.md.

pub mod diff_colors;
mod highlight;
mod icons;
mod image_diff;
mod panel;

pub use panel::{DiffHost, DiffPanel, DiffShown};
